// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use paging::PHYS_VIRT_OFFSET;
use rust_td_layout::RuntimeMemoryLayout;

use log::*;
use x86_64::{
    structures::paging::PageTableFlags as Flags,
    structures::paging::{OffsetPageTable, PageTable},
    PhysAddr, VirtAddr,
};

extern "win64" {
    fn asm_read_msr64 (index: u32) -> u64;
    fn asm_write_msr64 (index: u32, value: u64) -> u64;
}

const EXTENDED_FUNCTION_INFO: u32 = 0x80000000;
const EXTENDED_PROCESSOR_INFO: u32 = 0x80000001;

/// page_table_memory_base: page_table_memory_base
/// system_memory_size
pub fn setup_paging(layout: &RuntimeMemoryLayout, memory_end: u64) {
    let runtime_page_table_base = layout.runtime_page_table_base;
    let page_table_size = layout.runtime_payload_base - layout.runtime_page_table_base;
    info!(
        "Frame allocator init done: {:#x?}\n",
        runtime_page_table_base..(runtime_page_table_base + page_table_size)
    );

    let mut pt = unsafe {
        OffsetPageTable::new(
            &mut *(runtime_page_table_base as *mut PageTable),
            VirtAddr::new(PHYS_VIRT_OFFSET as u64),
        )
    };

    let shared_page_flag = tdx_tdcall::tdx::td_shared_page_mask();
    let flags = Flags::PRESENT | Flags::WRITABLE;
    let with_s_flags = unsafe { Flags::from_bits_unchecked(flags.bits() | shared_page_flag) };
    log::info!(
        "shared page flags - smask: {:#x} flags: {:?}\n",
        shared_page_flag,
        with_s_flags
    );

    // create to runtime_page_table_base
    paging::paging::create_mapping(
        &mut pt,
        PhysAddr::new(0),
        VirtAddr::new(0),
        layout.runtime_dma_base,
    );

    // runtime_dma_base..runtime_heap_base
    paging::paging::create_mapping_with_flags(
        &mut pt,
        PhysAddr::new(layout.runtime_dma_base),
        VirtAddr::new(layout.runtime_dma_base),
        layout.runtime_heap_base - layout.runtime_dma_base,
        with_s_flags,
    );

    // runtime_heap_base..memory_end
    paging::paging::create_mapping(
        &mut pt,
        PhysAddr::new(layout.runtime_heap_base),
        VirtAddr::new(layout.runtime_heap_base),
        memory_end - layout.runtime_heap_base,
    );

    //
    // enable the execute disable.
    //
    if is_execute_disable_bit_available() {
        //
        // For now EFER cannot be set in TDX, but the NX is enabled by default.
        //
        // enable_execute_disable_bit();
    }

    paging::paging::cr3_write();
}

fn is_execute_disable_bit_available () -> bool {

    let cpuid = unsafe { core::arch::x86_64::__cpuid(EXTENDED_FUNCTION_INFO) };

    if cpuid.eax >= EXTENDED_PROCESSOR_INFO {
        let cpuid = unsafe { core::arch::x86_64::__cpuid(EXTENDED_PROCESSOR_INFO) };
        if (cpuid.edx & 0x00100000) != 0 {
            //
            // Bit 20: Execute Disable Bit available.
            //
            return true;
        }
    }
    false
}

//
//  Enable Execute Disable Bit.
//
fn enable_execute_disable_bit () {
  let mut msr: u64;

  unsafe { msr = asm_read_msr64 (0xC0000080);}
  msr |= 0x800;
  unsafe { asm_write_msr64 (0xC0000080, msr);}
}
