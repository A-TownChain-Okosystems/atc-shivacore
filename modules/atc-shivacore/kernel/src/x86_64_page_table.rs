// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-owned x86_64 process page-table lifecycle.
#![cfg(feature = "x86-boot")]
extern crate alloc;
use alloc::collections::{BTreeMap, BTreeSet};
use x86_64::{structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};
use crate::ats1000::Pid;
use crate::memory::BootInfoFrameAllocator;
use crate::memory_isolation::PageFlags;
use crate::x86_64_address_space::{self, AddressSpaceSwitchError, PageTableRoot};
use crate::x86_64_user_mapping::{map_user_page, validate_user_page, UserMappingError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableError { InvalidPid, OutOfFrames, InvalidRoot, Mapping(UserMappingError), NotMapped, ProcessExists, ProcessMissing, RootOwnershipConflict, OutstandingMappings }
impl From<UserMappingError> for PageTableError { fn from(value: UserMappingError) -> Self { Self::Mapping(value) } }

pub struct ProcessPageTable {
    pid: Pid,
    root_frame: PhysFrame,
    physical_memory_offset: VirtAddr,
    user_mappings: BTreeSet<u64>,
}
impl ProcessPageTable {
    pub unsafe fn new(pid: Pid, physical_memory_offset: VirtAddr, active_root: &PageTable, frame_allocator: &mut BootInfoFrameAllocator) -> Result<Self, PageTableError> {
        if pid.0 == 0 { return Err(PageTableError::InvalidPid); }
        let root_frame = frame_allocator.allocate_frame().ok_or(PageTableError::OutOfFrames)?;
        let root_virt = physical_memory_offset + root_frame.start_address().as_u64();
        let root_ptr = root_virt.as_mut_ptr::<PageTable>(); root_ptr.write(PageTable::new());
        let root = &mut *root_ptr; for index in 256..512 { root[index] = active_root[index].clone(); }
        Ok(Self { pid, root_frame, physical_memory_offset, user_mappings: BTreeSet::new() })
    }
    pub const fn pid(&self) -> Pid { self.pid }
    pub fn root_frame(&self) -> PhysFrame { self.root_frame }
    pub fn root_context(&self) -> Result<PageTableRoot, AddressSpaceSwitchError> { PageTableRoot::new(self.pid, self.root_frame.start_address().as_u64()) }
    pub fn root_physical_address(&self) -> PhysAddr { self.root_frame.start_address() }
    pub fn user_mapping_count(&self) -> usize { self.user_mappings.len() }
    pub fn has_user_mapping(&self, virtual_address: u64) -> bool { self.user_mappings.contains(&virtual_address) }
    pub unsafe fn mapper(&mut self) -> OffsetPageTable<'static> { let root_virt = self.physical_memory_offset + self.root_frame.start_address().as_u64(); let root = &mut *root_virt.as_mut_ptr::<PageTable>(); OffsetPageTable::new(root, self.physical_memory_offset) }
    pub unsafe fn map_user_page(&mut self, page: Page<Size4KiB>, frame: PhysFrame, flags: PageFlags, frame_allocator: &mut BootInfoFrameAllocator) -> Result<(), PageTableError> {
        validate_user_page(page, flags)?;
        let virtual_address = page.start_address().as_u64();
        if self.user_mappings.contains(&virtual_address) { return Err(PageTableError::NotMapped); }
        let mut mapper = self.mapper();
        map_user_page(&mut mapper, page, frame, flags, frame_allocator).map_err(Into::into)?;
        self.user_mappings.insert(virtual_address);
        Ok(())
    }
    pub unsafe fn unmap_user_page(&mut self, page: Page<Size4KiB>) -> Result<PhysFrame, PageTableError> {
        let virtual_address = page.start_address().as_u64();
        if !self.user_mappings.contains(&virtual_address) { return Err(PageTableError::NotMapped); }
        let mut mapper = self.mapper();
        let (frame, flush) = mapper.unmap(page).map_err(|_| PageTableError::NotMapped)?;
        flush.flush();
        self.user_mappings.remove(&virtual_address);
        Ok(frame)
    }
    pub fn validate_user_page(&self, page: Page<Size4KiB>, flags: PageFlags) -> Result<(), PageTableError> { validate_user_page(page, flags)?; Ok(()) }
}

/// Owns all live process roots. `root_owners` is an explicit ownership ledger:
/// a physical L4 frame may belong to exactly one live PID. Root frames are not
/// returned to the boot allocator because that allocator has no reclamation API.
pub struct ProcessAddressSpaceManager {
    spaces: BTreeMap<Pid, ProcessPageTable>,
    root_owners: BTreeMap<u64, Pid>,
    current: Option<Pid>,
}
impl ProcessAddressSpaceManager {
    pub const fn new() -> Self { Self { spaces: BTreeMap::new(), root_owners: BTreeMap::new(), current: None } }
    pub unsafe fn create(&mut self, pid: Pid, physical_memory_offset: VirtAddr, active_root: &PageTable, frame_allocator: &mut BootInfoFrameAllocator) -> Result<(), PageTableError> {
        if self.spaces.contains_key(&pid) { return Err(PageTableError::ProcessExists); }
        let table = ProcessPageTable::new(pid, physical_memory_offset, active_root, frame_allocator)?;
        let root = table.root_frame().start_address().as_u64();
        if self.root_owners.insert(root, pid).is_some() { return Err(PageTableError::RootOwnershipConflict); }
        self.spaces.insert(pid, table); Ok(())
    }
    pub fn root(&self, pid: Pid) -> Result<PageTableRoot, PageTableError> { self.spaces.get(&pid).ok_or(PageTableError::ProcessMissing)?.root_context().map_err(|_| PageTableError::InvalidRoot) }
    pub unsafe fn switch_to(&mut self, pid: Pid) -> Result<(), PageTableError> { let root = self.root(pid)?; x86_64_address_space::switch_to(root); self.current = Some(pid); Ok(()) }
    pub fn current_pid(&self) -> Option<Pid> { self.current }
    pub fn contains(&self, pid: Pid) -> bool { self.spaces.contains_key(&pid) }
    pub fn root_owner(&self, physical_address: u64) -> Option<Pid> { self.root_owners.get(&physical_address).copied() }
    pub fn get_mut(&mut self, pid: Pid) -> Result<&mut ProcessPageTable, PageTableError> { self.spaces.get_mut(&pid).ok_or(PageTableError::ProcessMissing) }
    /// A process address space cannot be destroyed while it still owns user mappings.
    /// Root ownership is removed only after this invariant is satisfied.
    pub fn destroy(&mut self, pid: Pid) -> Result<ProcessPageTable, PageTableError> {
        if self.current == Some(pid) { return Err(PageTableError::InvalidRoot); }
        let table = self.spaces.get(&pid).ok_or(PageTableError::ProcessMissing)?;
        if table.user_mapping_count() != 0 { return Err(PageTableError::OutstandingMappings); }
        let table = self.spaces.remove(&pid).ok_or(PageTableError::ProcessMissing)?;
        let root = table.root_frame().start_address().as_u64();
        self.root_owners.remove(&root);
        Ok(table)
    }
}
impl Default for ProcessAddressSpaceManager { fn default() -> Self { Self::new() } }
