// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

pub fn clear_8259_interupt() {
    //
    // Clear 8259 interrupt
    //
    #[cfg(feature = "iocall")]
    unsafe {
        x86::io::outb(0x21u16, 0xffu8);
        x86::io::outb(0xA1u16, 0xffu8);
    }
    #[cfg(feature = "tdcall")]
    {
        tdx_tdcall::tdx::tdvmcall_io_write_8(0x21u16, 0xffu8);
        tdx_tdcall::tdx::tdvmcall_io_write_8(0xA1u16, 0xff);
    }
}
pub fn disable_a20_mask() {
    //
    // Disable A20 Mask
    //
    #[cfg(feature = "iocall")]
    unsafe {
        let res = x86::io::inb(0x92u16);
        x86::io::outb(0x92u16, res | 0b10 as u8);
    }

    #[cfg(feature = "tdcall")]
    {
        let res = tdx_tdcall::tdx::tdvmcall_io_read_8(0x92u16);
        tdx_tdcall::tdx::tdvmcall_io_write_8(0x92u16, res | 0b10 as u8);
    }
}
