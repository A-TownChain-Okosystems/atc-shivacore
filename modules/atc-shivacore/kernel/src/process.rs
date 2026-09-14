// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! ShivaCore Kernel — process lifecycle.
//!
//! The process manager owns the logical PCB lifecycle. Architecture-specific
//! address-space construction is kept in `x86_64_page_table`; the two layers
//! are connected through explicit lifecycle hooks rather than hidden globals.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::capability::{CapabilityTable, CapId, Pid, ResourceType, Rights};

#[cfg(feature = "x86-boot")]
use x86_64::{structures::paging::PageTable, VirtAddr};
#[cfg(feature = "x86-boot")]
use crate::memory::BootInfoFrameAllocator;
#[cfg(feature = "x86-boot")]
use crate::x86_64_page_table::{PageTableError, ProcessAddressSpaceManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessType { Agent, Service, Contract, System, Validator }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState { Ready, Running, Blocked, Terminated(ExitCode) }

pub type ExitCode = i32;

#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub pid: Pid,
    pub ptype: ProcessType,
    pub priority: u8,
    pub state: ProcessState,
    pub parent: Option<Pid>,
    pub children: Vec<Pid>,
}

pub struct ProcessManager {
    processes: BTreeMap<Pid, ProcessControlBlock>,
    next_pid: AtomicU32,
    pub caps: CapabilityTable,
    #[cfg(feature = "x86-boot")]
    address_spaces: Option<ProcessAddressSpaceManager>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            next_pid: AtomicU32::new(1),
            caps: CapabilityTable::new(),
            #[cfg(feature = "x86-boot")]
            address_spaces: None,
        }
    }

    /// Enables architecture-backed address-space lifecycle management.
    #[cfg(feature = "x86-boot")]
    pub fn attach_address_spaces(&mut self, manager: ProcessAddressSpaceManager) -> Result<(), PageTableError> {
        if self.address_spaces.is_some() { return Err(PageTableError::ProcessExists); }
        self.address_spaces = Some(manager);
        Ok(())
    }

    #[cfg(feature = "x86-boot")]
    pub fn address_spaces(&self) -> Option<&ProcessAddressSpaceManager> { self.address_spaces.as_ref() }

    #[cfg(feature = "x86-boot")]
    pub fn address_spaces_mut(&mut self) -> Option<&mut ProcessAddressSpaceManager> { self.address_spaces.as_mut() }

    /// Creates a logical process. On x86_64 the page-table root is attached by
    /// `spawn_with_address_space`, because constructing it requires boot-time
    /// physical-memory and frame-allocator state.
    pub fn spawn(&mut self, ptype: ProcessType, priority: u8) -> Pid {
        let pid = Pid(self.next_pid.fetch_add(1, Ordering::SeqCst));
        self.insert_process(pid, ptype, priority);
        pid
    }

    #[cfg(feature = "x86-boot")]
    /// Creates a process and its private address space as one lifecycle step.
    /// If address-space creation fails, the PCB and capability are rolled back.
    pub unsafe fn spawn_with_address_space(
        &mut self,
        ptype: ProcessType,
        priority: u8,
        physical_memory_offset: VirtAddr,
        active_root: &PageTable,
        frame_allocator: &mut BootInfoFrameAllocator,
    ) -> Result<Pid, PageTableError> {
        let pid = Pid(self.next_pid.fetch_add(1, Ordering::SeqCst));
        self.insert_process(pid, ptype, priority);
        let result = self.address_spaces_mut()
            .ok_or(PageTableError::InvalidRoot)
            .and_then(|spaces| spaces.create(pid, physical_memory_offset, active_root, frame_allocator));
        if let Err(error) = result {
            self.rollback_process(pid);
            return Err(error);
        }
        Ok(pid)
    }

    fn insert_process(&mut self, pid: Pid, ptype: ProcessType, priority: u8) {
        self.processes.insert(pid, ProcessControlBlock {
            pid, ptype, priority, state: ProcessState::Ready, parent: None, children: Vec::new(),
        });
        let addr_space = pid.0 as u64;
        self.caps.create(pid, ResourceType::Memory, addr_space, Rights::READ | Rights::WRITE | Rights::EXEC | Rights::DELEGATE);
    }

    fn rollback_process(&mut self, pid: Pid) {
        let cap_ids: Vec<CapId> = self.caps.list_for(pid).iter().map(|c| c.id).collect();
        for cap_id in cap_ids { self.caps.revoke(cap_id); }
        self.processes.remove(&pid);
    }

    pub fn spawn_child(&mut self, parent: Pid, ptype: ProcessType, priority: u8) -> Option<Pid> {
        if !self.processes.contains_key(&parent) { return None; }
        let child_pid = self.spawn(ptype, priority);
        if let Some(child) = self.processes.get_mut(&child_pid) { child.parent = Some(parent); }
        if let Some(parent_pcb) = self.processes.get_mut(&parent) { parent_pcb.children.push(child_pid); }
        Some(child_pid)
    }

    pub fn kill(&mut self, pid: Pid, exit_code: ExitCode) -> bool {
        let parent = match self.processes.get(&pid) {
            Some(p) if !matches!(p.state, ProcessState::Terminated(_)) => p.parent,
            _ => return false,
        };
        let cap_ids: Vec<CapId> = self.caps.list_for(pid).iter().map(|c| c.id).collect();
        for cap_id in cap_ids { self.caps.revoke(cap_id); }
        if let Some(pcb) = self.processes.get_mut(&pid) { pcb.state = ProcessState::Terminated(exit_code); }
        if let Some(parent_pid) = parent {
            if let Some(parent_pcb) = self.processes.get_mut(&parent_pid) { parent_pcb.children.retain(|&c| c != pid); }
        }
        true
    }

    #[cfg(feature = "x86-boot")]
    /// Terminates a process only after its address space has been detached.
    /// The currently active CR3 is never destroyed through this API.
    pub fn kill_with_address_space(&mut self, pid: Pid, exit_code: ExitCode) -> Result<bool, PageTableError> {
        if !self.processes.contains_key(&pid) { return Ok(false); }
        if self.address_spaces.as_ref().is_some_and(|spaces| spaces.current_pid() == Some(pid)) {
            return Err(PageTableError::InvalidRoot);
        }
        let killed = self.kill(pid, exit_code);
        if killed {
            if let Some(spaces) = self.address_spaces.as_mut() {
                let _ = spaces.destroy(pid)?;
            }
        }
        Ok(killed)
    }

    pub fn wait(&self, pid: Pid) -> Option<ExitCode> {
        match self.processes.get(&pid)?.state { ProcessState::Terminated(code) => Some(code), _ => None }
    }
    pub fn list_processes(&self) -> Vec<&ProcessControlBlock> { self.processes.values().collect() }
    pub fn get(&self, pid: Pid) -> Option<&ProcessControlBlock> { self.processes.get(&pid) }
    pub fn set_running(&mut self, pid: Pid) -> bool { match self.processes.get_mut(&pid) { Some(p) if p.state == ProcessState::Ready => { p.state = ProcessState::Running; true }, _ => false } }
    pub fn set_ready(&mut self, pid: Pid) -> bool { match self.processes.get_mut(&pid) { Some(p) if p.state == ProcessState::Running => { p.state = ProcessState::Ready; true }, _ => false } }
    pub fn set_blocked(&mut self, pid: Pid) -> bool { match self.processes.get_mut(&pid) { Some(p) if p.state == ProcessState::Running || p.state == ProcessState::Ready => { p.state = ProcessState::Blocked; true }, _ => false } }
    pub fn unblock(&mut self, pid: Pid) -> bool { match self.processes.get_mut(&pid) { Some(p) if p.state == ProcessState::Blocked => { p.state = ProcessState::Ready; true }, _ => false } }
    pub fn active_count(&self) -> usize { self.processes.values().filter(|p| !matches!(p.state, ProcessState::Terminated(_))).count() }
    pub fn check_capability(&self, pid: Pid, resource_type: ResourceType, resource_id: u64, required: Rights) -> bool { self.caps.check(pid, resource_type, resource_id, required) }
    pub fn delegate_capability(&mut self, source: Pid, cap_id: CapId, target: Pid, rights: Rights) -> Result<CapId, crate::capability::CapabilityError> { self.caps.delegate(source, cap_id, target, rights) }
}

impl Default for ProcessManager { fn default() -> Self { Self::new() } }
