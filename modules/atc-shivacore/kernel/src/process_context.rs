// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Kernel-owned process execution context for x86_64 user-mode activation.

#![cfg(feature = "x86-boot")]

use crate::ats1000::Pid;
use crate::x86_64_address_space::PageTableRoot;
use crate::x86_64_context_switch::{ContextStack, IretFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessContextError {
    InvalidPid,
    RootPidMismatch,
    NullContextStack,
    UnalignedContextStack,
    InvalidKernelStack,
    ContextStack(crate::x86_64_context_switch::ContextSwitchError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelStack {
    pub base: u64,
    pub top: u64,
}

impl KernelStack {
    pub const fn new(base: u64, top: u64) -> Self { Self { base, top } }
    pub fn validate(&self) -> Result<(), ProcessContextError> {
        if self.base == 0 || self.top <= self.base || self.top - self.base < 4096 || self.top & 0xf != 0 {
            return Err(ProcessContextError::InvalidKernelStack);
        }
        Ok(())
    }
    pub fn contains(&self, address: u64) -> bool { address >= self.base && address < self.top }
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessExecutionContext {
    pid: Pid,
    root: PageTableRoot,
    kernel_stack: KernelStack,
    context_stack: *const ContextStack,
}

impl ProcessExecutionContext {
    pub unsafe fn new(pid: Pid, root: PageTableRoot, kernel_stack: KernelStack, context_stack: *const ContextStack) -> Result<Self, ProcessContextError> {
        if pid.0 == 0 { return Err(ProcessContextError::InvalidPid); }
        if root.pid() != pid { return Err(ProcessContextError::RootPidMismatch); }
        kernel_stack.validate()?;
        if context_stack.is_null() { return Err(ProcessContextError::NullContextStack); }
        let address = context_stack as u64;
        if address & 0xf != 0 || !kernel_stack.contains(address) { return Err(ProcessContextError::UnalignedContextStack); }
        (*context_stack).validate().map_err(ProcessContextError::ContextStack)?;
        Ok(Self { pid, root, kernel_stack, context_stack })
    }
    pub const fn pid(&self) -> Pid { self.pid }
    pub const fn root(&self) -> PageTableRoot { self.root }
    pub const fn kernel_stack(&self) -> KernelStack { self.kernel_stack }
    pub const fn context_stack(&self) -> *const ContextStack { self.context_stack }

    /// The trampoline supplies the complete five-word `iretq` frame; therefore
    /// this activation path is explicitly restricted to CPL3 returns.
    pub unsafe fn validate_user_return(&self) -> Result<(), ProcessContextError> {
        let frame: &IretFrame = &(*self.context_stack).iret;
        if frame.cs & 0x3 != 0x3 || frame.ss & 0x3 != 0x3 {
            return Err(ProcessContextError::ContextStack(crate::x86_64_context_switch::ContextSwitchError::InvalidSegment));
        }
        if frame.rip < 0x0000_0001_0000_0000 || frame.rip >= 0x0000_8000_0000_0000 {
            return Err(ProcessContextError::ContextStack(crate::x86_64_context_switch::ContextSwitchError::InvalidRip));
        }
        if frame.rsp < 0x0000_0001_0000_0000 || frame.rsp >= 0x0000_8000_0000_0000 {
            return Err(ProcessContextError::ContextStack(crate::x86_64_context_switch::ContextSwitchError::InvalidRsp));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x86_64_context_switch::SavedRegisters;
    fn context() -> ContextStack { ContextStack { registers: SavedRegisters { r15:0,r14:0,r13:0,r12:0,r11:0,r10:0,r9:0,r8:0,rsi:0,rdi:0,rbp:0,rdx:0,rcx:0,rbx:0,rax:0 }, iret: IretFrame { rip:0x0000_0001_4000_1000, cs:0x1b, rflags:0x202, rsp:0x0000_0001_8000_1000, ss:0x23 } } }
    #[test] fn kernel_stack_requires_alignment_and_size() { assert!(KernelStack::new(0x1000,0x2000).validate().is_ok()); assert_eq!(KernelStack::new(0x1001,0x2000).validate(),Err(ProcessContextError::InvalidKernelStack)); }
    #[test] fn context_root_must_belong_to_process() { let frame=context(); let root=PageTableRoot::new(Pid(2),0x2000).unwrap(); let stack=KernelStack::new((&frame as *const _) as u64-0x1000,(&frame as *const _) as u64+0x2000); let result=unsafe{ProcessExecutionContext::new(Pid(1),root,stack,&frame)}; assert_eq!(result,Err(ProcessContextError::RootPidMismatch)); }
}
