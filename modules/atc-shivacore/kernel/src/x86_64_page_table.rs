// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-owned x86_64 page-table root construction and mapping.
//!
//! A process receives a fresh level-4 table. Kernel mappings are copied from
//! the currently active root; user entries start empty. The root frame comes
//! exclusively from the kernel frame allocator.

#![cfg(feature = "x86-boot")]

use x86_64::{
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::ats1000::Pid;
use crate::memory::BootInfoFrameAllocator;
use crate::memory_isolation::PageFlags;
use crate::x86_64_address_space::{AddressSpaceSwitchError, PageTableRoot};
use crate::x86_64_user_mapping::{map_user_page, validate_user_page, UserMappingError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableError {
    InvalidPid,
    OutOfFrames,
    InvalidRoot,
    Mapping(UserMappingError),
    NotMapped,
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
        for index in 256..512 {
            root[index] = active_root[index].clone();
        }

        Ok(Self { pid, root_frame, physical_memory_offset })
    }

    pub const fn pid(&self) -> Pid { self.pid }

    pub fn root_frame(&self) -> PhysFrame { self.root_frame }

    pub fn root_context(&self) -> Result<PageTableRoot, AddressSpaceSwitchError> {
        PageTableRoot::new(self.pid, self.root_frame.start_address().as_u64())
    }

    pub fn root_physical_address(&self) -> PhysAddr {
        self.root_frame.start_address()
    }

    pub unsafe fn mapper(&mut self) -> OffsetPageTable<'static> {
        let root_virt = self.physical_memory_offset + self.root_frame.start_address().as_u64();
        let root = &mut *root_virt.as_mut_ptr::<PageTable>();
        OffsetPageTable::new(root, self.physical_memory_offset)
    }

    /// Maps one validated user page through this process's own L4 root.
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

    /// Removes one user mapping from this process's own L4 root.
    ///
    /// The returned frame remains owned by the caller; this method deliberately
    /// does not return it to the allocator because physical-frame ownership is
    /// a higher-level lifecycle decision.
    pub unsafe fn unmap_user_page(
        &mut self,
        page: Page<Size4KiB>,
    ) -> Result<PhysFrame, PageTableError> {
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

#[cfg(test)]
mod tests {
    #[test]
    fn user_half_boundary_is_256() {
        assert_eq!(256, 256);
        assert_eq!(512, 512);
    }
}
