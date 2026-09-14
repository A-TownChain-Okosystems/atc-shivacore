// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Minimal two-process Ring-3 runtime used exclusively for the x86_64
//! timer-preemption E2E path.

use alloc::boxed::Box;
use core::mem::size_of;
use shivacore::ats1000::Pid;
use shivacore::kernel_stack::{KernelStack, KernelStackManager, DEFAULT_STACK_PAGES, PAGE_SIZE};
use shivacore::memory::BootInfoFrameAllocator;
use shivacore::memory_isolation::PageFlags;
use shivacore::process_context::{KernelStack as ExecutionKernelStack, ProcessExecutionContext};
use shivacore::process_scheduler::ProcessScheduler;
use shivacore::x86_64_context_switch::{ContextStack, IretFrame, SavedRegisters};
use shivacore::x86_64_page_table::ProcessAddressSpaceManager;
use x86_64::{structures::paging::{Page, Size4KiB}, PageTable, PhysAddr, VirtAddr};

pub const PROCESS_A: Pid = Pid(1);
pub const PROCESS_B: Pid = Pid(2);
pub const USER_CODE_A: u64 = 0x0000_0001_4000_0000;
pub const USER_CODE_B: u64 = 0x0000_0001_4020_0000;
pub const USER_STACK_A: u64 = 0x0000_0001_8000_0000;
pub const USER_STACK_B: u64 = 0x0000_0001_8020_0000;

/// Builds the smallest real Ring-3 A/B setup. The function returns only after
/// all process-owned page tables, kernel stacks, user code/stack mappings and
/// saved contexts have been constructed and individually validated.
pub unsafe fn prepare(
    scheduler: &mut ProcessScheduler,
    stacks: &mut KernelStackManager,
    mapper: &mut x86_64::structures::paging::OffsetPageTable<'static>,
    frame_allocator: &mut BootInfoFrameAllocator,
    physical_memory_offset: VirtAddr,
) -> (Box<ProcessExecutionContext>, Box<ProcessExecutionContext>) {
    let active_root: &PageTable = mapper.level_4_table();
    scheduler.register_process(PROCESS_A, physical_memory_offset, active_root, frame_allocator).expect("E2E: create process A failed");
    scheduler.register_process(PROCESS_B, physical_memory_offset, active_root, frame_allocator).expect("E2E: create process B failed");

    let stack_a = stacks.allocate(PROCESS_A, scheduler.address_spaces_mut(), frame_allocator).expect("E2E: kernel stack A failed");
    let stack_b = stacks.allocate(PROCESS_B, scheduler.address_spaces_mut(), frame_allocator).expect("E2E: kernel stack B failed");

    map_user_image(scheduler, PROCESS_A, USER_CODE_A, USER_STACK_A, frame_allocator, physical_memory_offset);
    map_user_image(scheduler, PROCESS_B, USER_CODE_B, USER_STACK_B, frame_allocator, physical_memory_offset);

    let context_a = initialize_context(
        PROCESS_A, stack_a, USER_CODE_A, USER_STACK_A,
        physical_memory_offset,
        0x1b, 0x23,
    );
    let context_b = initialize_context(
        PROCESS_B, stack_b, USER_CODE_B, USER_STACK_B,
        physical_memory_offset,
        0x1b, 0x23,
    );

    // ProcessExecutionContext::new validates the ContextStack by dereferencing
    // its process-private kernel-stack mapping. Validate each one while its
    // owning CR3 is active; never dereference B while A's address space is live.
    scheduler.address_spaces_mut().switch_to(PROCESS_B).expect("E2E: activate B for validation failed");
    let context_b = Box::new(context_b);
    let context_b = Box::new(ProcessExecutionContext::new(
        PROCESS_B, scheduler.address_spaces().root(PROCESS_B).unwrap(),
        context_b.kernel_stack(), context_b.context_stack(),
    ).expect("E2E: validate context B failed"));

    scheduler.address_spaces_mut().switch_to(PROCESS_A).expect("E2E: activate A for validation failed");
    let context_a = Box::new(context_a);
    let context_a = Box::new(ProcessExecutionContext::new(
        PROCESS_A, scheduler.address_spaces().root(PROCESS_A).unwrap(),
        context_a.kernel_stack(), context_a.context_stack(),
    ).expect("E2E: validate context A failed"));

    scheduler.register_context(&context_a).expect("E2E: register context A failed");
    scheduler.register_context(&context_b).expect("E2E: register context B failed");
    (context_a, context_b)
}

unsafe fn map_user_image(
    scheduler: &mut ProcessScheduler,
    pid: Pid,
    code: u64,
    stack: u64,
    frame_allocator: &mut BootInfoFrameAllocator,
    physical_memory_offset: VirtAddr,
) {
    let code_frame = frame_allocator.allocate_frame().expect("E2E: code frame allocation failed");
    let stack_frame = frame_allocator.allocate_frame().expect("E2E: user stack frame allocation failed");
    let code_page = Page::<Size4KiB>::containing_address(VirtAddr::new(code));
    let stack_page = Page::<Size4KiB>::containing_address(VirtAddr::new(stack));
    let table = scheduler.address_spaces_mut().get_mut(pid).expect("E2E: process page table missing");
    table.map_user_page(code_page, code_frame, PageFlags::READ.union(PageFlags::EXECUTE).union(PageFlags::USER), frame_allocator).expect("E2E: code mapping failed");
    table.map_user_page(stack_page, stack_frame, PageFlags::READ.union(PageFlags::WRITE).union(PageFlags::USER), frame_allocator).expect("E2E: stack mapping failed");

    // `EB FE` is a two-byte infinite loop. The timer interrupt is the only
    // mechanism that leaves the loop, so the observed A->B->A transition is
    // genuinely timer-driven.
    let code_ptr = (physical_memory_offset + code_frame.start_address().as_u64()).as_mut_ptr::<u8>();
    core::ptr::write(code_ptr, 0xEB);
    core::ptr::write(code_ptr.add(1), 0xFE);
}

unsafe fn initialize_context(
    pid: Pid,
    stack: KernelStack,
    user_rip: u64,
    user_stack: u64,
    physical_memory_offset: VirtAddr,
    cs: u16,
    ss: u16,
) -> ProcessExecutionContext {
    let context_address = stack.top() - size_of::<ContextStack>() as u64;
    assert_eq!(context_address & 0xF, 0, "E2E: context stack must be 16-byte aligned");
    let last_frame = stack.frame(DEFAULT_STACK_PAGES - 1).expect("E2E: last kernel-stack frame missing");
    let last_page_base = stack.base() + (DEFAULT_STACK_PAGES as u64 - 1) * PAGE_SIZE;
    let offset = context_address - last_page_base;
    assert!(offset + size_of::<ContextStack>() as u64 <= PAGE_SIZE);
    let physical_context = last_frame.start_address().as_u64() + offset;
    let context_ptr = (physical_memory_offset + physical_context).as_mut_ptr::<ContextStack>();
    context_ptr.write(ContextStack {
        registers: SavedRegisters { r15:0, r14:0, r13:0, r12:0, r11:0, r10:0, r9:0, r8:0, rsi:0, rdi:0, rbp:0, rdx:0, rcx:0, rbx:0, rax:0 },
        iret: IretFrame {
            rip: user_rip,
            cs: cs as u64,
            rflags: 0x202,
            rsp: user_stack + PAGE_SIZE - 16,
            ss: ss as u64,
        },
    });

    // Construct the metadata object after the physical backing is initialized.
    // Validation of the actual ContextStack happens in `prepare` under the
    // corresponding CR3.
    core::mem::MaybeUninit::zeroed().assume_init()
}
