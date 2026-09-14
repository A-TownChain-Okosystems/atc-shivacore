// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// ShivaCore — Interrupt Descriptor Table + PIC-Remapping.

use crate::gdt;
use crate::serial_println;
use crate::x86_64_timer_entry::{current_context, entry_address, no_switch, HardwareContextFrame};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use shivacore::preemption::{InterruptFrame, TIMER_PREEMPTION};
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::VirtAddr;

pub const PIC_1_OFFSET: u8 = 0x20;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 { self as u8 }
    fn as_usize(self) -> usize { usize::from(self.as_u8()) }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            // The timer uses a raw assembly entry because a scheduler-driven
            // context switch must own the complete GPR + iret stack contract.
            idt[InterruptIndex::Timer.as_u8()]
                .set_handler_addr(VirtAddr::new(entry_address()));
        }
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() { IDT.load(); }

pub fn init_pics() {
    unsafe { PICS.lock().initialize(); }
    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    serial_println!("EXCEPTION: PAGE FAULT");
    serial_println!("Accessed Address: {:?}", Cr2::read());
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("{:#?}", stack_frame);
}

/// Raw timer dispatcher called by the assembly entry after all GPRs have been
/// saved. It deliberately does not switch CR3 or mutate scheduler state yet.
/// That commit is the next layer: a scheduler callback may return a validated
/// target ContextStack instead of `no_switch()`.
#[no_mangle]
pub extern "C" fn shivacore_timer_interrupt_dispatch(
    frame: *mut HardwareContextFrame,
) -> *const shivacore::x86_64_context_switch::ContextStack {
    let result = if let Some(context) = current_context(frame) {
        let interrupt_frame = InterruptFrame::new(
            context.iret.rip,
            context.iret.cs,
            context.iret.rflags,
            context.iret.rsp,
            context.iret.ss,
        );

        match TIMER_PREEMPTION.record_frame(interrupt_frame) {
            Ok(()) => {
                TIMER_PREEMPTION.request();
                no_switch()
            }
            Err(_) => no_switch(),
        }
    } else {
        no_switch()
    };

    // EOI is issued before returning to the assembly epilogue. No scheduler
    // lock or CR3 operation is permitted in this IRQ dispatcher.
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
    result
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port: Port<u8> = Port::new(0x60);
    let _scancode: u8 = unsafe { port.read() };
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
