// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Explicit bridge between the x86 timer entry and the process scheduler.
#![cfg(feature = "x86-boot")]

use crate::kernel_stack::KernelStackManager;
use crate::process_context::ProcessExecutionContext;
use crate::process_scheduler::{ContextSwitchError, KernelStackActivator, ProcessScheduler};
use crate::x86_64_context_switch::ContextStack;

pub struct TimerSchedulerBridge<'a, A: KernelStackActivator> {
    scheduler: &'a mut ProcessScheduler,
    stacks: &'a KernelStackManager,
    activator: &'a mut A,
}

impl<'a, A: KernelStackActivator> TimerSchedulerBridge<'a, A> {
    pub fn new(scheduler: &'a mut ProcessScheduler, stacks: &'a KernelStackManager, activator: &'a mut A) -> Self {
        Self { scheduler, stacks, activator }
    }

    /// Performs one complete timer-side A -> B transition. The current PID is
    /// taken from scheduler state so the IRQ path cannot supply a mismatching
    /// process identity.
    pub unsafe fn dispatch(&mut self, interrupted: *mut ContextStack) -> Result<*const ContextStack, ContextSwitchError> {
        let current_pid = self.scheduler.current().ok_or(ContextSwitchError::InvalidState)?;
        self.scheduler.preempt_from_timer(current_pid, interrupted, self.stacks, self.activator)
    }

    /// Enters the first process context. This is intentionally one-way.
    pub unsafe fn activate_initial(&mut self, context: &ProcessExecutionContext) -> ! {
        self.scheduler
            .activate_context(context, self.stacks, self.activator)
            .expect("initial process context activation failed")
    }

    pub fn scheduler(&self) -> &ProcessScheduler { self.scheduler }
    pub fn scheduler_mut(&mut self) -> &mut ProcessScheduler { self.scheduler }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x86_64_page_table::ProcessAddressSpaceManager;

    struct TestActivator;
    impl KernelStackActivator for TestActivator {
        fn activate_kernel_stack(&mut self, _stack_top: u64) -> Result<(), ()> { Ok(()) }
    }

    #[test]
    fn bridge_keeps_scheduler_explicit() {
        let mut scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new());
        let stacks = KernelStackManager::new();
        let mut activator = TestActivator;
        let bridge = TimerSchedulerBridge::new(&mut scheduler, &stacks, &mut activator);
        assert_eq!(bridge.scheduler().current(), None);
    }
}
