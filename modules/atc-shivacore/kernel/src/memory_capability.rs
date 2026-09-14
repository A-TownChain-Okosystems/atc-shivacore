// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Capability-bound memory operations.
//!
//! The policy layer validates process ownership and capabilities before a
//! mapping is installed in the process registry and hardware page table.

extern crate alloc;

use crate::ats1000::Pid;
use crate::capability::{CapId, CapabilityTable, ResourceType, Rights};
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
    #[cfg(feature = "x86-boot")]
    PageTable(PageTableError),
}

impl From<AddressSpaceError> for MemoryCapabilityError {
    fn from(value: AddressSpaceError) -> Self { Self::AddressSpace(value) }
}

impl From<IsolationError> for MemoryCapabilityError {
    fn from(value: IsolationError) -> Self { Self::Isolation(value) }
}

#[cfg(feature = "x86-boot")]
impl From<PageTableError> for MemoryCapabilityError {
    fn from(value: PageTableError) -> Self { Self::PageTable(value) }
}

/// Coordinates capability authorization with the process-owned mapping policy.
pub struct CapabilityMemory<'a> {
    pub capabilities: &'a CapabilityTable,
    pub spaces: &'a mut ProcessAddressSpaces,
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
    /// Authorize and install one user page in the process-owned page table.
    ///
    /// The process page table creates its own mapper from its own L4 root;
    /// callers cannot accidentally supply a mapper for another process.
    /// The policy registry is updated only after hardware mapping succeeds.
    pub unsafe fn map_user_page(
        &mut self,
        pid: Pid,
        cap_id: CapId,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        flags: PageFlags,
        page_table: &mut ProcessPageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<(), MemoryCapabilityError> {
        self.require(pid, cap_id, Rights::WRITE)?;
        if page_table.pid() != pid {
            return Err(MemoryCapabilityError::InvalidCapability);
        }

        let mapping = crate::x86_64_user_mapping::mapping_for_page(page, flags)
            .map_err(crate::x86_64_user_mapping::UserMappingError::into)?;

        // Reserve the policy mapping first; hardware failure is rolled back.
        self.spaces.map(pid, mapping)?;

        if let Err(error) = page_table.map_user_page(page, frame, flags, frame_allocator) {
            let _ = self.spaces.unmap(pid, mapping.start);
            return Err(error.into());
        }
        Ok(())
    }

    fn require(&self, pid: Pid, cap_id: CapId, rights: Rights) -> Result<(), MemoryCapabilityError> {
        if !self.capabilities.check_any(pid, cap_id.0, rights) {
            return Err(MemoryCapabilityError::CapabilityMissing);
        }
        let cap = self.capabilities.get(cap_id).ok_or(MemoryCapabilityError::InvalidCapability)?;
        if cap.resource_type != ResourceType::Memory || cap.resource_id != pid.0 as u64 {
            return Err(MemoryCapabilityError::InvalidCapability);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping() -> Mapping {
        Mapping { start: 0x1_0000_0000, size: 4096, flags: PageFlags::READ.union(PageFlags::USER) }
    }

    #[test]
    fn memory_mapping_requires_owned_memory_capability() {
        let pid = Pid(1);
        let mut caps = CapabilityTable::new();
        let cap = caps.create(pid, ResourceType::Memory, pid.0 as u64, Rights::READ | Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new();
        spaces.create(pid).unwrap();
        let mut memory = CapabilityMemory { capabilities: &caps, spaces: &mut spaces };
        memory.map(pid, cap, mapping()).unwrap();
    }

    #[test]
    fn foreign_process_cannot_use_capability() {
        let owner = Pid(1);
        let attacker = Pid(2);
        let mut caps = CapabilityTable::new();
        let cap = caps.create(owner, ResourceType::Memory, owner.0 as u64, Rights::READ | Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new();
        spaces.create(owner).unwrap();
        spaces.create(attacker).unwrap();
        let mut memory = CapabilityMemory { capabilities: &caps, spaces: &mut spaces };
        assert_eq!(memory.map(attacker, cap, mapping()), Err(MemoryCapabilityError::CapabilityMissing));
    }

    #[test]
    fn mismatched_memory_scope_is_rejected() {
        let pid = Pid(3);
        let mut caps = CapabilityTable::new();
        let cap = caps.create(pid, ResourceType::Memory, 99, Rights::READ | Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new();
        spaces.create(pid).unwrap();
        let mut memory = CapabilityMemory { capabilities: &caps, spaces: &mut spaces };
        assert_eq!(memory.map(pid, cap, mapping()), Err(MemoryCapabilityError::InvalidCapability));
    }

    #[test]
    fn read_requires_read_right() {
        let pid = Pid(3);
        let mut caps = CapabilityTable::new();
        let cap = caps.create(pid, ResourceType::Memory, pid.0 as u64, Rights::WRITE);
        let mut spaces = ProcessAddressSpaces::new();
        spaces.create(pid).unwrap();
        let memory = CapabilityMemory { capabilities: &caps, spaces: &mut spaces };
        assert_eq!(memory.check_read(pid, cap, 0x1_0000_0000, 1), Err(MemoryCapabilityError::CapabilityMissing));
    }
}
