// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// ShivaCore — Kernel-Einstiegspunkt.
// K-Sprint 0: Boot (BIOS+UEFI via `bootloader` 0.11), serielle Debug-Konsole,
// Framebuffer-Textausgabe.
// K-Sprint 1: GDT + TSS (Double-Fault-Stack), IDT (Breakpoint/Double-Fault/
// Page-Fault), PIC-Remapping (0x20-0x2F), Timer+Keyboard-Interrupts aktiv.
// K-Sprint 2: Paging-Mapper, Frame-Allocator, Heap-Allokator.
// K-Sprint TPM: ACPI RSDP/XSDT -> TPM2 -> CRB Locality-0 MMIO probe + buffer validation.
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![no_main]

extern crate alloc;

mod acpi;
mod allocator;
mod ats1000;
mod framebuffer;
mod gdt;
mod interrupts;
mod memory;
mod mmio;
mod serial;
mod tpm_buffer;

use alloc::{boxed::Box, vec::Vec};
use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    entry_point, BootInfo,
};
use core::panic::PanicInfo;

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
        println!("ShivaCore Kernel v0.0.3 -- K-Sprint 2 + TPM");
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
    let mut tpm_buffers = tpm_buffer::TpmBufferAccess::new(phys_mem_offset, &boot_info.memory_regions);

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap-Initialisierung fehlgeschlagen");
    serial_println!("ShivaCore: Paging-Mapper + Heap initialisiert (100 KiB).");

    // TPM2 ACPI discovery is read-only. No TPM command is issued until the CRB
    // transport, locality state and command/response buffer ranges are validated.
    if let Some(rsdp) = boot_info.rsdp_addr.into_option() {
        let reader = acpi::PhysicalReader::new(phys_mem_offset);
        match reader.find_tpm2(rsdp) {
            Ok(table) => {
                serial_println!("ShivaCore: TPM2 ACPI table @ {:#x}, len={}", table.physical_address, table.length);
                if table.length >= 52 {
                    let mut tpm2 = [0u8; 52];
                    if unsafe { reader.read(table.physical_address, &mut tpm2) }.is_ok() {
                        let reserved = u16::from_le_bytes([tpm2[38], tpm2[39]]);
                        let control_area = u64::from_le_bytes([
                            tpm2[40], tpm2[41], tpm2[42], tpm2[43],
                            tpm2[44], tpm2[45], tpm2[46], tpm2[47],
                        ]);
                        let start_method = u32::from_le_bytes([tpm2[48], tpm2[49], tpm2[50], tpm2[51]]);
                        if reserved == 0 && start_method == 7 && control_area != 0 {
                            match mmio::map_mmio(&mut mapper, &mut frame_allocator, x86_64::PhysAddr::new(control_area), 0x1000) {
                                Ok(region) => {
                                    let _ = tpm_buffers.allow_mmio_range(x86_64::PhysAddr::new(control_area), 0x1000);
                                    match unsafe { region.read_u32(0x00) } {
                                        Ok(locality_state) => serial_println!("ShivaCore: TPM2 CRB MMIO probe OK | control_area={:#x} locality_state={:#010x}", control_area, locality_state),
                                        Err(_) => serial_println!("ShivaCore: TPM2 CRB MMIO mapped, locality-state read failed"),
                                    }

                                    // The TPM supplies these physical addresses. Validate the
                                    // complete ranges before any command/response dereference.
                                    let cmd_size = unsafe { region.read_u32(0x58) }.unwrap_or(0) as usize;
                                    let cmd_low = unsafe { region.read_u32(0x5C) }.unwrap_or(0) as u64;
                                    let cmd_high = unsafe { region.read_u32(0x60) }.unwrap_or(0) as u64;
                                    let cmd_address = cmd_low | (cmd_high << 32);
                                    let rsp_size = unsafe { region.read_u32(0x64) }.unwrap_or(0) as usize;
                                    let rsp_low = unsafe { region.read_u32(0x68) }.unwrap_or(0) as u64;
                                    let rsp_high = unsafe { region.read_u32(0x6C) }.unwrap_or(0) as u64;
                                    let rsp_address = rsp_low | (rsp_high << 32);

                                    if cmd_size > 0 && cmd_size <= 4096 && tpm_buffers.validate_range(cmd_address, cmd_size).is_ok() {
                                        serial_println!("ShivaCore: TPM CRB command buffer range validated | addr={:#x} len={}", cmd_address, cmd_size);
                                    } else {
                                        serial_println!("ShivaCore: TPM CRB command buffer rejected");
                                    }
                                    if rsp_size > 0 && rsp_size <= 4096 && tpm_buffers.validate_range(rsp_address, rsp_size).is_ok() {
                                        serial_println!("ShivaCore: TPM CRB response buffer range validated | addr={:#x} len={}", rsp_address, rsp_size);
                                    } else {
                                        serial_println!("ShivaCore: TPM CRB response buffer rejected");
                                    }
                                }
                                Err(_) => serial_println!("ShivaCore: TPM2 CRB MMIO mapping failed"),
                            }
                        } else {
                            serial_println!("ShivaCore: TPM2 ACPI descriptor rejected");
                        }
                    }
                }
            }
            Err(acpi::AcpiError::NotFound) => serial_println!("ShivaCore: no TPM2 ACPI table found"),
            Err(_) => serial_println!("ShivaCore: TPM2 ACPI discovery failed closed"),
        }
    } else {
        serial_println!("ShivaCore: no ACPI RSDP supplied by bootloader");
    }

    let boxed = Box::new(41);
    serial_println!("ShivaCore: Box-Test -- Wert: {}", *boxed);

    let mut vec = Vec::new();
    for i in 0..10 { vec.push(i); }
    serial_println!("ShivaCore: Vec-Test -- Summe 0..10: {}", vec.iter().sum::<i32>());

    println!("K-Sprint 2: Paging/Heap OK (Box+Vec getestet)");
    serial_println!("ShivaCore: K-Sprint 2 abgeschlossen. Uebergabe an Idle-Loop.");

    loop { x86_64::instructions::hlt(); }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! { panic!("Allokation fehlgeschlagen: {:?}", layout) }

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("ShivaCore: KERNEL PANIC -- {}", info);
    loop { x86_64::instructions::hlt(); }
}
