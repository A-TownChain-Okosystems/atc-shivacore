// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// ShivaCore — Global Descriptor Table + Task State Segment.

use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::segmentation::{Segment, CS, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
const STACK_SIZE: usize = 4096 * 5;

lazy_static! {
    static ref TSS: Mutex<TaskStateSegment> = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(core::ptr::addr_of!(STACK));
            stack_start + STACK_SIZE as u64
        };
        Mutex::new(tss)
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&*TSS.lock()));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

/// Updates RSP0, the ring-0 stack used by the CPU for CPL3 -> CPL0 transitions.
/// The caller must provide the exclusive, 16-byte-aligned top of a mapped kernel stack.
pub fn set_kernel_stack_top(stack_top: u64) -> Result<(), ()> {
    if stack_top == 0 || stack_top & 0xf != 0 { return Err(()); }
    TSS.lock().privilege_stack_table[0] = VirtAddr::new(stack_top);
    Ok(())
}

pub fn kernel_stack_top() -> u64 {
    TSS.lock().privilege_stack_table[0].as_u64()
}

pub fn init() {
    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
        SS::set_reg(SegmentSelector::NULL);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn stack_top_requires_nonzero_and_alignment() {
        assert_eq!(super::set_kernel_stack_top(0), Err(()));
        assert_eq!(super::set_kernel_stack_top(0x1001), Err(()));
    }
}
