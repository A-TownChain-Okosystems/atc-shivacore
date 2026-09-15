// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Physical MMIO mapping for kernel-owned device regions.
//!
//! This layer deliberately does not expose arbitrary physical-memory access to
//! services. A caller must explicitly provide a physical base and length, and the
//! mapping is created with cache-disabled page attributes. The returned region
//! performs volatile accesses only.

#![cfg(feature = "x86-boot")]

use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

/// Reserved kernel virtual window for device mappings.
pub const MMIO_WINDOW_START: u64 = 0xFFFF_9000_0000_0000;
pub const MMIO_WINDOW_END: u64 = 0xFFFF_A000_0000_0000;
const PAGE_SIZE: u64 = 0x1000;
static NEXT_MMIO_VIRT: AtomicU64 = AtomicU64::new(MMIO_WINDOW_START);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioMapError {
    ZeroLength,
    AddressOverflow,
    VirtualWindowExhausted,
    MapFailure,
}

pub struct MmioRegion {
    virtual_start: VirtAddr,
    physical_start: PhysAddr,
    length: usize,
}

impl MmioRegion {
    pub fn virtual_start(&self) -> VirtAddr { self.virtual_start }
    pub fn physical_start(&self) -> PhysAddr { self.physical_start }
    pub fn len(&self) -> usize { self.length }

    fn checked_address(&self, offset: usize, width: usize) -> Result<*mut u8, MmioMapError> {
        let end = offset.checked_add(width).ok_or(MmioMapError::AddressOverflow)?;
        if end > self.length { return Err(MmioMapError::AddressOverflow); }
        Ok((self.virtual_start + offset as u64).as_mut_ptr())
    }

    pub unsafe fn read_u32(&self, offset: usize) -> Result<u32, MmioMapError> {
        let ptr = self.checked_address(offset, core::mem::size_of::<u32>())? as *const u32;
        Ok(core::ptr::read_volatile(ptr))
    }

    pub unsafe fn write_u32(&self, offset: usize, value: u32) -> Result<(), MmioMapError> {
        let ptr = self.checked_address(offset, core::mem::size_of::<u32>())?;
        core::ptr::write_volatile(ptr as *mut u32, value);
        Ok(())
    }

    pub unsafe fn read_bytes(&self, offset: usize, out: &mut [u8]) -> Result<(), MmioMapError> {
        let ptr = self.checked_address(offset, out.len())? as *const u8;
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = core::ptr::read_volatile(ptr.add(index));
        }
        Ok(())
    }

    pub unsafe fn write_bytes(&self, offset: usize, data: &[u8]) -> Result<(), MmioMapError> {
        let ptr = self.checked_address(offset, data.len())?;
        for (index, byte) in data.iter().enumerate() {
            core::ptr::write_volatile(ptr.add(index), *byte);
        }
        Ok(())
    }
}

fn reserve_virtual_window(bytes: u64) -> Result<VirtAddr, MmioMapError> {
    loop {
        let current = NEXT_MMIO_VIRT.load(Ordering::Relaxed);
        let next = current.checked_add(bytes).ok_or(MmioMapError::VirtualWindowExhausted)?;
        if next > MMIO_WINDOW_END { return Err(MmioMapError::VirtualWindowExhausted); }
        if NEXT_MMIO_VIRT.compare_exchange(current, next, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
            return Ok(VirtAddr::new(current));
        }
    }
}

/// Map a physical device region into the reserved kernel MMIO window.
pub fn map_mmio<M: Mapper<Size4KiB>, F: FrameAllocator<Size4KiB>>(
    mapper: &mut M,
    frame_allocator: &mut F,
    physical_start: PhysAddr,
    length: usize,
) -> Result<MmioRegion, MmioMapError> {
    if length == 0 { return Err(MmioMapError::ZeroLength); }

    let page_offset = (physical_start.as_u64() & 0xFFF) as usize;
    let mapped_len = page_offset.checked_add(length).ok_or(MmioMapError::AddressOverflow)?;
    let page_count = mapped_len.checked_add(0xFFF).ok_or(MmioMapError::AddressOverflow)? / 0x1000;
    let bytes = (page_count as u64).checked_mul(PAGE_SIZE).ok_or(MmioMapError::AddressOverflow)?;
    let virt_base = reserve_virtual_window(bytes)?;
    let phys_base = PhysAddr::new(physical_start.as_u64() & !0xFFF);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    for index in 0..page_count {
        let virt = Page::<Size4KiB>::containing_address(virt_base + (index as u64) * PAGE_SIZE);
        let phys = PhysFrame::<Size4KiB>::containing_address(phys_base + (index as u64) * PAGE_SIZE);
        let result = unsafe { mapper.map_to(virt, phys, flags, frame_allocator) };
        match result {
            Ok(flush) => unsafe { flush.flush() },
            Err(_) => return Err(MmioMapError::MapFailure),
        }
    }

    Ok(MmioRegion {
        virtual_start: virt_base + page_offset as u64,
        physical_start,
        length,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn mmio_window_is_canonical() {
        assert_eq!(super::MMIO_WINDOW_START >> 48, 0xFFFF);
        assert!(super::MMIO_WINDOW_START < super::MMIO_WINDOW_END);
    }
}
