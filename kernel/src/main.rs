// ShivaCore — Kernel-Einstiegspunkt.
// K-Sprint 0: Boot (BIOS+UEFI via `bootloader` 0.11), serielle Debug-Konsole,
// Framebuffer-Textausgabe.
// K-Sprint 1: GDT + TSS (Double-Fault-Stack), IDT (Breakpoint/Double-Fault/
// Page-Fault), PIC-Remapping (0x20-0x2F), Timer+Keyboard-Interrupts aktiv.
// K-Sprint 2: Paging-Mapper (OffsetPageTable), Frame-Allocator, Heap-
// Allokator (linked_list_allocator) -- `alloc` (Box/Vec/String) nutzbar.
// Kein Linux-Unterbau, kein Fremdcode jenseits des minimalen Boot-Protokolls.
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![no_main]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

extern crate alloc;

mod allocator;
mod ats1000;
mod framebuffer;
mod gdt;
mod interrupts;
mod memory;
mod serial;
mod capability;
mod process;
mod scheduler;
mod ipc;
mod did;
mod remote_caps;
mod knowledge_graph;
mod memory_manager;
mod atcfs;
mod vfs;
mod syscall;
mod timer;
mod block;
mod net;
mod tcpip;
mod p2p;
mod security;
mod consensus;
mod mempool;
mod blockchain;
mod vm;
mod contract;
mod ai;
mod kernel_init;
mod cross_subsystem;
mod atcnet;
mod genesis;
mod genesis_bridge;
mod gossip_bridge;
mod security_audit;

use alloc::{boxed::Box, vec::Vec};
use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    entry_point, BootInfo,
};
use core::panic::PanicInfo;

// Bootloader anweisen, das gesamte physische RAM linear ins virtuelle
// Adressvolumen zu mappen (Voraussetzung fuer den Paging-Mapper in memory.rs).
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("ShivaCore: Kernel-Einstiegspunkt erreicht.");

    if let Some(fb) = boot_info.framebuffer.as_mut() {
        framebuffer::init(fb);
        println!("ShivaCore Kernel v0.0.3 -- K-Sprint 2");
        println!("Boot: OK | Serial: OK | Framebuffer: OK");
    } else {
        serial_println!("ShivaCore: WARNUNG -- kein Framebuffer vom Bootloader erhalten.");
    }

    gdt::init();
    interrupts::init_idt();
    x86_64::instructions::interrupts::int3();
    interrupts::init_pics();
    serial_println!("ShivaCore: GDT/IDT/PIC OK (K-Sprint 1).");

    let phys_mem_offset = boot_info
        .physical_memory_offset
        .into_option()
        .expect("Bootloader hat physical_memory_offset nicht gesetzt (Config fehlt?)");
    let phys_mem_offset = x86_64::VirtAddr::new(phys_mem_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap-Initialisierung fehlgeschlagen");
    serial_println!("ShivaCore: Paging-Mapper + Heap initialisiert (100 KiB).");

    // Heap live testen: Box + Vec muessen funktionieren, ohne zu crashen.
    let boxed = Box::new(41);
    serial_println!("ShivaCore: Box-Test -- Wert: {}", *boxed);

    let mut vec = Vec::new();
    for i in 0..10 {
        vec.push(i);
    }
    serial_println!("ShivaCore: Vec-Test -- Summe 0..10: {}", vec.iter().sum::<i32>());

    println!("K-Sprint 2: Paging/Heap OK (Box+Vec getestet)");

    // ── K-Sprint 23: Vollständige Kernel-Init aller Subsysteme ──
    serial_println!("ShivaCore: Starte Kernel-Init (K-Sprint 23)...");
    
    match kernel_init::KernelState::boot() {
        Ok(ks) => {
            serial_println!("ShivaCore: Kernel-Init erfolgreich!");
            for (phase, status) in &ks.init_log {
                let icon = match status {
                    kernel_init::InitStatus::Ready => "OK",
                    kernel_init::InitStatus::Initializing => "..",
                    kernel_init::InitStatus::Failed => "FAIL",
                    kernel_init::InitStatus::NotStarted => "--",
                };
                serial_println!("  [{}] {}", icon, phase.label());
            }
            serial_println!("ShivaCore: {} Module aktiv", kernel_init::kernel_version());
            serial_println!("ShivaCore: P2P port {}, Chain height {}, VM contracts: {}",
                ks.p2p.listen_port(),
                ks.chain.current_height(),
                ks.vm.contract_count());
            serial_println!("ShivaCore: Boot Complete. Uebergabe an Idle-Loop.");
        }
        Err(e) => {
            serial_println!("ShivaCore: KERNEL-INIT FEHLGESCHLAGEN: {:?}", e);
            panic!("Kernel init failed");
        }
    }

    println!("K-Sprint 23: Kernel-Init Complete (alle Subsysteme aktiv)");
    serial_println!("ShivaCore: K-Sprint 23 abgeschlossen. Uebergabe an Idle-Loop.");

    loop {
        x86_64::instructions::hlt();
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Allokation fehlgeschlagen: {:?}", layout)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("ShivaCore: KERNEL PANIC -- {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
