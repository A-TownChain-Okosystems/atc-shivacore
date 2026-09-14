// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Capability-bound memory operations.
//!
//! A Memory capability is scoped to one process address space. Mapping and
//! unmapping require WRITE authority; access checks require READ authority.
//! Hardware page-table mutation remains outside this policy layer.

extern crate alloc;

use crate::ats1000::Pid;
use crate::capability::{CapId, CapabilityTable, ResourceType, Rights};
use crate::memory_isolation::{IsolationError, Mapping, PageFlags};
use crate::process_address_space::{AddressSpaceError, ProcessAddressSpaces};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCapabilityError {
    CapabilityMissing,
    InvalidCapability,
    AddressSpace(AddressSpaceError),
    Isolation(IsolationError),
}

impl From<AddressSpaceError> for MemoryCapabilityError {
    fn from(value: AddressSpaceError) -> Self { Self::AddressSpace(value) }
}

impl From<IsolationError> for MemoryCapabilityError {
    fn from(value: IsolationError) -> Self { Self::Isolation(value) }
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
