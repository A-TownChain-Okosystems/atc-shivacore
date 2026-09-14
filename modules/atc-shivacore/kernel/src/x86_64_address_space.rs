// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 process address-space context.
//!
//! This module provides the privileged CR3 context boundary. It deliberately
//! does not allocate or mutate page tables yet; callers must supply a validated
//! physical root frame. That keeps ownership of page-table memory explicit and
//! prevents an arbitrary process from manufacturing a CR3 value.

#![cfg(feature = "x86-boot")]

use x86_64::{registers::control::{Cr3, Cr3Flags}, PhysAddr, VirtAddr};
use crate::ats1000::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceSwitchError {
    InvalidPid,
    InvalidRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableRoot {
    pid: Pid,
    physical_address: u64,
}

impl PageTableRoot {
    pub fn new(pid: Pid, physical_address: u64) -> Result<Self, AddressSpaceSwitchError> {
        if pid.0 == 0 {
            return Err(AddressSpaceSwitchError::InvalidPid);
        }
        if physical_address == 0 || physical_address & 0xfff != 0 {
            return Err(AddressSpaceSwitchError::InvalidRoot);
        }
        Ok(Self { pid, physical_address })
    }

    pub const fn pid(self) -> Pid { self.pid }
    pub const fn physical_address(self) -> u64 { self.physical_address }
}

/// Switches CR3 to a kernel-created page-table root.
///
/// # Safety
/// The root must refer to a valid level-4 page table owned by the kernel and
/// its mappings must preserve the kernel's required privileged address space.
pub unsafe fn switch_to(root: PageTableRoot) {
    Cr3::write(PhysAddr::new(root.physical_address), Cr3Flags::empty());
}

/// Returns the physical CR3 root currently active on the CPU.
pub fn current_root() -> u64 {
    let (frame, _flags) = Cr3::read();
    frame.start_address().as_u64()
}

/// Converts a user virtual address into a canonical x86_64 address.
/// This is a validation helper only; it never dereferences the address.
pub fn validate_user_address(address: u64) -> bool {
    let canonical = VirtAddr::new(address);
    canonical.as_u64() == address && address >= 0x0000_0001_0000_0000 && address < 0x0000_8000_0000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_requires_page_alignment() {
        assert_eq!(PageTableRoot::new(Pid(1), 0x1001), Err(AddressSpaceSwitchError::InvalidRoot));
    }

    #[test]
    fn root_requires_nonzero_pid() {
        assert_eq!(PageTableRoot::new(Pid(0), 0x1000), Err(AddressSpaceSwitchError::InvalidPid));
    }

    #[test]
    fn canonical_user_addresses_are_accepted() {
        assert!(validate_user_address(0x1_0000_0000));
        assert!(!validate_user_address(0xffff_8000_0000_0000));
    }
}
