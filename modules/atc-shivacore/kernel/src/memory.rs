// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 physical-frame allocator and OffsetPageTable bootstrap.

extern crate alloc;
use alloc::collections::BTreeSet;
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    structures::paging::{mapper::MapToError, FrameAllocator, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

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

/// Bootloader-backed allocator with an explicit reclaimed-frame pool.
/// Reclamation is accepted only from an owner that has already completed its
/// mapping/lifetime checks; the allocator itself does not infer ownership.
pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
    recycled: BTreeSet<u64>,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        Self { memory_regions, next: 0, recycled: BTreeSet::new() }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.memory_regions
            .iter()
            .filter(|region| region.kind == MemoryRegionKind::Usable)
            .flat_map(|region| {
                (region.start..region.end)
                    .step_by(4096)
                    .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
            })
    }

    pub fn reclaim_frame(&mut self, frame: PhysFrame) -> bool {
        self.recycled.insert(frame.start_address().as_u64())
    }

    pub fn recycled_count(&self) -> usize { self.recycled.len() }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(address) = self.recycled.pop_first() {
            return Some(PhysFrame::containing_address(PhysAddr::new(address)));
        }
        let frame = self.usable_frames().nth(self.next);
        self.next = self.next.saturating_add(1);
        frame
    }
}

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
    fn frame_size_is_four_kib() { assert_eq!(4096usize, 4 * 1024); }
}
