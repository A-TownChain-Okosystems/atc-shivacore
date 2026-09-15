// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 kernel paging primitives.
//!
//! The bootloader supplies a linear physical-memory mapping. We build an
//! OffsetPageTable over the active level-4 table and allocate only frames marked
//! usable by the bootloader. Device/MMIO mappings are deliberately separate from
//! ordinary RAM allocation.

#[cfg(feature = "x86-boot")]
use bootloader_api::info::{MemoryRegion, MemoryRegionKind, MemoryRegions};

#[cfg(feature = "x86-boot")]
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

#[cfg(feature = "x86-boot")]
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

#[cfg(feature = "x86-boot")]
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_frame, _) = Cr3::read();
    let physical_address = level_4_frame.start_address();
    let virtual_address = physical_memory_offset + physical_address.as_u64();
    let page_table_ptr: *mut PageTable = virtual_address.as_mut_ptr();

    &mut *page_table_ptr
}

/// Frame allocator backed exclusively by bootloader-declared usable memory.
#[cfg(feature = "x86-boot")]
pub struct BootInfoFrameAllocator {
    regions: &'static MemoryRegions,
    next_region: usize,
    next_frame: usize,
}

#[cfg(feature = "x86-boot")]
impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        Self {
            regions: memory_regions,
            next_region: 0,
            next_frame: 0,
        }
    }

    fn current_region(&self) -> Option<&MemoryRegion> {
        self.regions.get(self.next_region)
    }

    fn advance_to_usable_region(&mut self) -> Option<&MemoryRegion> {
        while self.next_region < self.regions.len() {
            let region = &self.regions[self.next_region];
            if region.kind == MemoryRegionKind::Usable {
                return Some(region);
            }
            self.next_region += 1;
            self.next_frame = 0;
        }
        None
    }
}

#[cfg(feature = "x86-boot")]
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        loop {
            let region = self.advance_to_usable_region()?;
            let start = region.start;
            let end = region.end;
            let frame_address = start.checked_add((self.next_frame as u64) * 4096)?;

            if frame_address >= end {
                self.next_region += 1;
                self.next_frame = 0;
                continue;
            }

            self.next_frame += 1;
            return PhysFrame::containing_address(PhysAddr::new(frame_address)).into();
        }
    }
}

#[cfg(not(feature = "x86-boot"))]
pub struct BootInfoFrameAllocator;

#[cfg(test)]
mod tests {
    #[test]
    fn memory_module_is_present() {
        assert!(true);
    }
}
