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

#[allow(unused)]
mod platform;
mod virtio_impl;
mod vsock_impl;

mod client;
mod server;

mod vsock_lib;

#[link(name = "main")]
extern "C" {
    fn server_entry() -> i32;
    fn client_entry() -> i32;
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

fn init_heap(heap_start: usize, heap_size: usize) {
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

    init_heap(
        runtime_memory_layout.runtime_heap_base as usize,
        TD_PAYLOAD_HEAP_SIZE as usize,
    );

    fw_pci::clear_8259_interupt();
    fw_pci::disable_a20_mask();
    fw_pci::initialize_acpi_pm();
    fw_pci::pci_ex_bar_initialization();

    platform::init();

    fw_pci::print_bus();

    virtio_impl::init(
        runtime_memory_layout.runtime_dma_base as usize,
        TD_PAYLOAD_DMA_SIZE,
    );

    vsock_impl::init_vsock_device();

    // client::test_client();
    let mut result;
    unsafe {
        result = client_entry();
    }
    log::debug!("Client example done: {}\n", result);

    // server::test_server();
    unsafe {
        result = server_entry();
    }

    log::debug!("Server Example done: {}\n", result);

    loop {}
}
