// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Interrupt-driven preemption boundary.
//!
//! The timer interrupt never performs a CR3 switch directly. It records a
//! preemption request and captures the architectural return frame. A context
//! switch may only be committed at an explicit kernel-safe preemption point.

#![cfg(feature = "x86-boot")]

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// Architectural state required to resume an interrupted execution context.
///
/// This is intentionally a value type owned by the kernel rather than a raw
/// pointer into the interrupted stack. General-purpose register saving remains
/// the responsibility of the eventual low-level context-switch trampoline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

impl InterruptFrame {
    pub const fn new(
        instruction_pointer: u64,
        code_segment: u64,
        cpu_flags: u64,
        stack_pointer: u64,
        stack_segment: u64,
    ) -> Self {
        Self {
            instruction_pointer,
            code_segment,
            cpu_flags,
            stack_pointer,
            stack_segment,
        }
    }

    pub const fn is_canonical(&self) -> bool {
        let upper = self.instruction_pointer >> 48;
        upper == 0 || upper == 0xffff
    }
}

/// Result of evaluating a timer interrupt at a kernel-safe preemption point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptionAction {
    Continue,
    Reschedule,
}

/// Tracks interrupt nesting, deferred timer preemption, and the most recent
/// validated architectural return frame.
///
/// The frame is stored with a small sequence lock so the IRQ path never takes
/// a spinlock that could already be held by interrupted kernel code.
pub struct PreemptionController {
    pending: AtomicBool,
    critical_depth: AtomicUsize,
    frame_sequence: AtomicUsize,
    frame_instruction_pointer: AtomicU64,
    frame_code_segment: AtomicU64,
    frame_cpu_flags: AtomicU64,
    frame_stack_pointer: AtomicU64,
    frame_stack_segment: AtomicU64,
}

impl PreemptionController {
    pub const fn new() -> Self {
        Self {
            pending: AtomicBool::new(false),
            critical_depth: AtomicUsize::new(0),
            frame_sequence: AtomicUsize::new(0),
            frame_instruction_pointer: AtomicU64::new(0),
            frame_code_segment: AtomicU64::new(0),
            frame_cpu_flags: AtomicU64::new(0),
            frame_stack_pointer: AtomicU64::new(0),
            frame_stack_segment: AtomicU64::new(0),
        }
    }

    /// Records a timer-driven preemption request. This operation is IRQ-safe.
    pub fn request(&self) {
        self.pending.store(true, Ordering::Release);
    }

    pub fn pending(&self) -> bool {
        self.pending.load(Ordering::Acquire)
    }

    /// Stores the validated architectural return frame without taking a lock.
    pub fn record_frame(&self, frame: InterruptFrame) -> Result<(), PreemptionError> {
        validate_interrupt_frame(&frame)?;
        self.frame_sequence.fetch_add(1, Ordering::AcqRel); // odd = write in progress
        self.frame_instruction_pointer
            .store(frame.instruction_pointer, Ordering::Relaxed);
        self.frame_code_segment
            .store(frame.code_segment, Ordering::Relaxed);
        self.frame_cpu_flags.store(frame.cpu_flags, Ordering::Relaxed);
        self.frame_stack_pointer
            .store(frame.stack_pointer, Ordering::Relaxed);
        self.frame_stack_segment
            .store(frame.stack_segment, Ordering::Relaxed);
        self.frame_sequence.fetch_add(1, Ordering::Release); // even = committed
        Ok(())
    }

    /// Returns the last complete frame, or None before the first interrupt.
    pub fn last_frame(&self) -> Option<InterruptFrame> {
        for _ in 0..4 {
            let before = self.frame_sequence.load(Ordering::Acquire);
            if before == 0 || before & 1 != 0 {
                continue;
            }
            let frame = InterruptFrame::new(
                self.frame_instruction_pointer.load(Ordering::Relaxed),
                self.frame_code_segment.load(Ordering::Relaxed),
                self.frame_cpu_flags.load(Ordering::Relaxed),
                self.frame_stack_pointer.load(Ordering::Relaxed),
                self.frame_stack_segment.load(Ordering::Relaxed),
            );
            let after = self.frame_sequence.load(Ordering::Acquire);
            if before == after {
                return Some(frame);
            }
        }
        None
    }

    /// Enters a non-preemptible kernel critical section.
    pub fn enter_critical(&self) {
        self.critical_depth.fetch_add(1, Ordering::AcqRel);
    }

    /// Leaves a critical section. Underflow is rejected rather than wrapped.
    pub fn exit_critical(&self) -> Result<(), PreemptionError> {
        let mut current = self.critical_depth.load(Ordering::Acquire);
        loop {
            if current == 0 {
                return Err(PreemptionError::CriticalSectionUnderflow);
            }
            match self.critical_depth.compare_exchange_weak(
                current,
                current - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(observed) => current = observed,
            }
        }
    }

    pub fn critical_depth(&self) -> usize {
        self.critical_depth.load(Ordering::Acquire)
    }

    /// Determines whether the scheduler may consume the pending request.
    pub fn preemption_point(&self) -> PreemptionAction {
        if self.pending() && self.critical_depth() == 0 {
            PreemptionAction::Reschedule
        } else {
            PreemptionAction::Continue
        }
    }

    /// Atomically consumes a pending request at a safe point.
    pub fn take_if_safe(&self) -> bool {
        if self.critical_depth() != 0 {
            return false;
        }
        self.pending.swap(false, Ordering::AcqRel)
    }
}

impl Default for PreemptionController {
    fn default() -> Self {
        Self::new()
    }
}

/// Global timer-preemption state shared by the architecture interrupt path and
/// the scheduler safe-point path. It contains atomics only and is therefore
/// safe to touch from an interrupt handler without acquiring a kernel lock.
pub static TIMER_PREEMPTION: PreemptionController = PreemptionController::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptionError {
    CriticalSectionUnderflow,
    InvalidInterruptFrame,
}

/// Validates the architectural return frame before it can become scheduler
/// state. This is deliberately conservative: a malformed frame fails closed.
pub fn validate_interrupt_frame(frame: &InterruptFrame) -> Result<(), PreemptionError> {
    if !frame.is_canonical() || frame.instruction_pointer == 0 || frame.stack_pointer == 0 {
        return Err(PreemptionError::InvalidInterruptFrame);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_request_is_deferred_inside_critical_section() {
        let controller = PreemptionController::new();
        controller.enter_critical();
        controller.request();
        assert_eq!(controller.preemption_point(), PreemptionAction::Continue);
        assert!(controller.pending());
        controller.exit_critical().unwrap();
        assert_eq!(controller.preemption_point(), PreemptionAction::Reschedule);
        assert!(controller.take_if_safe());
        assert!(!controller.pending());
    }

    #[test]
    fn critical_section_underflow_is_rejected() {
        let controller = PreemptionController::new();
        assert_eq!(
            controller.exit_critical(),
            Err(PreemptionError::CriticalSectionUnderflow)
        );
        assert_eq!(controller.critical_depth(), 0);
    }

    #[test]
    fn nested_critical_sections_block_preemption_until_outer_exit() {
        let controller = PreemptionController::new();
        controller.enter_critical();
        controller.enter_critical();
        controller.request();
        controller.exit_critical().unwrap();
        assert_eq!(controller.preemption_point(), PreemptionAction::Continue);
        controller.exit_critical().unwrap();
        assert_eq!(controller.preemption_point(), PreemptionAction::Reschedule);
    }

    #[test]
    fn valid_interrupt_frame_is_recorded_without_a_lock() {
        let controller = PreemptionController::new();
        let frame = InterruptFrame::new(
            0x0000_0000_4000_1000,
            0x8,
            0x202,
            0x0000_0000_8000_1000,
            0x10,
        );
        assert_eq!(controller.record_frame(frame), Ok(()));
        assert_eq!(controller.last_frame(), Some(frame));
    }

    #[test]
    fn zero_return_addresses_fail_closed() {
        let controller = PreemptionController::new();
        let frame = InterruptFrame::new(0, 0x8, 0x202, 0x8000, 0x10);
        assert_eq!(
            controller.record_frame(frame),
            Err(PreemptionError::InvalidInterruptFrame)
        );
        assert_eq!(controller.last_frame(), None);
    }
}
