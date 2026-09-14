// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Process scheduler boundary for architecture-backed address-space switches.
#![cfg(feature = "x86-boot")]
extern crate alloc;
use alloc::collections::{BTreeMap, VecDeque};
use x86_64::{structures::paging::PageTable, VirtAddr};
use crate::ats1000::Pid;
use crate::kernel_stack::KernelStackManager;
use crate::memory::BootInfoFrameAllocator;
use crate::preemption::{PreemptionAction, TIMER_PREEMPTION};
use crate::process_context::ProcessExecutionContext;
use crate::x86_64_context_switch::{ContextStack, switch_to_context};
use crate::x86_64_page_table::{PageTableError, ProcessAddressSpaceManager};

/// Architecture boundary for the CPU-specific active kernel stack (TSS.RSP0).
pub trait KernelStackActivator {
    fn activate_kernel_stack(&mut self, stack_top: u64) -> Result<(), ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitchError {
    ProcessMissing,
    InvalidState,
    AddressSpace(PageTableError),
    ContextRootMismatch,
    KernelStackMissing,
    KernelStackActivation,
    ContextMissing,
    InvalidTimerContext,
}
impl From<PageTableError> for ContextSwitchError { fn from(value: PageTableError) -> Self { Self::AddressSpace(value) } }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState { Ready, Running }

pub struct ProcessScheduler {
    address_spaces: ProcessAddressSpaceManager,
    states: BTreeMap<Pid, RunState>,
    ready: VecDeque<Pid>,
    current: Option<Pid>,
    /// Stable virtual addresses of saved ContextStack objects. During timer
    /// preemption these point into the owning process's kernel stack.
    contexts: BTreeMap<Pid, u64>,
}
impl ProcessScheduler {
    pub const fn new(address_spaces: ProcessAddressSpaceManager) -> Self { Self { address_spaces, states: BTreeMap::new(), ready: VecDeque::new(), current: None, contexts: BTreeMap::new() } }
    pub fn current(&self) -> Option<Pid> { self.current }
    pub fn state(&self, pid: Pid) -> Option<RunState> { self.states.get(&pid).copied() }
    pub fn address_spaces(&self) -> &ProcessAddressSpaceManager { &self.address_spaces }
    pub fn address_spaces_mut(&mut self) -> &mut ProcessAddressSpaceManager { &mut self.address_spaces }
    pub fn context(&self, pid: Pid) -> Result<*const ContextStack, ContextSwitchError> {
        let address = *self.contexts.get(&pid).ok_or(ContextSwitchError::ContextMissing)?;
        Ok(address as *const ContextStack)
    }
    /// Registers the initial saved context for a process. The caller must have
    /// validated the pointer with ProcessExecutionContext::new().
    pub unsafe fn register_context(&mut self, context: &ProcessExecutionContext) -> Result<(), ContextSwitchError> {
        let pid = context.pid();
        if !self.address_spaces.contains(pid) { return Err(ContextSwitchError::ProcessMissing); }
        self.contexts.insert(pid, context.context_stack() as u64);
        Ok(())
    }
    pub unsafe fn register_process(&mut self, pid: Pid, physical_memory_offset: VirtAddr, active_root: &PageTable, frame_allocator: &mut BootInfoFrameAllocator) -> Result<(), ContextSwitchError> {
        self.address_spaces.create(pid, physical_memory_offset, active_root, frame_allocator)?;
        self.states.insert(pid, RunState::Ready); self.ready.push_back(pid); Ok(())
    }
    pub unsafe fn schedule_next(&mut self) -> Result<Option<Pid>, ContextSwitchError> {
        let next = match self.ready.pop_front() { Some(pid) => pid, None => return Ok(None) };
        if let Err(e) = self.switch_to(next) { self.ready.push_front(next); return Err(e); }
        Ok(Some(next))
    }
    pub unsafe fn preemption_point(&mut self) -> Result<Option<Pid>, ContextSwitchError> {
        if TIMER_PREEMPTION.preemption_point() != PreemptionAction::Reschedule { return Ok(None); }
        if !TIMER_PREEMPTION.take_if_safe() { return Ok(None); }
        self.schedule_next()
    }

    pub unsafe fn bind_kernel_stack<A: KernelStackActivator>(&self, pid: Pid, stacks: &KernelStackManager, activator: &mut A) -> Result<(), ContextSwitchError> {
        if !self.address_spaces.contains(pid) { return Err(ContextSwitchError::ProcessMissing); }
        let top = stacks.top(pid).map_err(|_| ContextSwitchError::KernelStackMissing)?;
        activator.activate_kernel_stack(top).map_err(|_| ContextSwitchError::KernelStackActivation)
    }

    /// Records the interrupted context and performs the complete timer-side
    /// A -> B transition. All validation occurs before the architecture commit.
    /// After CR3 is switched, no fallible scheduler operation is performed.
    pub unsafe fn preempt_from_timer<A: KernelStackActivator>(
        &mut self,
        current_pid: Pid,
        interrupted: *mut ContextStack,
        stacks: &KernelStackManager,
        activator: &mut A,
    ) -> Result<*const ContextStack, ContextSwitchError> {
        if self.current != Some(current_pid) || self.states.get(&current_pid).copied() != Some(RunState::Running) {
            return Err(ContextSwitchError::InvalidState);
        }
        if interrupted.is_null() { return Err(ContextSwitchError::InvalidTimerContext); }
        (*interrupted).validate().map_err(|_| ContextSwitchError::InvalidTimerContext)?;
        let target = *self.ready.front().ok_or(ContextSwitchError::ContextMissing)?;
        if target == current_pid || self.states.get(&target).copied() != Some(RunState::Ready) {
            return Err(ContextSwitchError::InvalidState);
        }
        let target_context = self.context(target)?;
        (*target_context).validate().map_err(|_| ContextSwitchError::InvalidTimerContext)?;
        let target_root = self.address_spaces.root(target)?;
        let target_stack = stacks.top(target).map_err(|_| ContextSwitchError::KernelStackMissing)?;

        // Commit boundary begins only after every validation above succeeded.
        activator.activate_kernel_stack(target_stack).map_err(|_| ContextSwitchError::KernelStackActivation)?;
        self.address_spaces.switch_to(target)?;

        // These operations are logically infallible here: both process entries
        // already exist and the target was already present in the ready queue.
        let _ = self.ready.pop_front();
        self.states.insert(current_pid, RunState::Ready);
        self.states.insert(target, RunState::Running);
        self.current = Some(target);
        self.contexts.insert(current_pid, interrupted as u64);
        let _ = target_root;
        TIMER_PREEMPTION.clear();
        Ok(target_context)
    }

    pub unsafe fn activate_context<A: KernelStackActivator>(&mut self, context: &ProcessExecutionContext, stacks: &KernelStackManager, activator: &mut A) -> Result<!, ContextSwitchError> {
        let pid = context.pid();
        if !self.address_spaces.contains(pid) { return Err(ContextSwitchError::ProcessMissing); }
        if self.current == Some(pid) || self.states.get(&pid).copied() != Some(RunState::Ready) { return Err(ContextSwitchError::InvalidState); }
        let owned_root = self.address_spaces.root(pid)?;
        if owned_root.physical_address() != context.root().physical_address() || owned_root.pid() != context.root().pid() { return Err(ContextSwitchError::ContextRootMismatch); }
        context.validate_user_return().map_err(|_| ContextSwitchError::InvalidState)?;
        if !self.ready.iter().any(|queued| *queued == pid) { return Err(ContextSwitchError::InvalidState); }
        self.register_context(context)?;
        self.bind_kernel_stack(pid, stacks, activator)?;
        self.address_spaces.switch_to(pid)?;
        self.ready.retain(|queued| *queued != pid);
        if let Some(old) = self.current { self.states.insert(old, RunState::Ready); self.ready.push_back(old); }
        self.states.insert(pid, RunState::Running);
        self.current = Some(pid);
        switch_to_context(context.context_stack())
    }

    pub unsafe fn switch_to(&mut self, next: Pid) -> Result<(), ContextSwitchError> {
        if !self.address_spaces.contains(next) { return Err(ContextSwitchError::ProcessMissing); }
        if self.current == Some(next) { return Ok(()); }
        if self.states.get(&next).copied() != Some(RunState::Ready) { return Err(ContextSwitchError::InvalidState); }
        self.address_spaces.switch_to(next)?;
        if let Some(old) = self.current { self.states.insert(old, RunState::Ready); self.ready.push_back(old); }
        self.states.insert(next, RunState::Running); self.current = Some(next); Ok(())
    }
    pub fn yield_current(&mut self) -> Result<(), ContextSwitchError> {
        let pid = self.current.take().ok_or(ContextSwitchError::InvalidState)?;
        self.states.insert(pid, RunState::Ready); self.ready.push_back(pid); Ok(())
    }
    pub fn unregister_process(&mut self, pid: Pid) -> Result<(), ContextSwitchError> {
        if self.current == Some(pid) { return Err(ContextSwitchError::InvalidState); }
        self.address_spaces.destroy(pid)?;
        self.ready.retain(|queued| *queued != pid);
        self.states.remove(&pid);
        self.contexts.remove(&pid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starts_without_current_process() { let scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new()); assert_eq!(scheduler.current(), None); }
    #[test]
    fn yield_without_current_is_rejected_without_state_mutation() { let mut scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new()); assert_eq!(scheduler.yield_current(), Err(ContextSwitchError::InvalidState)); assert_eq!(scheduler.current(), None); }
    #[test]
    fn context_root_mismatch_is_distinct_from_process_missing() { assert_ne!(ContextSwitchError::ContextRootMismatch, ContextSwitchError::ProcessMissing); }
    #[test]
    fn context_missing_is_distinct_from_invalid_timer_context() { assert_ne!(ContextSwitchError::ContextMissing, ContextSwitchError::InvalidTimerContext); }
}
