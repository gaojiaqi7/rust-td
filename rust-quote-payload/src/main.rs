// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

#![feature(alloc_error_handler)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(unused_imports))]

extern crate alloc;
use uefi_pi::pi::hob_lib;

use rust_td_layout::runtime::*;
use rust_td_layout::RuntimeMemoryLayout;

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

use core::panic::PanicInfo;

use core::ffi::c_void;

use alloc::boxed::Box;
use tdx_tdcall::tdreport;

#[allow(unused)]
mod platform;
mod virtio_impl;
mod vsock_impl;

mod client;
mod server;

mod vsock_lib;

const QUOTE_ATTESTATION_HEAP_SIZE: usize = 0x100000;
const TD_QUOTE_SIZE: usize = 5000;
const TD_REPORT_VERIFY_SIZE: usize = 1024;

// #[link(name = "main")]
// extern "C" {
//     fn server_entry() -> i32;
//     fn client_entry() -> i32;
// }

#[link(name = "migtd_attest")]
extern "C" {
    fn get_quote(p_tdx_report: *const c_void, tdx_report_size: i32, p_quote: *mut c_void, p_quote_size: *mut i32) -> bool;
    fn verify_quote_integrity(p_quote: *const c_void, quote_size: i32, p_tdx_report: *mut c_void, p_tdx_report_size: *mut i32) -> bool;
    fn init_heap(p_td_heap_base: *mut c_void, td_heap_size: i32);
}

#[cfg(not(test))]
#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    log::info!("panic ... {:?}\n", _info);
    loop {}
}

#[cfg(not(test))]
#[alloc_error_handler]
#[allow(clippy::empty_loop)]
fn alloc_error(_info: core::alloc::Layout) -> ! {
    log::info!("alloc_error ... {:?}\n", _info);
    loop {}
}

fn init_payload_heap(heap_start: usize, heap_size: usize) {
    unsafe {
        ALLOCATOR.lock().init(heap_start, heap_size);
    }
    log::info!(
        "heap init: {:#010x} - {:#010x}\n",
        heap_start,
        heap_start + heap_size
    );
}

#[no_mangle]
#[cfg_attr(target_os = "uefi", export_name = "efi_main")]
pub extern "win64" fn _start(hob: *const c_void) -> ! {
    let _ = tdx_logger::init();
    log::info!("Starting rust-td-payload hob - {:p}\n", hob);

    tdx_exception::setup_exception_handlers();
    log::info!("setup_exception_handlers done\n");

    let hob =
        unsafe { core::slice::from_raw_parts(hob as *const u8, TD_PAYLOAD_HOB_SIZE as usize) };
    let hob_size = hob_lib::get_hob_total_size(hob).unwrap();
    let hob = &hob[..hob_size];
    hob_lib::dump_hob(hob);

    let runtime_memory_layout =
        RuntimeMemoryLayout::new(hob_lib::get_system_memory_size_below_4gb(hob));

    init_payload_heap(
        runtime_memory_layout.runtime_heap_base as usize,
        TD_PAYLOAD_HEAP_SIZE as usize,
    );

    fw_pci::clear_8259_interupt();
    fw_pci::disable_a20_mask();
    fw_pci::initialize_acpi_pm();
    fw_pci::pci_ex_bar_initialization();

    platform::init();

    fw_pci::print_bus();

    log::info!("runtime_memory_layout.runtime_dma_base is {:X}\n", runtime_memory_layout.runtime_dma_base);
    virtio_impl::init(
        runtime_memory_layout.runtime_dma_base as usize,
        TD_PAYLOAD_DMA_SIZE,
    );

    vsock_impl::init_vsock_device();

    let mut heap: Box<[u8; QUOTE_ATTESTATION_HEAP_SIZE]> =Box::new([0; QUOTE_ATTESTATION_HEAP_SIZE]);

    let additional_data: [u8;tdreport::TD_REPORT_ADDITIONAL_DATA_SIZE] = [0;tdreport::TD_REPORT_ADDITIONAL_DATA_SIZE];

    let td_report = tdreport::tdcall_report(&additional_data).to_buff();
    let mut quote: Box<[u8; TD_QUOTE_SIZE]> =Box::new([0; TD_QUOTE_SIZE]);
    let mut quote_size: i32 = TD_QUOTE_SIZE as i32;

    let mut td_report_verify: Box<[u8; TD_REPORT_VERIFY_SIZE]> = Box::new([0; TD_REPORT_VERIFY_SIZE]);
    let mut report_verify_size = TD_REPORT_VERIFY_SIZE as i32;

    let mut result;

    unsafe {
        init_heap (heap.as_mut_ptr() as *mut c_void, QUOTE_ATTESTATION_HEAP_SIZE as i32);

        result = get_quote (td_report.as_ptr() as *mut c_void, tdreport::TD_REPORT_SIZE as i32, quote.as_mut_ptr() as *mut c_void, &mut quote_size as *mut i32);
        log::info!("get_quote result is {}\n", result);

        result = verify_quote_integrity(quote.as_ptr() as *mut c_void, quote_size, td_report_verify.as_mut_ptr() as *mut c_void, &mut report_verify_size as *mut i32);
        log::info!("verify_quote_integrity result is {}\n", result);
    }

    loop {}
}