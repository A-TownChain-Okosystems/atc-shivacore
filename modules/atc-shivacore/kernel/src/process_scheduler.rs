// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Process scheduling boundary for architecture-backed address-space switches.

#![cfg(feature = "x86-boot")]

use x86_64::{structures::paging::PageTable, VirtAddr};

use crate::ats1000::Pid;
use crate::memory::BootInfoFrameAllocator;
use crate::x86_64_page_table::{PageTableError, ProcessAddressSpaceManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitchError {
    ProcessMissing,
    InvalidState,
    AddressSpace(PageTableError),
}

impl From<PageTableError> for ContextSwitchError {
    fn from(value: PageTableError) -> Self { Self::AddressSpace(value) }
}

/// Kernel scheduler boundary for process/address-space context switching.
///
/// Scheduling policy remains separate from the architectural CR3 operation.
/// This object only permits switching to roots created and owned by its manager.
pub struct ProcessScheduler {
    address_spaces: ProcessAddressSpaceManager,
    current: Option<Pid>,
}

impl ProcessScheduler {
    pub const fn new(address_spaces: ProcessAddressSpaceManager) -> Self {
        Self { address_spaces, current: None }
    }

    pub fn current(&self) -> Option<Pid> { self.current }
    pub fn address_spaces(&self) -> &ProcessAddressSpaceManager { &self.address_spaces }
    pub fn address_spaces_mut(&mut self) -> &mut ProcessAddressSpaceManager { &mut self.address_spaces }

    /// Registers a process address space before the process becomes runnable.
    pub unsafe fn register_process(
        &mut self,
        pid: Pid,
        physical_memory_offset: VirtAddr,
        active_root: &PageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<(), ContextSwitchError> {
        self.address_spaces.create(pid, physical_memory_offset, active_root, frame_allocator)?;
        Ok(())
    }

    /// Switches the CPU to a manager-owned process root.
    pub unsafe fn switch_to(&mut self, next: Pid) -> Result<(), ContextSwitchError> {
        if !self.address_spaces.contains(next) {
            return Err(ContextSwitchError::ProcessMissing);
        }
        if self.current == Some(next) {
            return Ok(());
        }
        self.address_spaces.switch_to(next)?;
        self.current = Some(next);
        Ok(())
    }

    /// Removes only a non-running process address space.
    pub fn unregister_process(&mut self, pid: Pid) -> Result<(), ContextSwitchError> {
        if self.current == Some(pid) {
            return Err(ContextSwitchError::InvalidState);
        }
        self.address_spaces.destroy(pid)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_without_current_process() {
        let scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new());
        assert_eq!(scheduler.current(), None);
    }
}
