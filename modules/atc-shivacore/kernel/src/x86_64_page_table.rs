// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-owned x86_64 process page-table lifecycle.

#![cfg(feature = "x86-boot")]

extern crate alloc;

use alloc::collections::BTreeMap;

use x86_64::{
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::ats1000::Pid;
use crate::memory::BootInfoFrameAllocator;
use crate::memory_isolation::PageFlags;
use crate::x86_64_address_space::{self, AddressSpaceSwitchError, PageTableRoot};
use crate::x86_64_user_mapping::{map_user_page, validate_user_page, UserMappingError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableError {
    InvalidPid,
    OutOfFrames,
    InvalidRoot,
    Mapping(UserMappingError),
    NotMapped,
    ProcessExists,
    ProcessMissing,
}

impl From<UserMappingError> for PageTableError {
    fn from(value: UserMappingError) -> Self { Self::Mapping(value) }
}

pub struct ProcessPageTable {
    pid: Pid,
    root_frame: PhysFrame,
    physical_memory_offset: VirtAddr,
}

impl ProcessPageTable {
    pub unsafe fn new(
        pid: Pid,
        physical_memory_offset: VirtAddr,
        active_root: &PageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<Self, PageTableError> {
        if pid.0 == 0 { return Err(PageTableError::InvalidPid); }

        let root_frame = frame_allocator.allocate_frame().ok_or(PageTableError::OutOfFrames)?;
        let root_virt = physical_memory_offset + root_frame.start_address().as_u64();
        let root_ptr = root_virt.as_mut_ptr::<PageTable>();
        root_ptr.write(PageTable::new());
        let root = &mut *root_ptr;
        for index in 256..512 { root[index] = active_root[index].clone(); }

        Ok(Self { pid, root_frame, physical_memory_offset })
    }

    pub const fn pid(&self) -> Pid { self.pid }
    pub fn root_frame(&self) -> PhysFrame { self.root_frame }

    pub fn root_context(&self) -> Result<PageTableRoot, AddressSpaceSwitchError> {
        PageTableRoot::new(self.pid, self.root_frame.start_address().as_u64())
    }

    pub fn root_physical_address(&self) -> PhysAddr { self.root_frame.start_address() }

    pub unsafe fn mapper(&mut self) -> OffsetPageTable<'static> {
        let root_virt = self.physical_memory_offset + self.root_frame.start_address().as_u64();
        let root = &mut *root_virt.as_mut_ptr::<PageTable>();
        OffsetPageTable::new(root, self.physical_memory_offset)
    }

    pub unsafe fn map_user_page(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        flags: PageFlags,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<(), PageTableError> {
        let mut mapper = self.mapper();
        map_user_page(&mut mapper, page, frame, flags, frame_allocator).map_err(Into::into)
    }

    pub unsafe fn unmap_user_page(&mut self, page: Page<Size4KiB>) -> Result<PhysFrame, PageTableError> {
        let mut mapper = self.mapper();
        let (frame, flush) = mapper.unmap(page).map_err(|_| PageTableError::NotMapped)?;
        flush.flush();
        Ok(frame)
    }

    pub fn validate_user_page(&self, page: Page<Size4KiB>, flags: PageFlags) -> Result<(), PageTableError> {
        validate_user_page(page, flags)?;
        Ok(())
    }
}

/// Owns the kernel-created page-table roots for live processes and provides
/// the process-level CR3 switching boundary.
pub struct ProcessAddressSpaceManager {
    spaces: BTreeMap<Pid, ProcessPageTable>,
    current: Option<Pid>,
}

impl ProcessAddressSpaceManager {
    pub const fn new() -> Self {
        Self { spaces: BTreeMap::new(), current: None }
    }

    /// Creates a private L4 root by cloning only the kernel half of `active_root`.
    pub unsafe fn create(
        &mut self,
        pid: Pid,
        physical_memory_offset: VirtAddr,
        active_root: &PageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<(), PageTableError> {
        if self.spaces.contains_key(&pid) { return Err(PageTableError::ProcessExists); }
        let table = ProcessPageTable::new(pid, physical_memory_offset, active_root, frame_allocator)?;
        self.spaces.insert(pid, table);
        Ok(())
    }

    pub fn root(&self, pid: Pid) -> Result<PageTableRoot, PageTableError> {
        self.spaces.get(&pid).ok_or(PageTableError::ProcessMissing)?.root_context().map_err(|_| PageTableError::InvalidRoot)
    }

    /// Switches CR3 to the process root and records the active PID.
    ///
    /// # Safety
    /// The process root must have been created by this manager and its kernel
    /// mappings must remain valid on the current CPU.
    pub unsafe fn switch_to(&mut self, pid: Pid) -> Result<(), PageTableError> {
        let root = self.root(pid)?;
        x86_64_address_space::switch_to(root);
        self.current = Some(pid);
        Ok(())
    }

    pub fn current_pid(&self) -> Option<Pid> { self.current }
    pub fn contains(&self, pid: Pid) -> bool { self.spaces.contains_key(&pid) }

    pub fn get_mut(&mut self, pid: Pid) -> Result<&mut ProcessPageTable, PageTableError> {
        self.spaces.get_mut(&pid).ok_or(PageTableError::ProcessMissing)
    }

    /// A live address space cannot be destroyed while its CR3 is active.
    pub fn destroy(&mut self, pid: Pid) -> Result<ProcessPageTable, PageTableError> {
        if self.current == Some(pid) { return Err(PageTableError::InvalidRoot); }
        self.spaces.remove(&pid).ok_or(PageTableError::ProcessMissing)
    }
}

impl Default for ProcessAddressSpaceManager {
    fn default() -> Self { Self::new() }
}
