// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![allow(unused)]
#![feature(global_asm)]
#![feature(alloc_error_handler)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(unused_imports)]

mod heap;
mod ipl;
mod memory;
mod memslice;
mod mp;
mod stack_guard;
mod tcg;

extern "win64" {
    fn switch_stack_call(entry_point: usize, stack_top: usize, P1: usize, P2: usize);
}

mod asm;

use r_efi::efi;

use r_uefi_pi::pi;
use r_uefi_pi::pi::hob;
use tdx_tdcall::tdx;
use uefi_pi::pi::hob_lib;

use rust_td_layout::build_time::*;
use rust_td_layout::runtime::*;
use rust_td_layout::RuntimeMemoryLayout;

use core::panic::PanicInfo;

use core::ffi::c_void;

use crate::memory::Memory;
use crate::memslice::SliceType;
use scroll::{Pread, Pwrite};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pwrite, Pread)]
pub struct HobTemplate {
    pub handoff_info_table: hob::HandoffInfoTable,
    pub firmware_volume: hob::FirmwareVolume,
    pub cpu: hob::Cpu,
    pub payload: hob::MemoryAllocation,
    pub page_table: hob::MemoryAllocation,
    pub stack: hob::MemoryAllocation,
    pub memory_above_1m: hob::ResourceDescription,
    pub memory_blow_1m: hob::ResourceDescription,
    pub end_off_hob: hob::Header,
}
#[cfg(not(test))]
#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    log::info!("panic ... {:?}\n", _info);
    panic!("deadloop");
}
#[cfg(not(test))]
#[alloc_error_handler]
#[allow(clippy::empty_loop)]
fn alloc_error(_info: core::alloc::Layout) -> ! {
    log::info!("alloc_error ... {:?}\n", _info);
    panic!("deadloop");
}

#[derive(Pread, Pwrite)]
struct Guid {
    data1: u32,
    data2: u32,
    data3: u32,
    data4: u32,
}

const TD_HOB_GUID: Guid = Guid {
    data1: 0xf706dd8f,
    data2: 0x11e9eebe,
    data3: 0xa7e41499,
    data4: 0x51e6daa0,
};

const EV_EFI_EVENT_BASE: u32 = 0x80000000;
const EV_EFI_HANDOFF_TABLES2: u32 = EV_EFI_EVENT_BASE + 0xB;

#[derive(Pwrite)]
struct ConfigurationTable {
    guid: Guid,
    table: u64, // should be usize, usize can't be derived by pwrite, but tdx only support 64bit
}

#[derive(Pwrite)]
struct TdxHandoffTablePointers {
    table_descripion_size: u8,
    table_description: [u8; 8],
    number_of_tables: u64,
    table_entry: [ConfigurationTable; 1],
}

fn log_hob_list(hob_list: &[u8], td_event_log: &mut tcg::TdEventLog) {
    hob_lib::dump_hob(hob_list);

    let hand_off_table_pointers = TdxHandoffTablePointers {
        table_descripion_size: 8,
        table_description: [b't', b'd', b'_', b'h', b'o', b'b', 0, 0],
        number_of_tables: 1,
        table_entry: [ConfigurationTable {
            guid: TD_HOB_GUID,
            table: hob_list as *const _ as *const c_void as u64,
        }],
    };

    let mut tdx_handofftable_pointers_buffer =
        [0u8; core::mem::size_of::<TdxHandoffTablePointers>()];
    let _writen = tdx_handofftable_pointers_buffer
        .pwrite(hand_off_table_pointers, 0)
        .unwrap();

    td_event_log.create_td_event(
        1,
        EV_EFI_HANDOFF_TABLES2,
        &tdx_handofftable_pointers_buffer,
        hob_list,
    );
}

#[cfg(not(test))]
#[no_mangle]
#[export_name = "efi_main"]
pub extern "win64" fn _start(
    boot_fv: *const c_void,
    top_of_stack: *const c_void,
    init_vp: *const c_void,
    info: usize,
) -> ! {
    let _ = tdx_logger::init();
    log::info!("Starting RUST Based TdShim boot_fv - {:p}, Top of stack - {:p}, init_vp - {:p}, info - 0x{:x} \n", boot_fv, top_of_stack, init_vp, info);
    tdx_exception::setup_exception_handlers();
    log::info!("setup_exception_handlers done\n");

    let hob_list = memslice::get_mem_slice(memslice::SliceType::ShimHob);
    let hob_size = hob_lib::get_hob_total_size(hob_list).unwrap();
    let hob_list = &hob_list[0..hob_size];
    hob_lib::dump_hob(hob_list);

    let mut td_info = tdx::TdInfoReturnData {
        gpaw: 0,
        attributes: 0,
        max_vcpus: 0,
        num_vcpus: 0,
        rsvd: [0; 3],
    };
    tdx::tdcall_get_td_info(&mut td_info);

    log::info!("gpaw - {:?}\n", td_info.gpaw);
    log::info!("num_vcpus - {:?}\n", td_info.num_vcpus);

    let memory_top = hob_lib::get_system_memory_size_below_4gb(hob_list);

    // TBD: change hardcode value
    mp::mp_accept_memory_resource_range(0x800000, memory_top - 0x800000);

    let runtime_memorey_layout = RuntimeMemoryLayout::new(memory_top);

    let memory_bottom = runtime_memorey_layout.runtime_memory_bottom;
    let td_payload_hob_base = runtime_memorey_layout.runtime_hob_base;
    let td_payload_stack_base = runtime_memorey_layout.runtime_stack_base;
    let td_event_log_base = runtime_memorey_layout.runtime_event_log_base;

    heap::init();
    paging::init();

    let mut td_event_log = tcg::TdEventLog::init(memslice::get_dynamic_mem_slice_mut(
        SliceType::EventLog,
        td_event_log_base as usize,
    ));

    log_hob_list(hob_list, &mut td_event_log);

    let fv_buffer = memslice::get_mem_slice(memslice::SliceType::ShimPayload);
    let _hob_buffer = memslice::get_mem_slice(memslice::SliceType::ShimHob);

    let _hob_header = hob::Header {
        r#type: hob::HOB_TYPE_END_OF_HOB_LIST,
        length: core::mem::size_of::<hob::Header>() as u16,
        reserved: 0,
    };

    let handoff_info_table = hob::HandoffInfoTable {
        header: hob::Header {
            r#type: hob::HOB_TYPE_HANDOFF,
            length: core::mem::size_of::<hob::HandoffInfoTable>() as u16,
            reserved: 0,
        },
        version: 9u32,
        boot_mode: pi::boot_mode::BOOT_WITH_FULL_CONFIGURATION,
        efi_memory_top: memory_top,
        efi_memory_bottom: memory_bottom,
        efi_free_memory_top: memory_top,
        efi_free_memory_bottom: memory_bottom
            + ipl::efi_page_to_size(ipl::efi_size_to_page(
                core::mem::size_of::<HobTemplate>() as u64
            )),
        efi_end_of_hob_list: td_payload_hob_base + core::mem::size_of::<HobTemplate>() as u64,
    };

    let cpu = hob::Cpu {
        header: hob::Header {
            r#type: hob::HOB_TYPE_CPU,
            length: core::mem::size_of::<hob::Cpu>() as u16,
            reserved: 0,
        },
        size_of_memory_space: ipl::cpu_get_memory_space_size(),
        size_of_io_space: 16u8,
        reserved: [0u8; 6],
    };

    let firmware_volume = hob::FirmwareVolume {
        header: hob::Header {
            r#type: hob::HOB_TYPE_FV,
            length: core::mem::size_of::<hob::FirmwareVolume>() as u16,
            reserved: 0,
        },
        base_address: TD_SHIM_PAYLOAD_BASE as u64,
        length: TD_SHIM_PAYLOAD_SIZE as u64,
    };

    const MEMORY_ALLOCATION_STACK_GUID: efi::Guid = efi::Guid::from_fields(
        0x4ED4BF27,
        0x4092,
        0x42E9,
        0x80,
        0x7D,
        &[0x52, 0x7B, 0x1D, 0x00, 0xC9, 0xBD],
    );
    let stack = hob::MemoryAllocation {
        header: hob::Header {
            r#type: hob::HOB_TYPE_MEMORY_ALLOCATION,
            length: core::mem::size_of::<hob::MemoryAllocation>() as u16,
            reserved: 0,
        },
        alloc_descriptor: hob::MemoryAllocationHeader {
            name: *MEMORY_ALLOCATION_STACK_GUID.as_bytes(),
            memory_base_address: td_payload_stack_base as u64,
            memory_length: TD_PAYLOAD_STACK_SIZE as u64
                - (stack_guard::STACK_GUARD_PAGE_SIZE + stack_guard::STACK_EXCEPTION_PAGE_SIZE),
            memory_type: efi::MemoryType::BootServicesData as u32,
            reserved: [0u8; 4],
        },
    };

    // Enable host Paging
    const PAGE_TABLE_NAME_GUID: efi::Guid = efi::Guid::from_fields(
        0xF8E21975,
        0x0899,
        0x4F58,
        0xA4,
        0xBE,
        &[0x55, 0x25, 0xA9, 0xC6, 0xD7, 0x7A],
    );

    let memory_size = ipl::get_memory_size(hob_list);
    let mut mem = Memory::new(&runtime_memorey_layout, memory_size);

    mem.setup_paging();

    let page_table = hob::MemoryAllocation {
        header: hob::Header {
            r#type: hob::HOB_TYPE_MEMORY_ALLOCATION,
            length: core::mem::size_of::<hob::MemoryAllocation>() as u16,
            reserved: 0,
        },
        alloc_descriptor: hob::MemoryAllocationHeader {
            name: *PAGE_TABLE_NAME_GUID.as_bytes(),
            memory_base_address: TD_PAYLOAD_PAGE_TABLE_BASE,
            memory_length: paging::PAGE_TABLE_SIZE as u64,
            memory_type: efi::MemoryType::BootServicesData as u32,
            reserved: [0u8; 4],
        },
    };

    let lowmemory = hob_lib::get_system_memory_size_below_4gb(hob_list);

    let memory_above_1m = hob::ResourceDescription {
        header: hob::Header {
            r#type: hob::HOB_TYPE_RESOURCE_DESCRIPTOR,
            length: core::mem::size_of::<hob::ResourceDescription>() as u16,
            reserved: 0,
        },
        owner: *efi::Guid::from_fields(
            0x4ED4BF27,
            0x4092,
            0x42E9,
            0x80,
            0x7D,
            &[0x52, 0x7B, 0x1D, 0x00, 0xC9, 0xBD],
        )
        .as_bytes(),
        resource_type: hob::RESOURCE_SYSTEM_MEMORY,
        resource_attribute: hob::RESOURCE_ATTRIBUTE_PRESENT
            | hob::RESOURCE_ATTRIBUTE_INITIALIZED
            | hob::RESOURCE_ATTRIBUTE_UNCACHEABLE
            | hob::RESOURCE_ATTRIBUTE_WRITE_COMBINEABLE
            | hob::RESOURCE_ATTRIBUTE_WRITE_THROUGH_CACHEABLE
            | hob::RESOURCE_ATTRIBUTE_WRITE_BACK_CACHEABLE
            | hob::RESOURCE_ATTRIBUTE_TESTED,
        physical_start: 0x100000u64,
        resource_length: lowmemory - 0x100000u64,
    };

    let memory_below_1m = hob::ResourceDescription {
        header: hob::Header {
            r#type: hob::HOB_TYPE_RESOURCE_DESCRIPTOR,
            length: core::mem::size_of::<hob::ResourceDescription>() as u16,
            reserved: 0,
        },
        owner: *efi::Guid::from_fields(
            0x4ED4BF27,
            0x4092,
            0x42E9,
            0x80,
            0x7D,
            &[0x52, 0x7B, 0x1D, 0x00, 0xC9, 0xBD],
        )
        .as_bytes(),
        resource_type: hob::RESOURCE_SYSTEM_MEMORY,
        resource_attribute: hob::RESOURCE_ATTRIBUTE_PRESENT
            | hob::RESOURCE_ATTRIBUTE_INITIALIZED
            | hob::RESOURCE_ATTRIBUTE_UNCACHEABLE
            | hob::RESOURCE_ATTRIBUTE_WRITE_COMBINEABLE
            | hob::RESOURCE_ATTRIBUTE_WRITE_THROUGH_CACHEABLE
            | hob::RESOURCE_ATTRIBUTE_WRITE_BACK_CACHEABLE
            | hob::RESOURCE_ATTRIBUTE_TESTED,
        physical_start: 0u64,
        resource_length: 0x80000u64 + 0x20000u64,
    };

    let (entry, basefw, basefwsize) =
        ipl::find_and_report_entry_point(&mut mem, fv_buffer).unwrap();
    let entry = entry as usize;

    const PAYLOAD_NAME_GUID: efi::Guid = efi::Guid::from_fields(
        0x6948d4a,
        0xd359,
        0x4721,
        0xad,
        0xf6,
        &[0x52, 0x25, 0x48, 0x5a, 0x6a, 0x3a],
    );

    let payload = hob::MemoryAllocation {
        header: hob::Header {
            r#type: hob::HOB_TYPE_MEMORY_ALLOCATION,
            length: core::mem::size_of::<hob::MemoryAllocation>() as u16,
            reserved: 0,
        },
        alloc_descriptor: hob::MemoryAllocationHeader {
            name: *PAYLOAD_NAME_GUID.as_bytes(),
            memory_base_address: basefw,
            memory_length: ipl::efi_page_to_size(ipl::efi_size_to_page(basefwsize)),
            memory_type: efi::MemoryType::BootServicesCode as u32,
            reserved: [0u8; 4],
        },
    };

    let hob_template = HobTemplate {
        handoff_info_table,
        firmware_volume,
        cpu,
        payload,
        page_table,
        stack,
        memory_above_1m,
        memory_blow_1m: memory_below_1m,
        end_off_hob: hob::Header {
            r#type: hob::HOB_TYPE_END_OF_HOB_LIST,
            length: core::mem::size_of::<hob::Header>() as u16,
            reserved: 0,
        },
    };

    let hob_slice =
        memslice::get_dynamic_mem_slice_mut(SliceType::PayloadHob, td_payload_hob_base as usize);
    let _res = hob_slice.pwrite(hob_template, 0);

    stack_guard::stack_guard_enable(&mut mem);

    let stack_top = (td_payload_stack_base + TD_PAYLOAD_STACK_SIZE as u64) as usize;
    log::info!(
        " start launching payload {:p} and switch stack {:p}...\n",
        entry as *const usize,
        stack_top as *const usize
    );

    unsafe {
        switch_stack_call(entry, stack_top, td_payload_hob_base as usize, 0);
    }

    panic!("deadloop");
}
