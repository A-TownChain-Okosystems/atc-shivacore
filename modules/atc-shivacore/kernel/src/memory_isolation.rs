// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Process memory-isolation contract.
//!
//! This layer is deliberately architecture-neutral. It establishes the
//! security invariants that an x86_64/aarch64 page-table backend must enforce:
//! - one address space per process;
//! - page-aligned, non-overlapping mappings;
//! - explicit user/kernel separation;
//! - no mapping may cross the user address ceiling;
//! - access rights are checked before a mapping is exposed.
//!
//! This is not a hardware MMU implementation yet. No physical memory is
//! dereferenced here. The page-table backend is the next layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use crate::ats1000::Pid;

pub const PAGE_SIZE: u64 = 4096;
pub const USER_BASE: u64 = 0x0000_0001_0000_0000;
pub const USER_LIMIT: u64 = 0x0000_7fff_ffff_f000;
pub const KERNEL_BASE: u64 = 0xffff_8000_0000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags(u8);

impl PageFlags {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);

    pub const fn empty() -> Self { Self(0) }
    pub const fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }
    pub const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationError {
    InvalidPid,
    ZeroLength,
    NotPageAligned,
    AddressOverflow,
    OutsideUserRange,
    KernelAddress,
    Overlap,
    NotMapped,
    PermissionDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub start: u64,
    pub size: u64,
    pub flags: PageFlags,
}

impl Mapping {
    pub fn end(&self) -> Result<u64, IsolationError> {
        self.start.checked_add(self.size).ok_or(IsolationError::AddressOverflow)
    }

    pub fn contains(&self, addr: u64, len: u64) -> bool {
        match (addr.checked_add(len), self.end()) {
            (Some(request_end), Ok(mapping_end)) => addr >= self.start && request_end <= mapping_end,
            _ => false,
        }
    }
}

/// One virtual address space owned by exactly one process.
pub struct AddressSpace {
    owner: Pid,
    mappings: BTreeMap<u64, Mapping>,
}

impl AddressSpace {
    pub fn new(owner: Pid) -> Result<Self, IsolationError> {
        if owner.0 == 0 { return Err(IsolationError::InvalidPid); }
        Ok(Self { owner, mappings: BTreeMap::new() })
    }

    pub const fn owner(&self) -> Pid { self.owner }

    pub fn map(&mut self, mapping: Mapping) -> Result<(), IsolationError> {
        validate_user_mapping(&mapping)?;
        let end = mapping.end()?;

        if let Some((_, previous)) = self.mappings.range(..=mapping.start).next_back() {
            if previous.end()? > mapping.start { return Err(IsolationError::Overlap); }
        }
        if let Some((_, next)) = self.mappings.range(mapping.start..).next() {
            if end > next.start { return Err(IsolationError::Overlap); }
        }

        self.mappings.insert(mapping.start, mapping);
        Ok(())
    }

    pub fn unmap(&mut self, start: u64) -> Result<Mapping, IsolationError> {
        self.mappings.remove(&start).ok_or(IsolationError::NotMapped)
    }

    pub fn check_access(&self, addr: u64, len: u64, required: PageFlags) -> Result<(), IsolationError> {
        if len == 0 { return Err(IsolationError::ZeroLength); }
        let mapping = self.mappings.range(..=addr).next_back()
            .map(|(_, mapping)| *mapping)
            .ok_or(IsolationError::NotMapped)?;
        if !mapping.contains(addr, len) { return Err(IsolationError::NotMapped); }
        if !mapping.flags.contains(required) { return Err(IsolationError::PermissionDenied); }
        Ok(())
    }

    pub fn mappings(&self) -> impl Iterator<Item = &Mapping> { self.mappings.values() }
}

pub struct MemoryIsolation {
    spaces: BTreeMap<Pid, AddressSpace>,
}

impl MemoryIsolation {
    pub fn new() -> Self { Self { spaces: BTreeMap::new() } }

    pub fn create_space(&mut self, pid: Pid) -> Result<(), IsolationError> {
        if self.spaces.contains_key(&pid) { return Ok(()); }
        self.spaces.insert(pid, AddressSpace::new(pid)?);
        Ok(())
    }

    pub fn destroy_space(&mut self, pid: Pid) -> Result<(), IsolationError> {
        self.spaces.remove(&pid).map(|_| ()).ok_or(IsolationError::NotMapped)
    }

    pub fn map(&mut self, pid: Pid, mapping: Mapping) -> Result<(), IsolationError> {
        self.spaces.get_mut(&pid).ok_or(IsolationError::NotMapped)?.map(mapping)
    }

    pub fn unmap(&mut self, pid: Pid, start: u64) -> Result<Mapping, IsolationError> {
        self.spaces.get_mut(&pid).ok_or(IsolationError::NotMapped)?.unmap(start)
    }

    pub fn check_access(&self, pid: Pid, addr: u64, len: u64, required: PageFlags) -> Result<(), IsolationError> {
        self.spaces.get(&pid).ok_or(IsolationError::NotMapped)?.check_access(addr, len, required)
    }

    pub fn contains_space(&self, pid: Pid) -> bool { self.spaces.contains_key(&pid) }
}

fn validate_user_mapping(mapping: &Mapping) -> Result<(), IsolationError> {
    if mapping.size == 0 { return Err(IsolationError::ZeroLength); }
    if mapping.start % PAGE_SIZE != 0 || mapping.size % PAGE_SIZE != 0 {
        return Err(IsolationError::NotPageAligned);
    }
    let end = mapping.end()?;
    if mapping.start < USER_BASE || end > USER_LIMIT { return Err(IsolationError::OutsideUserRange); }
    if mapping.start >= KERNEL_BASE || end > KERNEL_BASE { return Err(IsolationError::KernelAddress); }
    if !mapping.flags.contains(PageFlags::USER) { return Err(IsolationError::PermissionDenied); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(n: u32) -> Pid { Pid(n) }
    fn rw_user() -> PageFlags { PageFlags::READ.union(PageFlags::WRITE).union(PageFlags::USER) }

    #[test]
    fn process_spaces_are_independent() {
        let mut isolation = MemoryIsolation::new();
        isolation.create_space(pid(1)).unwrap();
        isolation.create_space(pid(2)).unwrap();
        let mapping = Mapping { start: USER_BASE, size: PAGE_SIZE, flags: rw_user() };
        isolation.map(pid(1), mapping).unwrap();
        assert!(isolation.check_access(pid(1), USER_BASE, 8, PageFlags::READ).is_ok());
        assert_eq!(isolation.check_access(pid(2), USER_BASE, 8, PageFlags::READ), Err(IsolationError::NotMapped));
    }

    #[test]
    fn overlapping_mappings_are_rejected() {
        let mut space = AddressSpace::new(pid(1)).unwrap();
        space.map(Mapping { start: USER_BASE, size: PAGE_SIZE * 2, flags: rw_user() }).unwrap();
        assert_eq!(space.map(Mapping { start: USER_BASE + PAGE_SIZE, size: PAGE_SIZE, flags: rw_user() }), Err(IsolationError::Overlap));
    }

    #[test]
    fn unaligned_mappings_are_rejected() {
        let mut space = AddressSpace::new(pid(1)).unwrap();
        assert_eq!(space.map(Mapping { start: USER_BASE + 1, size: PAGE_SIZE, flags: rw_user() }), Err(IsolationError::NotPageAligned));
    }

    #[test]
    fn kernel_addresses_are_never_user_mapped() {
        let mut space = AddressSpace::new(pid(1)).unwrap();
        assert_eq!(space.map(Mapping { start: KERNEL_BASE, size: PAGE_SIZE, flags: rw_user() }), Err(IsolationError::OutsideUserRange));
    }

    #[test]
    fn write_requires_write_permission() {
        let mut space = AddressSpace::new(pid(1)).unwrap();
        space.map(Mapping { start: USER_BASE, size: PAGE_SIZE, flags: PageFlags::READ.union(PageFlags::USER) }).unwrap();
        assert_eq!(space.check_access(pid(1), USER_BASE, 1, PageFlags::WRITE), Err(IsolationError::PermissionDenied));
    }

    #[test]
    fn access_cannot_cross_mapping_boundary() {
        let mut space = AddressSpace::new(pid(1)).unwrap();
        space.map(Mapping { start: USER_BASE, size: PAGE_SIZE, flags: rw_user() }).unwrap();
        assert_eq!(space.check_access(USER_BASE, PAGE_SIZE + 1, PageFlags::READ), Err(IsolationError::NotMapped));
    }
}
