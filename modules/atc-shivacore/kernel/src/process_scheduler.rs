// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Process scheduler boundary for architecture-backed address-space switches.
#![cfg(feature = "x86-boot")]
extern crate alloc;
use alloc::collections::{BTreeMap, VecDeque};
use x86_64::{structures::paging::PageTable, VirtAddr};
use crate::ats1000::Pid;
use crate::memory::BootInfoFrameAllocator;
use crate::preemption::{PreemptionAction, TIMER_PREEMPTION};
use crate::x86_64_page_table::{PageTableError, ProcessAddressSpaceManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitchError { ProcessMissing, InvalidState, AddressSpace(PageTableError) }
impl From<PageTableError> for ContextSwitchError { fn from(value: PageTableError) -> Self { Self::AddressSpace(value) } }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState { Ready, Running }

pub struct ProcessScheduler {
    address_spaces: ProcessAddressSpaceManager,
    states: BTreeMap<Pid, RunState>,
    ready: VecDeque<Pid>,
    current: Option<Pid>,
}
impl ProcessScheduler {
    pub const fn new(address_spaces: ProcessAddressSpaceManager) -> Self { Self { address_spaces, states: BTreeMap::new(), ready: VecDeque::new(), current: None } }
    pub fn current(&self) -> Option<Pid> { self.current }
    pub fn state(&self, pid: Pid) -> Option<RunState> { self.states.get(&pid).copied() }
    pub fn address_spaces(&self) -> &ProcessAddressSpaceManager { &self.address_spaces }
    pub fn address_spaces_mut(&mut self) -> &mut ProcessAddressSpaceManager { &mut self.address_spaces }
    pub unsafe fn register_process(&mut self, pid: Pid, physical_memory_offset: VirtAddr, active_root: &PageTable, frame_allocator: &mut BootInfoFrameAllocator) -> Result<(), ContextSwitchError> {
        self.address_spaces.create(pid, physical_memory_offset, active_root, frame_allocator)?;
        self.states.insert(pid, RunState::Ready); self.ready.push_back(pid); Ok(())
    }
    pub unsafe fn schedule_next(&mut self) -> Result<Option<Pid>, ContextSwitchError> {
        let next = match self.ready.pop_front() { Some(pid) => pid, None => return Ok(None) };
        if let Err(e) = self.switch_to(next) { self.ready.push_front(next); return Err(e); }
        Ok(Some(next))
    }
    /// Consumes a timer request only at a scheduler-defined safe point.
    ///
    /// The timer IRQ itself never calls this method and never changes CR3.
    /// If the kernel is in a critical section the request remains pending.
    pub unsafe fn preemption_point(&mut self) -> Result<Option<Pid>, ContextSwitchError> {
        if TIMER_PREEMPTION.preemption_point() != PreemptionAction::Reschedule {
            return Ok(None);
        }
        if !TIMER_PREEMPTION.take_if_safe() {
            return Ok(None);
        }
        self.schedule_next()
    }
    /// Architecture switch is the commit boundary: scheduler state is not
    /// changed until the address-space switch succeeds.
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
    /// Address-space destruction is performed before scheduler metadata is
    /// removed, so a failed destroy leaves the scheduler state untouched.
    pub fn unregister_process(&mut self, pid: Pid) -> Result<(), ContextSwitchError> {
        if self.current == Some(pid) { return Err(ContextSwitchError::InvalidState); }
        self.address_spaces.destroy(pid)?;
        self.ready.retain(|queued| *queued != pid);
        self.states.remove(&pid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starts_without_current_process() {
        let scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new());
        assert_eq!(scheduler.current(), None);
    }
    #[test]
    fn yield_without_current_is_rejected_without_state_mutation() {
        let mut scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new());
        assert_eq!(scheduler.yield_current(), Err(ContextSwitchError::InvalidState));
        assert_eq!(scheduler.current(), None);
    }
}
