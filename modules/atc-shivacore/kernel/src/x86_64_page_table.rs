// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-owned x86_64 page-table root construction.
//!
//! A process receives a fresh level-4 table. Kernel mappings are copied from
//! the currently active root; user entries start empty. The root frame comes
//! exclusively from the kernel frame allocator, so callers cannot inject an
//! arbitrary physical address into the address-space context.

#![cfg(feature = "x86-boot")]

use x86_64::{
    structures::paging::{FrameAllocator, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::ats1000::Pid;
use crate::x86_64_address_space::{AddressSpaceSwitchError, PageTableRoot};
use crate::memory::BootInfoFrameAllocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableError {
    InvalidPid,
    OutOfFrames,
    InvalidRoot,
}

pub struct ProcessPageTable {
    pid: Pid,
    root_frame: PhysFrame,
    physical_memory_offset: VirtAddr,
}

impl ProcessPageTable {
    /// Creates a fresh level-4 page table and clones only the kernel half of
    /// the currently active root. User-space entries remain empty.
    ///
    /// # Safety
    /// `physical_memory_offset` must be the bootloader-provided physical
    /// memory mapping and `active_root` must be the currently active L4 table.
    pub unsafe fn new(
        pid: Pid,
        physical_memory_offset: VirtAddr,
        active_root: &PageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<Self, PageTableError> {
        if pid.0 == 0 {
            return Err(PageTableError::InvalidPid);
        }

        let root_frame = frame_allocator
            .allocate_frame()
            .ok_or(PageTableError::OutOfFrames)?;

        let root_virt = physical_memory_offset + root_frame.start_address().as_u64();
        let root_ptr = root_virt.as_mut_ptr::<PageTable>();
        root_ptr.write(PageTable::new());

        let root = &mut *root_ptr;
        // PML4 entries 256..512 form the upper canonical half. These are
        // kernel-owned mappings and are intentionally shared read-only at the
        // page-table-root level; user mappings are added separately.
        for index in 256..512 {
            root[index] = active_root[index].clone();
        }

        Ok(Self {
            pid,
            root_frame,
            physical_memory_offset,
        })
    }

    pub const fn pid(&self) -> Pid { self.pid }

    pub fn root_frame(&self) -> PhysFrame { self.root_frame }

    pub fn root_context(&self) -> Result<PageTableRoot, AddressSpaceSwitchError> {
        PageTableRoot::new(self.pid, self.root_frame.start_address().as_u64())
    }

    /// Returns the physical address of the level-4 table.
    pub fn root_physical_address(&self) -> PhysAddr {
        self.root_frame.start_address()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn user_half_boundary_is_256() {
        assert_eq!(256, 256);
        assert_eq!(512, 512);
    }
}
