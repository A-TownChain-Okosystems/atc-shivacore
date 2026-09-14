// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Low-level x86_64 timer-interrupt context contract.

#![cfg(feature = "x86-boot")]

use core::arch::global_asm;
use core::ptr;

use crate::x86_64_context_switch::{ContextStack, IretFrame, SavedRegisters};

/// Exact stack layout produced by `timer_interrupt_entry`.
/// CPU iret state is followed by the saved general-purpose registers in the
/// in-memory layout expected by `ContextStack`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareContextFrame {
    pub registers: SavedRegisters,
    pub iret: IretFrame,
}

impl HardwareContextFrame {
    pub const fn as_context(&self) -> &ContextStack {
        // SAFETY: both types have identical repr(C) field order and types.
        unsafe { &*(self as *const Self as *const ContextStack) }
    }

    pub const fn as_context_mut(&mut self) -> &mut ContextStack {
        // SAFETY: both types have identical repr(C) field order and types.
        unsafe { &mut *(self as *mut Self as *mut ContextStack) }
    }

    pub const fn pointer_is_valid(pointer: *const Self) -> bool {
        !pointer.is_null()
    }
}

/// Returns the address installed into the timer IDT entry.
pub fn entry_address() -> u64 {
    timer_interrupt_entry as usize as u64
}

/// Converts the assembly-owned frame into the existing context-switch ABI.
pub fn current_context<'a>(frame: *mut HardwareContextFrame) -> Option<&'a mut ContextStack> {
    if frame.is_null() {
        return None;
    }
    // SAFETY: valid only during the dispatcher call from the assembly entry.
    Some(unsafe { (*frame).as_context_mut() })
}

/// Null target means resume the interrupted context.
pub const fn no_switch() -> *const ContextStack {
    ptr::null()
}

global_asm!(r#"
    .global shivacore_timer_interrupt_entry
    .type shivacore_timer_interrupt_entry,@function
shivacore_timer_interrupt_entry:
    // CPU already pushed the iret frame. Push GPRs so memory is
    // SavedRegisters { r15..rax } followed by IretFrame.
    push rax
    push rbx
    push rcx
    push rdx
    push rbp
    push rdi
    push rsi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    // Preserve the current frame pointer across the Rust call.
    mov rbx, rsp
    mov rdi, rsp
    sub rsp, 8
    call shivacore_timer_interrupt_dispatch
    add rsp, 8

    // RAX is either null (resume current context) or a target ContextStack.
    test rax, rax
    jz 1f
    mov rsp, rax
    jmp 2f

1:
    mov rsp, rbx

2:
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rsi
    pop rdi
    pop rbp
    pop rdx
    pop rcx
    pop rbx
    pop rax
    iretq
"#);

unsafe extern "C" {
    #[link_name = "shivacore_timer_interrupt_entry"]
    pub fn timer_interrupt_entry();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_context_matches_context_stack_layout() {
        assert_eq!(core::mem::size_of::<HardwareContextFrame>(), core::mem::size_of::<ContextStack>());
        assert_eq!(core::mem::align_of::<HardwareContextFrame>(), core::mem::align_of::<ContextStack>());
    }

    #[test]
    fn null_pointer_is_rejected() {
        assert!(!HardwareContextFrame::pointer_is_valid(ptr::null()));
        assert!(current_context(ptr::null_mut()).is_none());
    }

    #[test]
    fn no_switch_returns_null() {
        assert!(no_switch().is_null());
    }
}
