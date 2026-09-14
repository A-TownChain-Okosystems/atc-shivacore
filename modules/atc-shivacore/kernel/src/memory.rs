// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 physical-frame allocator and OffsetPageTable bootstrap.

extern crate alloc;
use alloc::collections::BTreeSet;
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{structures::paging::{mapper::MapToError, FrameAllocator, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    OffsetPageTable::new(active_level_4_table(physical_memory_offset), physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_frame, _) = x86_64::registers::control::Cr3::read();
    let virt = physical_memory_offset + level_4_frame.start_address().as_u64();
    &mut *(virt.as_mut_ptr())
}

/// Tracks frames handed out by the allocator and frames returned for reuse.
/// A frame may only enter the recycle pool after it has first been allocated.
#[derive(Default)]
struct FrameRecyclePool {
    allocated: BTreeSet<u64>,
    recycled: BTreeSet<u64>,
}

impl FrameRecyclePool {
    fn allocated(&self, address: u64) -> bool { self.allocated.contains(&address) }

    fn allocate(&mut self, address: u64) -> bool {
        if !self.recycled.remove(&address) && self.allocated.contains(&address) {
            return false;
        }
        self.allocated.insert(address)
    }

    fn reclaim(&mut self, address: u64) -> bool {
        if !self.allocated.remove(&address) { return false; }
        self.recycled.insert(address)
    }

    fn pop_recycled(&mut self) -> Option<u64> {
        let address = self.recycled.pop_first()?;
        debug_assert!(!self.allocated.contains(&address));
        self.allocated.insert(address);
        Some(address)
    }

    fn recycled_count(&self) -> usize { self.recycled.len() }
}

/// Bootloader-backed allocator with explicit, ownership-aware frame reclamation.
pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
    pool: FrameRecyclePool,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        Self { memory_regions, next: 0, pool: FrameRecyclePool::default() }
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

    /// Returns true only for a frame currently owned by this allocator.
    pub fn can_reclaim(&self, frame: PhysFrame) -> bool {
        self.pool.allocated(frame.start_address().as_u64())
    }

    /// Returns an allocated frame to the allocator. Unknown or already-reclaimed frames fail closed.
    pub fn reclaim_frame(&mut self, frame: PhysFrame) -> bool {
        self.pool.reclaim(frame.start_address().as_u64())
    }

    pub fn recycled_count(&self) -> usize { self.pool.recycled_count() }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(address) = self.pool.pop_recycled() {
            return Some(PhysFrame::containing_address(PhysAddr::new(address)));
        }

        let frame = self.usable_frames().nth(self.next)?;
        self.next = self.next.saturating_add(1);
        let address = frame.start_address().as_u64();
        if !self.pool.allocate(address) { return None; }
        Some(frame)
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
    use super::*;

    #[test]
    fn frame_size_is_four_kib() { assert_eq!(4096usize, 4 * 1024); }

    #[test]
    fn unknown_frame_cannot_be_reclaimed() {
        let mut pool = FrameRecyclePool::default();
        assert!(!pool.reclaim(0x20_0000));
        assert_eq!(pool.recycled_count(), 0);
    }

    #[test]
    fn double_reclaim_is_rejected() {
        let mut pool = FrameRecyclePool::default();
        assert!(pool.allocate(0x21_0000));
        assert!(pool.reclaim(0x21_0000));
        assert!(!pool.reclaim(0x21_0000));
        assert_eq!(pool.recycled_count(), 1);
    }

    #[test]
    fn reclaimed_frame_becomes_reusable() {
        let mut pool = FrameRecyclePool::default();
        assert!(pool.allocate(0x22_0000));
        assert!(pool.reclaim(0x22_0000));
        assert_eq!(pool.pop_recycled(), Some(0x22_0000));
        assert!(pool.allocated(0x22_0000));
        assert_eq!(pool.recycled_count(), 0);
    }

    #[test]
    fn active_frame_cannot_be_reallocated() {
        let mut pool = FrameRecyclePool::default();
        assert!(pool.allocate(0x23_0000));
        assert!(!pool.allocate(0x23_0000));
    }

    #[test]
    fn reclaim_then_reallocate_is_single_use() {
        let mut pool = FrameRecyclePool::default();
        assert!(pool.allocate(0x24_0000));
        assert!(pool.reclaim(0x24_0000));
        assert_eq!(pool.pop_recycled(), Some(0x24_0000));
        assert!(!pool.reclaim(0x24_0000));
        assert!(pool.reclaim(0x24_0000));
    }
}
