// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// ShivaCore — Kernel-Einstiegspunkt.
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![no_main]

extern crate alloc;

mod allocator;
mod ats1000;
mod e2e_preemption;
mod framebuffer;
mod gdt;
mod interrupts;
mod memory;
mod serial;

use alloc::{boxed::Box, vec::Vec};
use bootloader_api::{config::{BootloaderConfig, Mapping}, entry_point, BootInfo};
use core::panic::PanicInfo;
use shivacore::kernel_stack::KernelStackManager;
use shivacore::process_scheduler::{KernelStackActivator, ProcessScheduler};
use shivacore::timer_scheduler_bridge::TimerSchedulerBridge;
use shivacore::x86_64_page_table::ProcessAddressSpaceManager;

struct TssKernelStackActivator;
impl KernelStackActivator for TssKernelStackActivator {
    fn activate_kernel_stack(&mut self, stack_top: u64) -> Result<(), ()> { gdt::set_kernel_stack_top(stack_top) }
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("ShivaCore: Kernel-Einstiegspunkt erreicht.");
    serial_println!("E2E_PREEMPTION_START");

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
    // The PIC initialization enables interrupts. Disable them while the
    // process address spaces and scheduler bridge are being constructed.
    x86_64::instructions::interrupts::disable();
    serial_println!("ShivaCore: GDT/IDT/PIC OK.");

    let phys_mem_offset = boot_info
        .physical_memory_offset
        .into_option()
        .expect("Bootloader hat physical_memory_offset nicht gesetzt (Config fehlt?)");
    let phys_mem_offset = x86_64::VirtAddr::new(phys_mem_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap-Initialisierung fehlgeschlagen");
    serial_println!("ShivaCore: Paging-Mapper + Heap initialisiert.");

    let mut scheduler = ProcessScheduler::new(ProcessAddressSpaceManager::new());
    let mut stacks = KernelStackManager::new();
    let (context_a, _context_b) = unsafe {
        e2e_preemption::prepare(&mut scheduler, &mut stacks, &mut mapper, &mut frame_allocator, phys_mem_offset)
    };

    serial_println!("E2E_PREEMPTION_A");

    let scheduler: &'static mut ProcessScheduler = Box::leak(Box::new(scheduler));
    let stacks: &'static KernelStackManager = Box::leak(Box::new(stacks));
    let activator: &'static mut TssKernelStackActivator = Box::leak(Box::new(TssKernelStackActivator));
    let bridge: &'static mut TimerSchedulerBridge<'static, TssKernelStackActivator> =
        Box::leak(Box::new(TimerSchedulerBridge::new(scheduler, stacks, activator)));
    let context_a: &'static shivacore::process_context::ProcessExecutionContext = Box::leak(context_a);

    interrupts::install_timer_scheduler_bridge(bridge);
    serial_println!("ShivaCore: Timer scheduler bridge installiert.");
    x86_64::instructions::interrupts::enable();

    unsafe { interrupts::activate_initial_context(context_a) }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! { panic!("Allokation fehlgeschlagen: {:?}", layout) }

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("ShivaCore: KERNEL PANIC -- {}", info);
    loop { x86_64::instructions::hlt(); }
}
