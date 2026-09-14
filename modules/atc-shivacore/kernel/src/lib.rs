// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! ShivaCore Kernel — Library Crate für Test-Ausführung
#![cfg_attr(not(test), no_std)]
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[cfg(test)]
extern crate std;
extern crate alloc;

pub mod allocator;
pub mod ats1000;
pub mod net;
pub mod capability;
pub mod process;
pub mod scheduler;
#[cfg(feature = "x86-boot")]
pub mod process_scheduler;
pub mod preemption;
pub mod ipc;
pub mod memory_manager;
pub mod memory_isolation;
pub mod process_address_space;
pub mod memory_capability;
pub mod frame_ownership;
#[cfg(feature = "x86-boot")]
pub mod memory;
#[cfg(feature = "x86-boot")]
pub mod x86_64_paging;
#[cfg(feature = "x86-boot")]
pub mod x86_64_address_space;
#[cfg(feature = "x86-boot")]
pub mod x86_64_page_table;
#[cfg(feature = "x86-boot")]
pub mod x86_64_user_mapping;
#[cfg(feature = "x86-boot")]
pub mod x86_64_context_switch;
#[cfg(feature = "x86-boot")]
pub mod x86_64_timer_entry;
#[cfg(feature = "x86-boot")]
pub mod process_context;
#[cfg(feature = "x86-boot")]
pub mod kernel_stack;
#[cfg(feature = "x86-boot")]
pub mod timer_scheduler_bridge;
pub mod atcfs;
pub mod vfs;
pub mod syscall;
pub mod timer;
pub mod tcpip;
pub mod p2p;
pub mod p2p_secure;
pub mod security;
pub mod mempool;
pub mod vm;
pub mod contract;
pub mod ai;
pub mod kernel_init;
pub mod cross_subsystem;
