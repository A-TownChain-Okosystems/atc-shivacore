// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-stack ownership and lifecycle for x86_64 process contexts.
#![cfg(feature = "x86-boot")]

extern crate alloc;
use alloc::collections::BTreeMap;
use crate::ats1000::Pid;
use crate::memory::BootInfoFrameAllocator;
use crate::x86_64_page_table::{PageTableError, ProcessAddressSpaceManager};
use x86_64::{structures::paging::{Page, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};

pub const PAGE_SIZE: u64 = 4096;
pub const DEFAULT_STACK_PAGES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelStackError { InvalidPid, AlreadyExists, ProcessMissing, InvalidRange, OutOfFrames, Mapping(PageTableError), Unknown, OwnershipConflict, OutstandingMappings }
impl From<PageTableError> for KernelStackError { fn from(value: PageTableError) -> Self { Self::Mapping(value) } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelStack {
    pid: Pid,
    guard_start: u64,
    base: u64,
    top: u64,
    pages: usize,
    frames: [u64; DEFAULT_STACK_PAGES],
}
impl KernelStack {
    pub const fn pid(&self) -> Pid { self.pid }
    pub const fn guard_start(&self) -> u64 { self.guard_start }
    pub const fn base(&self) -> u64 { self.base }
    pub const fn top(&self) -> u64 { self.top }
    pub const fn pages(&self) -> usize { self.pages }
    pub fn contains(&self, address: u64) -> bool { address >= self.base && address < self.top }
    pub fn frame(&self, index: usize) -> Option<PhysFrame> { if index >= self.pages { None } else { Some(PhysFrame::containing_address(PhysAddr::new(self.frames[index]))) } }
}

pub struct KernelStackManager { stacks: BTreeMap<Pid, KernelStack> }
impl KernelStackManager {
    pub const fn new() -> Self { Self { stacks: BTreeMap::new() } }
    pub fn get(&self, pid: Pid) -> Option<&KernelStack> { self.stacks.get(&pid) }
    pub fn contains(&self, pid: Pid) -> bool { self.stacks.contains_key(&pid) }

    /// Creates a four-page kernel-only stack with one unmapped guard page below it.
    /// The stack lives in the canonical kernel half and is never mapped USER-accessible.
    pub unsafe fn allocate(&mut self, pid: Pid, address_spaces: &mut ProcessAddressSpaceManager, frame_allocator: &mut BootInfoFrameAllocator) -> Result<KernelStack, KernelStackError> {
        if pid.0 == 0 { return Err(KernelStackError::InvalidPid); }
        if self.stacks.contains_key(&pid) { return Err(KernelStackError::AlreadyExists); }
        if !address_spaces.contains(pid) { return Err(KernelStackError::ProcessMissing); }
        let base = 0xffff_9000_0000_0000u64.checked_add((pid.0 as u64).checked_mul(0x20_0000).ok_or(KernelStackError::InvalidRange)?).ok_or(KernelStackError::InvalidRange)?;
        let guard_start = base.checked_sub(PAGE_SIZE).ok_or(KernelStackError::InvalidRange)?;
        let top = base.checked_add((DEFAULT_STACK_PAGES as u64) * PAGE_SIZE).ok_or(KernelStackError::InvalidRange)?;
        if base & 0xfff != 0 || top <= base || top > 0xffff_ffff_ffff_f000 { return Err(KernelStackError::InvalidRange); }

        let mut frames = [0u64; DEFAULT_STACK_PAGES];
        let mut allocated: [Option<PhysFrame>; DEFAULT_STACK_PAGES] = [None; DEFAULT_STACK_PAGES];
        for index in 0..DEFAULT_STACK_PAGES {
            let frame = match frame_allocator.allocate_frame() { Some(frame) => frame, None => {
                for rollback in allocated.into_iter().flatten() { let _ = frame_allocator.reclaim_frame(rollback); }
                return Err(KernelStackError::OutOfFrames);
            }};
            allocated[index] = Some(frame);
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(base + (index as u64) * PAGE_SIZE));
            if let Err(error) = address_spaces.get_mut(pid)?.map_kernel_page(page, frame, frame_allocator) {
                for rollback in allocated.into_iter().flatten() { let _ = frame_allocator.reclaim_frame(rollback); }
                return Err(error.into());
            }
            frames[index] = frame.start_address().as_u64();
        }
        let stack = KernelStack { pid, guard_start, base, top, pages: DEFAULT_STACK_PAGES, frames };
        self.stacks.insert(pid, stack);
        Ok(stack)
    }

    /// Removes all stack mappings and returns their physical frames to the allocator.
    pub unsafe fn reclaim(&mut self, pid: Pid, address_spaces: &mut ProcessAddressSpaceManager, frame_allocator: &mut BootInfoFrameAllocator) -> Result<(), KernelStackError> {
        let stack = *self.stacks.get(&pid).ok_or(KernelStackError::Unknown)?;
        let table = address_spaces.get_mut(pid)?;
        let mut frames = [None; DEFAULT_STACK_PAGES];
        for index in 0..DEFAULT_STACK_PAGES {
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(stack.base + (index as u64) * PAGE_SIZE));
            frames[index] = Some(table.unmap_kernel_page(page)?);
        }
        for frame in frames.into_iter().flatten() { if !frame_allocator.reclaim_frame(frame) { return Err(KernelStackError::OwnershipConflict); } }
        self.stacks.remove(&pid);
        Ok(())
    }
}
impl Default for KernelStackManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stack_range_has_unmapped_guard_below_base() {
        let stack = KernelStack { pid: Pid(1), guard_start: 0xffff_8fff_ffff_f000, base: 0xffff_9000_0000_0000, top: 0xffff_9000_0000_4000, pages: 4, frames: [0x1000,0x2000,0x3000,0x4000] };
        assert_eq!(stack.top - stack.base, 4 * PAGE_SIZE);
        assert_eq!(stack.base - stack.guard_start, PAGE_SIZE);
        assert!(!stack.contains(stack.guard_start));
        assert!(stack.contains(stack.base));
        assert!(!stack.contains(stack.top));
    }
}
