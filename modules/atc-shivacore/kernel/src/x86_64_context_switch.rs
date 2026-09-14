// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 interrupt-return context switch boundary.
//!
//! This module defines the exact stack contract for a future scheduler-driven
//! context switch. The trampoline is deliberately small: it restores the
//! general-purpose registers from a kernel-owned context stack and finishes
//! with `iretq`, so the CPU returns through the architectural interrupt frame.
//!
//! The scheduler must construct and validate a complete ContextStack before
//! invoking the trampoline. No page-table or scheduler metadata is mutated by
//! the assembly itself.

#![cfg(feature = "x86-boot")]

use core::arch::{asm, global_asm};

/// Saved register set immediately below an interrupt-return frame.
///
/// Stack order is part of the ABI and must remain synchronized with
/// `context_switch_trampoline`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedRegisters {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
}

/// Architectural interrupt-return frame used by `iretq`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IretFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Complete target stack contract.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextStack {
    pub registers: SavedRegisters,
    pub iret: IretFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitchError {
    NullStack,
    UnalignedStack,
    InvalidRip,
    InvalidRsp,
    InvalidFlags,
    InvalidSegment,
}

impl ContextStack {
    /// Validates the complete target context before entering assembly.
    pub fn validate(&self) -> Result<(), ContextSwitchError> {
        if self.iret.rip == 0 {
            return Err(ContextSwitchError::InvalidRip);
        }
        if self.iret.rsp == 0 {
            return Err(ContextSwitchError::InvalidRsp);
        }
        // RFLAGS bit 1 is architecturally fixed to one. Rejecting a malformed
        // frame here prevents an invalid return state from reaching iretq.
        if self.iret.rflags & 0x2 == 0 {
            return Err(ContextSwitchError::InvalidFlags);
        }
        if self.iret.cs == 0 || self.iret.ss == 0 {
            return Err(ContextSwitchError::InvalidSegment);
        }
        Ok(())
    }
}

/// Switches to a kernel-owned, already-validated context stack.
///
/// # Safety
/// `stack` must point to a live `ContextStack` whose memory remains valid until
/// the CPU has completed `iretq`. The target CR3 must already be selected by
/// the scheduler/address-space layer before entering this trampoline.
pub unsafe fn switch_to_context(stack: *const ContextStack) -> ! {
    if stack.is_null() {
        panic!("ShivaCore: null context stack");
    }
    let stack_address = stack as usize;
    if stack_address & 0xf != 0 {
        panic!("ShivaCore: unaligned context stack");
    }
    if let Err(error) = (*stack).validate() {
        panic!("ShivaCore: invalid context stack: {:?}", error);
    }
    context_switch_trampoline(stack as u64)
}

/// Assembly entry point. The target pointer is passed in RDI using the SysV
/// calling convention; the trampoline never returns to Rust.
global_asm!(r#"
    .global context_switch_trampoline
    .type context_switch_trampoline,@function
context_switch_trampoline:
    mov rsp, rdi

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

extern "C" {
    fn context_switch_trampoline(stack: u64) -> !;
}

/// Prevent accidental compiler assumptions about a normal Rust return path.
#[inline(never)]
pub unsafe fn architecture_barrier() {
    asm!("", options(nomem, nostack, preserves_flags));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ContextStack {
        ContextStack {
            registers: SavedRegisters {
                r15: 15,
                r14: 14,
                r13: 13,
                r12: 12,
                r11: 11,
                r10: 10,
                r9: 9,
                r8: 8,
                rsi: 6,
                rdi: 7,
                rbp: 5,
                rdx: 2,
                rcx: 1,
                rbx: 3,
                rax: 0,
            },
            iret: IretFrame {
                rip: 0x4000,
                cs: 0x1b,
                rflags: 0x202,
                rsp: 0x8000,
                ss: 0x23,
            },
        }
    }

    #[test]
    fn valid_context_is_accepted() {
        assert_eq!(context().validate(), Ok(()));
    }

    #[test]
    fn zero_rip_is_rejected() {
        let mut context = context();
        context.iret.rip = 0;
        assert_eq!(context.validate(), Err(ContextSwitchError::InvalidRip));
    }

    #[test]
    fn zero_rsp_is_rejected() {
        let mut context = context();
        context.iret.rsp = 0;
        assert_eq!(context.validate(), Err(ContextSwitchError::InvalidRsp));
    }

    #[test]
    fn malformed_rflags_are_rejected() {
        let mut context = context();
        context.iret.rflags = 0x200;
        assert_eq!(context.validate(), Err(ContextSwitchError::InvalidFlags));
    }
}
