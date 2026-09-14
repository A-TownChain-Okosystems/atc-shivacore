// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Explicit bridge between the x86 timer entry and the process scheduler.
#![cfg(feature = "x86-boot")]

use crate::ats1000::Pid;
use crate::kernel_stack::KernelStackManager;
use crate::process_scheduler::{ContextSwitchError, KernelStackActivator, ProcessScheduler};
use crate::x86_64_context_switch::ContextStack;

/// Owns the reference boundary required by the timer dispatcher.
///
/// The bridge is deliberately not global: the boot/runtime layer owns it and
/// decides how the interrupt entry obtains access to the scheduler. This keeps
/// scheduler state explicit instead of hiding it behind ambient mutable state.
pub struct TimerSchedulerBridge<'a, A: KernelStackActivator> {
    scheduler: &'a mut ProcessScheduler,
    stacks: &'a KernelStackManager,
    activator: &'a mut A,
}

impl<'a, A: KernelStackActivator> TimerSchedulerBridge<'a, A> {
    pub fn new(
        scheduler: &'a mut ProcessScheduler,
        stacks: &'a KernelStackManager,
        activator: &'a mut A,
    ) -> Self {
        Self { scheduler, stacks, activator }
    }

    /// Performs one complete timer-side A -> B transition and returns the
    /// target context for the assembly restore path.
    pub unsafe fn dispatch(
        &mut self,
        current_pid: Pid,
        interrupted: *mut ContextStack,
    ) -> Result<*const ContextStack, ContextSwitchError> {
        self.scheduler.preempt_from_timer(
            current_pid,
            interrupted,
            self.stacks,
            self.activator,
        )
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
