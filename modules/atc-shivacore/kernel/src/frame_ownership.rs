// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-side physical frame ownership ledger.

use alloc::collections::BTreeMap;
use x86_64::{PhysAddr, structures::paging::{PhysFrame, Size4KiB}};
use crate::ats1000::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    Allocated { owner: Pid },
    Mapped { owner: Pid, virtual_address: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOwnershipError {
    InvalidPid,
    AlreadyTracked,
    UnknownFrame,
    WrongOwner,
    AlreadyMapped,
    NotMapped,
    VirtualAddressMismatch,
}

pub struct FrameOwnership { frames: BTreeMap<u64, FrameState> }

impl FrameOwnership {
    pub const fn new() -> Self { Self { frames: BTreeMap::new() } }

    pub fn track_allocated(&mut self, owner: Pid, frame: PhysFrame) -> Result<(), FrameOwnershipError> {
        if owner.0 == 0 { return Err(FrameOwnershipError::InvalidPid); }
        let key = frame.start_address().as_u64();
        if self.frames.contains_key(&key) { return Err(FrameOwnershipError::AlreadyTracked); }
        self.frames.insert(key, FrameState::Allocated { owner });
        Ok(())
    }

    pub fn mark_mapped(&mut self, owner: Pid, frame: PhysFrame, virtual_address: u64) -> Result<(), FrameOwnershipError> {
        if owner.0 == 0 { return Err(FrameOwnershipError::InvalidPid); }
        let key = frame.start_address().as_u64();
        match self.frames.get(&key).copied() {
            Some(FrameState::Allocated { owner: current }) if current == owner => {
                self.frames.insert(key, FrameState::Mapped { owner, virtual_address });
                Ok(())
            }
            Some(FrameState::Mapped { .. }) => Err(FrameOwnershipError::AlreadyMapped),
            Some(FrameState::Allocated { .. }) => Err(FrameOwnershipError::WrongOwner),
            None => Err(FrameOwnershipError::UnknownFrame),
        }
    }

    pub fn validate_mapped(&self, owner: Pid, virtual_address: u64) -> Result<PhysFrame, FrameOwnershipError> {
        for (address, state) in &self.frames {
            if let FrameState::Mapped { owner: current, virtual_address: current_va } = *state {
                if current == owner && current_va == virtual_address {
                    return Ok(PhysFrame::containing_address(PhysAddr::new(*address)));
                }
            }
        }
        Err(FrameOwnershipError::NotMapped)
    }

    pub fn release_mapped(&mut self, owner: Pid, frame: PhysFrame, virtual_address: u64) -> Result<(), FrameOwnershipError> {
        let key = frame.start_address().as_u64();
        match self.frames.get(&key).copied() {
            Some(FrameState::Mapped { owner: current, virtual_address: current_va })
                if current == owner && current_va == virtual_address => {
                    self.frames.remove(&key);
                    Ok(())
                }
            Some(FrameState::Mapped { owner: current, .. }) if current != owner => Err(FrameOwnershipError::WrongOwner),
            Some(FrameState::Mapped { .. }) => Err(FrameOwnershipError::VirtualAddressMismatch),
            Some(FrameState::Allocated { .. }) => Err(FrameOwnershipError::NotMapped),
            None => Err(FrameOwnershipError::UnknownFrame),
        }
    }

    pub fn release_allocated(&mut self, owner: Pid, frame: PhysFrame) -> Result<(), FrameOwnershipError> {
        let key = frame.start_address().as_u64();
        match self.frames.get(&key).copied() {
            Some(FrameState::Allocated { owner: current }) if current == owner => {
                self.frames.remove(&key);
                Ok(())
            }
            Some(FrameState::Allocated { .. }) => Err(FrameOwnershipError::WrongOwner),
            Some(FrameState::Mapped { .. }) => Err(FrameOwnershipError::AlreadyMapped),
            None => Err(FrameOwnershipError::UnknownFrame),
        }
    }

    pub fn state(&self, frame: PhysFrame) -> Option<FrameState> { self.frames.get(&frame.start_address().as_u64()).copied() }
    pub fn contains(&self, frame: PhysFrame) -> bool { self.frames.contains_key(&frame.start_address().as_u64()) }
    pub fn len(&self) -> usize { self.frames.len() }
}

impl Default for FrameOwnership { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(address: u64) -> PhysFrame { PhysFrame::containing_address(PhysAddr::new(address)) }

    #[test]
    fn lifecycle_is_allocated_mapped_released() {
        let pid = Pid(1); let f = frame(0x20_0000); let mut ledger = FrameOwnership::new();
        ledger.track_allocated(pid, f).unwrap(); ledger.mark_mapped(pid, f, 0x1_0000_0000).unwrap();
        assert_eq!(ledger.validate_mapped(pid, 0x1_0000_0000), Ok(f));
        ledger.release_mapped(pid, f, 0x1_0000_0000).unwrap(); assert!(!ledger.contains(f));
    }

    #[test]
    fn second_mapping_is_rejected() {
        let pid = Pid(1); let f = frame(0x21_0000); let mut ledger = FrameOwnership::new();
        ledger.track_allocated(pid, f).unwrap(); ledger.mark_mapped(pid, f, 0x1_0000_0000).unwrap();
        assert_eq!(ledger.mark_mapped(pid, f, 0x1_0000_1000), Err(FrameOwnershipError::AlreadyMapped));
    }

    #[test]
    fn wrong_owner_cannot_release() {
        let f = frame(0x22_0000); let mut ledger = FrameOwnership::new(); ledger.track_allocated(Pid(1), f).unwrap();
        assert_eq!(ledger.release_allocated(Pid(2), f), Err(FrameOwnershipError::WrongOwner));
    }

    #[test]
    fn mapped_frame_cannot_be_released_as_allocated() {
        let pid = Pid(1); let f = frame(0x23_0000); let mut ledger = FrameOwnership::new();
        ledger.track_allocated(pid, f).unwrap(); ledger.mark_mapped(pid, f, 0x1_0000_0000).unwrap();
        assert_eq!(ledger.release_allocated(pid, f), Err(FrameOwnershipError::AlreadyMapped));
    }
}
