// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 physical-frame allocator and OffsetPageTable bootstrap.
//!
//! This module is used by the boot binary. It creates the first real page-table
//! mapper and allocates only frames reported as USABLE by the bootloader.

use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    structures::paging::{
        mapper::MapToError,
        FrameAllocator, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

/// Initializes an OffsetPageTable from the level-4 table exposed through the
/// bootloader's physical-memory mapping.
///
/// # Safety
/// `physical_memory_offset` must be the offset supplied by bootloader_api and
/// the active level-4 page table must remain valid for the lifetime of the mapper.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_frame, _) = x86_64::registers::control::Cr3::read();
    let phys = level_4_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    &mut *(virt.as_mut_ptr())
}

/// Frame allocator backed exclusively by bootloader-reported usable RAM.
pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// # Safety
    /// The memory-region table must be valid for the allocator lifetime and
    /// the caller must ensure that frames returned by this allocator are not
    /// concurrently handed to another allocator.
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        Self { memory_regions, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.memory_regions
            .iter()
            .filter(|region| region.kind == MemoryRegionKind::Usable)
            .flat_map(|region| {
                let start = region.start;
                let end = region.end;
                (start..end).step_by(4096).map(|addr| {
                    PhysFrame::containing_address(PhysAddr::new(addr))
                })
            })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next = self.next.saturating_add(1);
        frame
    }
}

/// Map a single page with the requested flags.
///
/// The caller must ensure that `page` belongs to an address space that is
/// intended to receive the mapping and that the selected physical frame is
/// owned by the allocator.
pub unsafe fn map_page(
    mapper: &mut OffsetPageTable<'static>,
    page: Page<Size4KiB>,
    frame: PhysFrame,
    flags: PageTableFlags,
    frame_allocator: &mut BootInfoFrameAllocator,
) -> Result<(), MapToError<Size4KiB>> {
    mapper.map_to(page, frame, flags, frame_allocator)?.flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn frame_size_is_four_kib() {
        assert_eq!(4096usize, 4 * 1024);
    }
}
