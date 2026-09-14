// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Validated user-page mapping policy for x86_64.

#![cfg(feature = "x86-boot")]

use x86_64::structures::paging::{mapper::MapToError, Page, PageTableFlags, PhysFrame, Size4KiB};

use crate::memory::BootInfoFrameAllocator;
use crate::memory_isolation::{Mapping, PageFlags, PAGE_SIZE, USER_BASE, USER_LIMIT};
use crate::x86_64_paging::to_page_table_flags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserMappingError {
    InvalidVirtualAddress,
    InvalidSize,
    InvalidFlags,
    NonUserPage,
    HardwareMapping,
}

pub fn validate_user_page(page: Page<Size4KiB>, flags: PageFlags) -> Result<PageTableFlags, UserMappingError> {
    let start = page.start_address().as_u64();
    if start < USER_BASE || start >= USER_LIMIT {
        return Err(UserMappingError::NonUserPage);
    }
    if !flags.contains(PageFlags::USER) || !flags.contains(PageFlags::READ) {
        return Err(UserMappingError::InvalidFlags);
    }
    to_page_table_flags(flags).ok_or(UserMappingError::InvalidFlags)
}

pub fn mapping_for_page(page: Page<Size4KiB>, flags: PageFlags) -> Result<Mapping, UserMappingError> {
    validate_user_page(page, flags)?;
    Ok(Mapping { start: page.start_address().as_u64(), size: PAGE_SIZE, flags })
}

/// Installs one user mapping into an already-created process mapper.
///
/// The physical frame must have been allocated/owned by the kernel's frame
/// allocator. This function does not accept an arbitrary physical address.
pub unsafe fn map_user_page(
    mapper: &mut x86_64::structures::paging::OffsetPageTable<'static>,
    page: Page<Size4KiB>,
    frame: PhysFrame,
    flags: PageFlags,
    frame_allocator: &mut BootInfoFrameAllocator,
) -> Result<(), UserMappingError> {
    let hardware_flags = validate_user_page(page, flags)?;
    mapper
        .map_to(page, frame, hardware_flags, frame_allocator)
        .map_err(|_| UserMappingError::HardwareMapping)?
        .flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use x86_64::{structures::paging::{Page, Size4KiB}, VirtAddr};

    #[test]
    fn user_page_requires_user_and_read() {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(USER_BASE));
        assert_eq!(validate_user_page(page, PageFlags::USER), Err(UserMappingError::InvalidFlags));
    }

    #[test]
    fn kernel_page_is_rejected() {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(0xffff_8000_0000_0000));
        assert_eq!(validate_user_page(page, PageFlags::READ.union(PageFlags::USER)), Err(UserMappingError::NonUserPage));
    }

    #[test]
    fn read_only_user_page_is_valid() {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(USER_BASE));
        assert!(validate_user_page(page, PageFlags::READ.union(PageFlags::USER)).is_ok());
    }
}
