// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! x86_64 XSAVE/XRSTOR process floating-point and SIMD state.

#![cfg(feature = "x86-boot")]

extern crate alloc;

use alloc::alloc::{alloc_zeroed, dealloc, Layout};
use core::arch::{asm, global_asm};
use core::ptr::NonNull;

const CPUID_XSAVE: u32 = 1 << 26;
const CPUID_OSXSAVE: u32 = 1 << 27;
const XCR0_X87: u64 = 1 << 0;
const XCR0_SSE: u64 = 1 << 1;
const MAX_XSAVE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XsaveError {
    Unsupported,
    InvalidAreaSize,
    AllocationFailed,
    InvalidMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XsaveConfig {
    area_size: usize,
    mask: u64,
}

impl XsaveConfig {
    pub fn detect() -> Result<Self, XsaveError> {
        let leaf1 = unsafe { core::arch::x86_64::__cpuid(1) };
        if leaf1.ecx & CPUID_XSAVE == 0 || leaf1.ecx & CPUID_OSXSAVE == 0 {
            return Err(XsaveError::Unsupported);
        }

        let xcr0 = read_xcr0();
        if xcr0 & (XCR0_X87 | XCR0_SSE) != (XCR0_X87 | XCR0_SSE) {
            return Err(XsaveError::InvalidMask);
        }

        let leaf0d = unsafe { core::arch::x86_64::__cpuid_count(0xD, 0) };
        let area_size = leaf0d.ebx as usize;
        if area_size < 512 || area_size > MAX_XSAVE_BYTES || area_size % 64 != 0 {
            return Err(XsaveError::InvalidAreaSize);
        }

        Ok(Self { area_size, mask: xcr0 })
    }

    pub const fn area_size(self) -> usize { self.area_size }
    pub const fn mask(self) -> u64 { self.mask }
}

pub struct XsaveState {
    ptr: NonNull<u8>,
    layout: Layout,
    mask: u64,
}

impl XsaveState {
    pub fn new(config: XsaveConfig) -> Result<Self, XsaveError> {
        let layout = Layout::from_size_align(config.area_size, 64)
            .map_err(|_| XsaveError::InvalidAreaSize)?;
        let ptr = unsafe { alloc_zeroed(layout) };
        let ptr = NonNull::new(ptr).ok_or(XsaveError::AllocationFailed)?;
        Ok(Self { ptr, layout, mask: config.mask })
    }

    pub fn capture(&mut self) {
        unsafe { shivacore_xsave(self.ptr.as_ptr() as u64, self.mask); }
    }

    pub fn restore(&self) {
        unsafe { shivacore_xrstor(self.ptr.as_ptr() as u64, self.mask); }
    }

    pub const fn mask(&self) -> u64 { self.mask }
}

impl Drop for XsaveState {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr.as_ptr(), self.layout); }
    }
}

global_asm!(r#"
    .global shivacore_xsave
    .type shivacore_xsave,@function
shivacore_xsave:
    mov rax, rsi
    mov rdx, rsi
    shr rdx, 32
    xsave64 [rdi]
    ret

    .global shivacore_xrstor
    .type shivacore_xrstor,@function
shivacore_xrstor:
    mov rax, rsi
    mov rdx, rsi
    shr rdx, 32
    xrstor64 [rdi]
    ret
"#);

extern "C" {
    fn shivacore_xsave(ptr: u64, mask: u64);
    fn shivacore_xrstor(ptr: u64, mask: u64);
}

#[inline]
fn read_xcr0() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!("xgetbv", in("ecx") 0u32, out("eax") low, out("edx") high, options(nostack));
    }
    ((high as u64) << 32) | low as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_xcr0_bits_are_defined() {
        assert_eq!(XCR0_X87 | XCR0_SSE, 0x3);
    }

    #[test]
    fn aligned_layout_is_valid() {
        assert!(Layout::from_size_align(512, 64).is_ok());
    }
}
