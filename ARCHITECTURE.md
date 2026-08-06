# ARCHITECTURE.md — atc-shivacore

> Copyright © Michael Wroblewski / A-TownChain-Okosystems. All Rights Reserved.

## File Tree
```tree
atc-shivacore/
├── Cargo.toml — Kernel crate configuration for no_std x86_64-unknown-none target
├── .gitignore — Git ignore configuration for build artifacts and target directories
└── kernel/
    └── src/
        ├── lib.rs — Kernel crate root, global module declarations, and main initialization
        ├── boot.rs — Early boot sequence & multi-stage initialization pipeline (K0-K40)
        ├── gdt.rs — Global Descriptor Table setup, segment descriptors, and TSS loading
        ├── idt.rs — Interrupt Descriptor Table, exception vectors, and ISR routines
        ├── pic.rs — 8259 Programmable Interrupt Controller driver & IRQ mapping
        ├── paging.rs — x86_64 4-level page table management & virtual memory protection
        ├── heap.rs — Kernel heap management & dynamic memory allocation engine
        ├── alloc.rs — Global memory allocator implementation for no_std environment
        ├── capabilities.rs — Capability-based access control framework and security tokens
        ├── process.rs — Process control blocks (PCB), process creation, and lifecycle
        ├── scheduler.rs — Preemptive task scheduler & multi-core CPU scheduling
        ├── ipc.rs — Inter-process communication primitives and zero-copy message queues
        ├── did.rs — Decentralized Identity primitives integrated into kernel safety layer
        ├── ed25519.rs — In-kernel Ed25519 cryptographic signature verification
        ├── knowledge_graph.rs — In-kernel structured knowledge graph and state store
        ├── vfs.rs — Virtual File System abstraction, mount table, and file descriptors
        ├── syscall.rs — System call dispatch table & Ring 3 to Ring 0 execution bridge
        ├── timer.rs — Programmable Interval Timer (PIT) and LAPIC timer drivers
        ├── block_device.rs — Abstract block device interface for storage drives
        ├── network.rs — Embedded network stack abstraction & packet buffer management
        ├── tcpip.rs — In-kernel TCP/IP protocol suite implementation
        ├── consensus.rs — Consensus validation primitives executed within kernel space
        ├── security.rs — Kernel ring protection, permissions, and security audit
        ├── consensus_engine.rs — Core consensus state machine engine
        ├── userspace.rs — Ring 3 context switching and user execution isolation
        ├── elf_loader.rs — Executable and Linkable Format (ELF) binary loader
        ├── page_fault.rs — Page fault interrupt handler, demand paging & copy-on-write
        ├── user_sched.rs — User-level thread scheduling policy and task queue
        ├── user_io.rs — User-space buffer memory validation and safe copying
        ├── hw_drivers.rs — Hardware device driver abstraction layer
        ├── system.rs — System telemetry, system info syscalls, and shutdown/reboot
        ├── sockets.rs — Socket network primitives for userspace applications
        ├── devfs.rs — Device filesystem (/dev) nodes and dynamic driver bindings
        ├── threads.rs — Kernel thread management and synchronization primitives
        ├── futex.rs — Fast userspace mutex support and wait queue handling
        └── acpi.rs — ACPI table parser and system power management interface
```

## Module Descriptions
- kernel/src/lib.rs — Entry point for ShivaCore kernel library exposing core subsystems.
- kernel/src/boot.rs — Executes boot stages K0 through K40 to initialize hardware, memory, and kernel subsystems.
- kernel/src/gdt.rs / idt.rs / pic.rs — Sets up low-level CPU descriptor tables, task state segments, and hardware IRQ routing.
- kernel/src/paging.rs / heap.rs / alloc.rs — Manages page tables, virtual-to-physical mappings, and no_std global heap allocator.
- kernel/src/capabilities.rs / security.rs — Implements object-level capability security and ring protection.
- kernel/src/process.rs / scheduler.rs / threads.rs — Controls task state, process tree execution, context switching, and multi-core scheduling.
- kernel/src/ipc.rs / sockets.rs — Provides synchronous and asynchronous zero-copy inter-process communication channels.
- kernel/src/did.rs / ed25519.rs — Embedded cryptographic identity verification for decentralized kernel operations.
- kernel/src/knowledge_graph.rs — In-kernel graph database for dynamic entity and permission state tracking.
- kernel/src/vfs.rs / devfs.rs / block_device.rs — Provides virtual filesystem trees, storage device drivers, and device node abstractions.
- kernel/src/syscall.rs / userspace.rs / elf_loader.rs — Handles syscall interrupts, user binary loading, and safe Ring 3 sandboxing.
- kernel/src/timer.rs / acpi.rs — Manages hardware timer interrupts, system clock ticks, and power state transitions.
- kernel/src/network.rs / tcpip.rs — Kernel-level network stack for high-throughput packet routing.
- kernel/src/consensus.rs / consensus_engine.rs — Hardware-accelerated on-chain consensus state verification.

## Build System
- Cargo.toml — Configured for bare-metal `#![no_std]` Rust compilation targeting `x86_64-unknown-none`.
- Targets bare-metal hardware and virtualized execution environments without standard OS runtime dependencies.

## Dependencies
- spin — Bare-metal spinlock primitives for multi-threaded kernel synchronization.
- x86_64 — Low-level x86_64 CPU instructions, control registers, and page table definitions.
- volatile — Safe access wrappers for memory-mapped hardware I/O addresses.
- lazy_static / conquer-once — Thread-safe static initialization for kernel data structures.
