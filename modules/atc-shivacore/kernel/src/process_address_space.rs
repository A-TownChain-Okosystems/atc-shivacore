// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Process-owned address-space mapping registry.
//!
//! This layer binds the architecture-neutral memory-isolation contract to a
//! process identity before an architecture-specific page-table backend is
//! invoked. It deliberately does not expose raw page-table authority.

extern crate alloc;

use alloc::collections::BTreeMap;
use crate::ats1000::Pid;
use crate::memory_isolation::{AddressSpace, IsolationError, Mapping, PageFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceError {
    Isolation(IsolationError),
    ProcessMissing,
    MappingOwnerMismatch,
    AlreadyMapped,
}

impl From<IsolationError> for AddressSpaceError {
    fn from(value: IsolationError) -> Self { Self::Isolation(value) }
}

/// Kernel-owned registry of process address spaces.
///
/// The registry is the authority boundary used by future architecture
/// backends. A process can only mutate mappings through its own PID entry.
pub struct ProcessAddressSpaces {
    spaces: BTreeMap<Pid, AddressSpace>,
}

impl ProcessAddressSpaces {
    pub const fn new() -> Self {
        Self { spaces: BTreeMap::new() }
    }

    pub fn create(&mut self, pid: Pid) -> Result<(), AddressSpaceError> {
        if pid.0 == 0 {
            return Err(AddressSpaceError::Isolation(IsolationError::InvalidPid));
        }
        self.spaces.entry(pid).or_insert(AddressSpace::new(pid)?);
        Ok(())
    }

    pub fn destroy(&mut self, pid: Pid) -> Result<(), AddressSpaceError> {
        self.spaces.remove(&pid).map(|_| ()).ok_or(AddressSpaceError::ProcessMissing)
    }

    pub fn map(&mut self, pid: Pid, mapping: Mapping) -> Result<(), AddressSpaceError> {
        let space = self.spaces.get_mut(&pid).ok_or(AddressSpaceError::ProcessMissing)?;
        if space.mappings().any(|existing| existing.start == mapping.start) {
            return Err(AddressSpaceError::AlreadyMapped);
        }
        space.map(mapping).map_err(Into::into)
    }

    pub fn unmap(&mut self, pid: Pid, start: u64) -> Result<Mapping, AddressSpaceError> {
        self.spaces.get_mut(&pid)
            .ok_or(AddressSpaceError::ProcessMissing)?
            .unmap(start)
            .map_err(Into::into)
    }

    pub fn check_access(
        &self,
        pid: Pid,
        addr: u64,
        len: u64,
        required: PageFlags,
    ) -> Result<(), AddressSpaceError> {
        self.spaces.get(&pid)
            .ok_or(AddressSpaceError::ProcessMissing)?
            .check_access(addr, len, required)
            .map_err(Into::into)
    }

    pub fn mappings(&self, pid: Pid) -> Result<impl Iterator<Item = &Mapping>, AddressSpaceError> {
        self.spaces.get(&pid)
            .map(AddressSpace::mappings)
            .ok_or(AddressSpaceError::ProcessMissing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(start: u64) -> Mapping {
        Mapping { start, size: 4096, flags: PageFlags::READ.union(PageFlags::USER) }
    }

    #[test]
    fn mappings_are_process_owned() {
        let mut spaces = ProcessAddressSpaces::new();
        let a = Pid(1);
        let b = Pid(2);
        spaces.create(a).unwrap();
        spaces.create(b).unwrap();
        spaces.map(a, mapping(0x1_0000_0000)).unwrap();
        assert!(spaces.check_access(a, 0x1_0000_0000, 1, PageFlags::READ).is_ok());
        assert!(spaces.check_access(b, 0x1_0000_0000, 1, PageFlags::READ).is_err());
    }

    #[test]
    fn unmap_is_scoped_to_process() {
        let mut spaces = ProcessAddressSpaces::new();
        let a = Pid(1);
        let b = Pid(2);
        spaces.create(a).unwrap();
        spaces.create(b).unwrap();
        spaces.map(a, mapping(0x1_0000_0000)).unwrap();
        assert!(spaces.unmap(b, 0x1_0000_0000).is_err());
        assert!(spaces.unmap(a, 0x1_0000_0000).is_ok());
    }

    #[test]
    fn overlapping_mapping_is_rejected() {
        let mut spaces = ProcessAddressSpaces::new();
        let pid = Pid(7);
        spaces.create(pid).unwrap();
        spaces.map(pid, mapping(0x1_0000_0000)).unwrap();
        assert!(spaces.map(pid, mapping(0x1_0000_0000 + 4096)).is_ok());
        assert!(spaces.map(pid, mapping(0x1_0000_0000 + 2048)).is_err());
    }
}
