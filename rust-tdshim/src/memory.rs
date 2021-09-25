// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use rust_td_layout::{RuntimeMemoryLayout, runtime::TD_PAYLOAD_EVENT_LOG_SIZE};

use log::*;
use x86_64::{PhysAddr, VirtAddr, structures::paging::PageTableFlags as Flags, structures::paging::{OffsetPageTable, PageTable}};

extern "win64" {
    fn asm_read_msr64 (index: u32) -> u64;
    fn asm_write_msr64 (index: u32, value: u64) -> u64;
}

const EXTENDED_FUNCTION_INFO: u32 = 0x80000000;
const EXTENDED_PROCESSOR_INFO: u32 = 0x80000001;

pub struct Memory<'a> {
    pt: OffsetPageTable<'a>,
    layout: &'a RuntimeMemoryLayout,
    memory_size: u64,
}

impl<'a> Memory<'a> {
    pub fn new (layout: &RuntimeMemoryLayout, memory_size: u64) -> Memory {
        Memory {
            pt: unsafe {
                OffsetPageTable::new(
                    &mut *(layout.runtime_page_table_base as *mut PageTable),
                    VirtAddr::new(paging::PHYS_VIRT_OFFSET as u64),
                )
            },
            layout,
            memory_size,
        }
    }

    /// page_table_memory_base: page_table_memory_base
    /// system_memory_size
    pub fn setup_paging(&mut self) {

        let shared_page_flag = tdx_tdcall::tdx::td_shared_page_mask();
        let flags = Flags::PRESENT | Flags::WRITABLE;
        let with_s_flags = unsafe { Flags::from_bits_unchecked(flags.bits() | shared_page_flag) };
        let with_nx_flags = flags | Flags::NO_EXECUTE;
        log::info!(
            "shared page flags - smask: {:#x} flags: {:?}\n",
            shared_page_flag,
            with_s_flags
        );

        // 0..runtime_payload_base
        paging::paging::create_mapping(
            &mut self.pt,
            PhysAddr::new(0),
            VirtAddr::new(0),
            self.layout.runtime_dma_base - 0,
        );

        // runtime_dma_base..runtime_heap_base with Shared flag
        paging::paging::create_mapping_with_flags(
            &mut self.pt,
            PhysAddr::new(self.layout.runtime_dma_base),
            VirtAddr::new(self.layout.runtime_dma_base),
            self.layout.runtime_heap_base - self.layout.runtime_dma_base,
            with_s_flags | with_nx_flags,
        );

        let runtime_memory_top = self.layout.runtime_event_log_base + TD_PAYLOAD_EVENT_LOG_SIZE as u64;
        // runtime_heap_base..memory_top with NX flag
        paging::paging::create_mapping_with_flags(
            &mut self.pt,
            PhysAddr::new(self.layout.runtime_heap_base),
            VirtAddr::new(self.layout.runtime_heap_base),
            runtime_memory_top - self.layout.runtime_heap_base,
            with_nx_flags,
        );

        // runtime_memory_top..memory_size (end)
        paging::paging::create_mapping(
            &mut self.pt,
            PhysAddr::new(runtime_memory_top),
            VirtAddr::new(runtime_memory_top),
            self.memory_size - runtime_memory_top,
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
