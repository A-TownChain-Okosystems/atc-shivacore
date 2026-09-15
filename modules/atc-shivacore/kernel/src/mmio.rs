// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Physical MMIO mapping for kernel-owned device regions.
//!
//! This layer deliberately does not expose arbitrary physical-memory access to
//! services. A caller must explicitly provide a physical base and length, and the
//! mapping is created with cache-disabled page attributes. The returned region
//! performs volatile accesses only.

#![cfg(feature = "x86-boot")]

use x86_64::{
    structures::paging::{mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

/// Reserved kernel virtual window for device mappings.
///
/// This address is intentionally outside the heap window used by the kernel.
pub const MMIO_WINDOW_START: u64 = 0xFFFF_9000_0000_0000;

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

    /// Perform a volatile 32-bit MMIO read.
    pub unsafe fn read_u32(&self, offset: usize) -> Result<u32, MmioMapError> {
        let ptr = self.checked_address(offset, core::mem::size_of::<u32>())? as *const u32;
        Ok(core::ptr::read_volatile(ptr))
    }

    /// Perform a volatile 32-bit MMIO write.
    pub unsafe fn write_u32(&self, offset: usize, value: u32) -> Result<(), MmioMapError> {
        let ptr = self.checked_address(offset, core::mem::size_of::<u32>())?;
        core::ptr::write_volatile(ptr as *mut u32, value);
        Ok(())
    }

    /// Perform a volatile byte read.
    pub unsafe fn read_bytes(&self, offset: usize, out: &mut [u8]) -> Result<(), MmioMapError> {
        let ptr = self.checked_address(offset, out.len())? as *const u8;
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = core::ptr::read_volatile(ptr.add(index));
        }
        Ok(())
    }

    /// Perform a volatile byte write.
    pub unsafe fn write_bytes(&self, offset: usize, data: &[u8]) -> Result<(), MmioMapError> {
        let ptr = self.checked_address(offset, data.len())?;
        for (index, byte) in data.iter().enumerate() {
            core::ptr::write_volatile(ptr.add(index), *byte);
        }
        Ok(())
    }
}

/// Map a physical device region into the reserved kernel MMIO window.
///
/// `physical_start` may be unaligned; the returned virtual address preserves the
/// original byte offset. Physical frames are mapped directly and are never obtained
/// from the ordinary RAM frame allocator.
pub fn map_mmio<M: Mapper<Size4KiB>, F: FrameAllocator<Size4KiB>>(
    mapper: &mut M,
    _frame_allocator: &mut F,
    physical_start: PhysAddr,
    length: usize,
) -> Result<MmioRegion, MmioMapError> {
    if length == 0 { return Err(MmioMapError::ZeroLength); }

    let page_offset = (physical_start.as_u64() & 0xFFF) as usize;
    let mapped_len = page_offset.checked_add(length).ok_or(MmioMapError::AddressOverflow)?;
    let page_count = mapped_len.checked_add(0xFFF).ok_or(MmioMapError::AddressOverflow)? / 0x1000;
    let bytes = page_count.checked_mul(0x1000).ok_or(MmioMapError::AddressOverflow)?;
    let virt_base = VirtAddr::new(MMIO_WINDOW_START);
    let virt_end = virt_base.as_u64().checked_add(bytes as u64).ok_or(MmioMapError::VirtualWindowExhausted)?;
    if virt_end > 0xFFFF_A000_0000_0000 { return Err(MmioMapError::VirtualWindowExhausted); }

    let phys_base = PhysAddr::new(physical_start.as_u64() & !0xFFF);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    for index in 0..page_count {
        let virt = Page::<Size4KiB>::containing_address(virt_base + (index * 0x1000) as u64);
        let phys = PhysFrame::<Size4KiB>::containing_address(phys_base + (index * 0x1000) as u64);
        let result = unsafe { mapper.map_to(virt, phys, flags, _frame_allocator) };
        if result.is_err() { return Err(MmioMapError::MapFailure); }
        unsafe { result.unwrap_unchecked().flush(); }
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
    }
}
