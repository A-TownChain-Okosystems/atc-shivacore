// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 hardware paging backend for the memory-isolation contract.
//!
//! This module is intentionally small: it translates the architecture-neutral
//! isolation policy into x86_64 page-table flags. Physical-frame allocation and
//! page-table installation remain explicit integration points for the boot
//! layer; this prevents the policy layer from gaining unsafe ambient access.

#![cfg(feature = "x86-boot")]

use x86_64::structures::paging::PageTableFlags;
use crate::memory_isolation::PageFlags;

pub fn to_page_table_flags(flags: PageFlags) -> Option<PageTableFlags> {
    if !flags.contains(PageFlags::USER) || !flags.contains(PageFlags::READ) {
        return None;
    }

    let mut result = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags.contains(PageFlags::WRITE) {
        result |= PageTableFlags::WRITABLE;
    }
    if !flags.contains(PageFlags::EXECUTE) {
        result |= PageTableFlags::NO_EXECUTE;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_read_mapping_is_present_and_user_accessible() {
        let flags = PageFlags::READ.union(PageFlags::USER);
        let translated = to_page_table_flags(flags).unwrap();
        assert!(translated.contains(PageTableFlags::PRESENT));
        assert!(translated.contains(PageTableFlags::USER_ACCESSIBLE));
        assert!(translated.contains(PageTableFlags::NO_EXECUTE));
        assert!(!translated.contains(PageTableFlags::WRITABLE));
    }

    #[test]
    fn writable_mapping_gets_write_permission() {
        let flags = PageFlags::READ.union(PageFlags::WRITE).union(PageFlags::USER);
        let translated = to_page_table_flags(flags).unwrap();
        assert!(translated.contains(PageTableFlags::WRITABLE));
    }

    #[test]
    fn executable_mapping_does_not_get_nx() {
        let flags = PageFlags::READ.union(PageFlags::EXECUTE).union(PageFlags::USER);
        let translated = to_page_table_flags(flags).unwrap();
        assert!(!translated.contains(PageTableFlags::NO_EXECUTE));
    }

    #[test]
    fn kernel_only_flags_are_rejected() {
        assert!(to_page_table_flags(PageFlags::READ).is_none());
    }
}
