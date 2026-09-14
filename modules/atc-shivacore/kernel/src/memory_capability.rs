// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Capability-bound memory operations.

extern crate alloc;

use crate::ats1000::Pid;
use crate::capability::{CapId, CapabilityTable, ResourceType, Rights};
use crate::frame_ownership::{FrameOwnership, FrameOwnershipError};
use crate::memory_isolation::{IsolationError, Mapping, PageFlags};
use crate::process_address_space::{AddressSpaceError, ProcessAddressSpaces};

#[cfg(feature = "x86-boot")]
use crate::memory::BootInfoFrameAllocator;
#[cfg(feature = "x86-boot")]
use crate::x86_64_page_table::{PageTableError, ProcessPageTable};
#[cfg(feature = "x86-boot")]
use x86_64::structures::paging::{Page, PhysFrame, Size4KiB};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCapabilityError {
    CapabilityMissing,
    InvalidCapability,
    AddressSpace(AddressSpaceError),
    Isolation(IsolationError),
    FrameOwnership(FrameOwnershipError),
    #[cfg(feature = "x86-boot")]
    PageTable(PageTableError),
}

impl From<AddressSpaceError> for MemoryCapabilityError { fn from(value: AddressSpaceError) -> Self { Self::AddressSpace(value) } }
impl From<IsolationError> for MemoryCapabilityError { fn from(value: IsolationError) -> Self { Self::Isolation(value) } }
impl From<FrameOwnershipError> for MemoryCapabilityError { fn from(value: FrameOwnershipError) -> Self { Self::FrameOwnership(value) } }
#[cfg(feature = "x86-boot")]
impl From<PageTableError> for MemoryCapabilityError { fn from(value: PageTableError) -> Self { Self::PageTable(value) } }

pub struct CapabilityMemory<'a> {
    pub capabilities: &'a CapabilityTable,
    pub spaces: &'a mut ProcessAddressSpaces,
    pub frames: &'a mut FrameOwnership,
}

impl<'a> CapabilityMemory<'a> {
    pub fn map(&mut self, pid: Pid, cap_id: CapId, mapping: Mapping) -> Result<(), MemoryCapabilityError> {
        self.require(pid, cap_id, Rights::WRITE)?;
        self.spaces.map(pid, mapping).map_err(Into::into)
    }

    pub fn unmap(&mut self, pid: Pid, cap_id: CapId, start: u64) -> Result<Mapping, MemoryCapabilityError> {
        self.require(pid, cap_id, Rights::WRITE)?;
        self.spaces.unmap(pid, start).map_err(Into::into)
    }

    pub fn check_read(&self, pid: Pid, cap_id: CapId, addr: u64, len: u64) -> Result<(), MemoryCapabilityError> {
        self.require(pid, cap_id, Rights::READ)?;
        self.spaces.check_access(pid, addr, len, PageFlags::READ).map_err(Into::into)
    }

    #[cfg(feature = "x86-boot")]
    pub unsafe fn map_user_page(
        &mut self, pid: Pid, cap_id: CapId, page: Page<Size4KiB>, frame: PhysFrame,
        flags: PageFlags, page_table: &mut ProcessPageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<(), MemoryCapabilityError> {
        self.require(pid, cap_id, Rights::WRITE)?;
        if page_table.pid() != pid { return Err(MemoryCapabilityError::InvalidCapability); }

        let mapping = crate::x86_64_user_mapping::mapping_for_page(page, flags)
            .map_err(crate::x86_64_user_mapping::UserMappingError::into)?;
        self.frames.track_allocated(pid, frame)?;
        if let Err(error) = self.spaces.map(pid, mapping) {
            let _ = self.frames.release_allocated(pid, frame);
            return Err(error.into());
        }
        if let Err(error) = page_table.map_user_page(page, frame, flags, frame_allocator) {
            let _ = self.spaces.unmap(pid, mapping.start);
            let _ = self.frames.release_allocated(pid, frame);
            return Err(error.into());
        }
        if let Err(error) = self.frames.mark_mapped(pid, frame, mapping.start) {
            let _ = page_table.unmap_user_page(page);
            let _ = self.spaces.unmap(pid, mapping.start);
            let _ = self.frames.release_allocated(pid, frame);
            return Err(error.into());
        }
        Ok(())
    }

    #[cfg(feature = "x86-boot")]
    /// Hardware unmapping happens first; policy and ownership are committed only
    /// after the MMU operation succeeds. The caller receives the now-unmapped frame.
    pub unsafe fn unmap_user_page(
        &mut self, pid: Pid, cap_id: CapId, page: Page<Size4KiB>,
        page_table: &mut ProcessPageTable,
    ) -> Result<PhysFrame, MemoryCapabilityError> {
        self.require(pid, cap_id, Rights::WRITE)?;
        if page_table.pid() != pid { return Err(MemoryCapabilityError::InvalidCapability); }

        let start = page.start_address().as_u64();
        let mapping = self.spaces.mapping_at(pid, start)?;
        let expected_frame = self.frames.validate_mapped(pid, start)?;

        let frame = page_table.unmap_user_page(page)?;
        if frame != expected_frame {
            return Err(MemoryCapabilityError::FrameOwnership(FrameOwnershipError::VirtualAddressMismatch));
        }

        let removed = self.spaces.unmap(pid, mapping.start)?;
        debug_assert_eq!(removed.start, mapping.start);
        self.frames.release_mapped(pid, frame, mapping.start)?;
        Ok(frame)
    }

    fn require(&self, pid: Pid, cap_id: CapId, rights: Rights) -> Result<(), MemoryCapabilityError> {
        if !self.capabilities.check_any(pid, cap_id.0, rights) { return Err(MemoryCapabilityError::CapabilityMissing); }
        let cap = self.capabilities.get(cap_id).ok_or(MemoryCapabilityError::InvalidCapability)?;
        if cap.resource_type != ResourceType::Memory || cap.resource_id != pid.0 as u64 { return Err(MemoryCapabilityError::InvalidCapability); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping() -> Mapping { Mapping { start: 0x1_0000_0000, size: 4096, flags: PageFlags::READ.union(PageFlags::USER) } }
    fn memory<'a>(caps: &'a CapabilityTable, spaces: &'a mut ProcessAddressSpaces, frames: &'a mut FrameOwnership) -> CapabilityMemory<'a> {
        CapabilityMemory { capabilities: caps, spaces, frames }
    }

    #[test]
    fn memory_mapping_requires_owned_memory_capability() {
        let pid = Pid(1); let mut caps = CapabilityTable::new();
        let cap = caps.create(pid, ResourceType::Memory, pid.0 as u64, Rights::READ | Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new(); spaces.create(pid).unwrap(); let mut frames = FrameOwnership::new();
        let mut memory = memory(&caps, &mut spaces, &mut frames); memory.map(pid, cap, mapping()).unwrap();
    }

    #[test]
    fn foreign_process_cannot_use_capability() {
        let owner = Pid(1); let attacker = Pid(2); let mut caps = CapabilityTable::new();
        let cap = caps.create(owner, ResourceType::Memory, owner.0 as u64, Rights::READ | Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new(); spaces.create(owner).unwrap(); spaces.create(attacker).unwrap(); let mut frames = FrameOwnership::new();
        let mut memory = memory(&caps, &mut spaces, &mut frames);
        assert_eq!(memory.map(attacker, cap, mapping()), Err(MemoryCapabilityError::CapabilityMissing));
    }

    #[test]
    fn mismatched_memory_scope_is_rejected() {
        let pid = Pid(3); let mut caps = CapabilityTable::new();
        let cap = caps.create(pid, ResourceType::Memory, 99, Rights::READ | Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new(); spaces.create(pid).unwrap(); let mut frames = FrameOwnership::new();
        let mut memory = memory(&caps, &mut spaces, &mut frames);
        assert_eq!(memory.map(pid, cap, mapping()), Err(MemoryCapabilityError::InvalidCapability));
    }

    #[test]
    fn read_requires_read_right() {
        let pid = Pid(3); let mut caps = CapabilityTable::new();
        let cap = caps.create(pid, ResourceType::Memory, pid.0 as u64, Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new(); spaces.create(pid).unwrap(); let mut frames = FrameOwnership::new();
        let memory = memory(&caps, &mut spaces, &mut frames);
        assert_eq!(memory.check_read(pid, cap, 0x1_0000_0000, 1), Err(MemoryCapabilityError::CapabilityMissing));
    }
}
