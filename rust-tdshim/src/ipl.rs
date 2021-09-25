// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use r_uefi_pi::pi::fv;
use uefi_pi::pi::fv_lib;

use elf_loader::elf;
use pe_loader::pe;

use crate::memslice;
use crate::Memory;

const EXTENDED_FUNCTION_INFO: u32 = 0x80000000;
const VIRT_PHYS_MEM_SIZES: u32 = 0x80000008;

pub const SIZE_4KB: u64 = 0x00001000u64;
// pub const SIZE_2MB: u64 = 0x00200000u64;

// pub const LOCAL_APIC_MODE_XAPIC: u64 = 0x1;
// pub const LOCAL_APIC_MODE_X2APIC: u64 = 0x2;

pub fn efi_size_to_page(size: u64) -> u64 {
    (size + SIZE_4KB - 1) / SIZE_4KB
}

pub fn efi_page_to_size(page: u64) -> u64 {
    page * SIZE_4KB
}

pub fn find_and_report_entry_point(mem: &mut Memory, fv_buffer: &[u8]) -> Option<(u64, u64, u64)> {
    let image_buffer =
        fv_lib::get_image_from_fv(fv_buffer, fv::FV_FILETYPE_DXE_CORE, fv::SECTION_PE32).unwrap();

    let loaded_buffer = memslice::get_mem_slice_mut(memslice::SliceType::TdPayloadSlice);

    let res = if elf::is_elf(image_buffer) {
        let (image_entry, image_base, image_size, program_headers) = elf::relocate_elf(image_buffer, loaded_buffer);
        for ph in program_headers {
            if !ph.is_executable() {
                mem.set_nx_bit(ph.p_vaddr + loaded_buffer.as_ptr() as u64, ph.p_filesz);
            }
            if !ph.is_write() {
                log::info!("WP in elf: {:x}\n", ph.p_vaddr + loaded_buffer.as_ptr() as u64);
                mem.set_write_protect(ph.p_vaddr + loaded_buffer.as_ptr() as u64, ph.p_filesz);
            }
        }
        (image_entry, image_base, image_size)
    } else if pe::is_pe(image_buffer) {
        let (image_entry, image_base, image_size, section_table) =
            pe::relocate_pe_mem(image_buffer, loaded_buffer);
        for sc in section_table {
            if !sc.is_executable()  {
                mem.set_nx_bit(sc.vaddr + loaded_buffer.as_ptr() as u64, sc.size);
            }
            if !sc.is_write() {
                mem.set_write_protect(sc.vaddr + loaded_buffer.as_ptr() as u64, sc.size);
            }
        }
        (image_entry, image_base, image_size)
    } else {
        return None;
    };
    log::info!(
        "image_entry: {:x}, image_base: {:x}, image_size: {:x}\n",
        res.0,
        res.1,
        res.2
    );
    Some(res)
}

/// CpuGetMemorySpaceSize returns the maximum physical memory addressability of the processor.
pub fn cpu_get_memory_space_size() -> u8 {
    let cpuid = unsafe { core::arch::x86_64::__cpuid(EXTENDED_FUNCTION_INFO) };

    let size_of_mem_space = if cpuid.eax >= VIRT_PHYS_MEM_SIZES {
        let cpuid = unsafe { core::arch::x86_64::__cpuid(VIRT_PHYS_MEM_SIZES) };
        // CPUID.80000008H:EAX[bits 7-0]: the size of the physical address range
        cpuid.eax as u8
    } else {
        // fallback value according to edk2 core
        36
    };

    log::info!(
        "Maximum physical memory addressability of the processor - {}\n",
        size_of_mem_space
    );

    size_of_mem_space
}

pub fn get_memory_size(hob: &[u8]) -> u64 {
    let cpu_men_space_size = cpu_get_memory_space_size() as u32;
    let cpu_memory_size = 2u64.pow(cpu_men_space_size);
    let hob_memory_size = uefi_pi::hob_lib::get_total_memory_top(hob);
    let mem_size = core::cmp::min(cpu_memory_size, hob_memory_size);
    log::info!("memory_size: 0x{:x}\n", mem_size);
    mem_size
}
