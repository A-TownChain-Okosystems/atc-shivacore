// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! ShivaCore Kernel — Library Crate für Test-Ausführung
//!
//! Re-exportiert alle Kernel-Module für Unit- und Integrationstests.
#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

#[cfg(test)]
extern crate std;

extern crate alloc;

pub mod allocator;
pub mod ats1000;
// [K29-Build] ausgeschlossen: pub mod framebuffer;
// [K29-Build] ausgeschlossen: pub mod gdt;
// [K29-Build] ausgeschlossen: pub mod interrupts;
// [K29-Build] ausgeschlossen: pub mod memory;
// [K29-Build] ausgeschlossen: pub mod serial;
pub mod net;
pub mod capability;
pub mod process;
pub mod scheduler;
pub mod ipc;
pub mod memory_manager;
pub mod memory_isolation;
pub mod process_address_space;
pub mod memory_capability;
#[cfg(feature = "x86-boot")]
pub mod x86_64_paging;
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
// [K29-Build] ausgeschlossene optionale/experimentelle Module bleiben bewusst nicht exportiert:
// userspace, elf_loader, page_fault, user_sched, user_io, hw_drivers, system,
// sockets, devfs, threads, power, container, signals, smp, vmm, cow, tracing,
// container_net, lkm, module_security, fs_journal.
