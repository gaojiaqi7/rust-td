// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use bitmap_allocator::{BitAlloc, BitAlloc4K};
use log::trace;
use spin::Mutex;

static DMA_ALLOCATOR: Mutex<DmaAlloc> = Mutex::new(DmaAlloc::empty());

pub fn init(dma_base: usize, dma_size: usize) {
    log::info!("init dma - {:#x} - {:#x}\n", dma_base, dma_base + dma_size);
    init_dma(dma_base, dma_size);
}

fn init_dma(dma_base: usize, dma_size: usize) {
    // set page table flags TBD:
    *DMA_ALLOCATOR.lock() = DmaAlloc::new(dma_base as usize, dma_size);
}

#[no_mangle]
extern "C" fn virtio_dma_alloc(blocks: usize) -> PhysAddr {
    let paddr = unsafe { DMA_ALLOCATOR.lock().alloc_contiguous(blocks, 0) }.unwrap_or(0);
    paddr
}

#[no_mangle]
extern "C" fn virtio_dma_dealloc(paddr: PhysAddr, blocks: usize) -> i32 {
    let _ = unsafe { DMA_ALLOCATOR.lock().dealloc_contiguous(paddr, blocks) };
    0
}

#[no_mangle]
extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    paddr
}

#[no_mangle]
extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    vaddr
}

type VirtAddr = usize;
type PhysAddr = usize;

struct DmaAlloc {
    base: usize,
    inner: BitAlloc4K,
}

const BLOCK_SIZE: usize = 4096;

impl Default for DmaAlloc {
    fn default() -> Self {
        Self {
            base: 0,
            inner: BitAlloc4K::DEFAULT,
        }
    }
}

impl DmaAlloc {
    pub fn new(base: usize, length: usize) -> Self {
        let mut inner = BitAlloc4K::DEFAULT;
        let blocks = length / BLOCK_SIZE;
        assert!(blocks <= BitAlloc4K::CAP);
        inner.insert(0..blocks);
        DmaAlloc { base, inner }
    }

    const fn empty() -> Self {
        Self {
            base: 0,
            inner: BitAlloc4K::DEFAULT,
        }
    }

    /// # Safety
    ///
    /// This function is unsafe because manual deallocation is needed.
    #[allow(unused)]
    pub unsafe fn alloc(&mut self) -> Option<usize> {
        let ret = self.inner.alloc().map(|idx| idx * 4096 + self.base);
        trace!("Alloc DMA block: {:x?}\n", ret);
        ret
    }

    /// # Safety
    ///
    /// This function is unsafe because manual deallocation is needed.
    pub unsafe fn alloc_contiguous(
        &mut self,
        block_count: usize,
        align_log2: usize,
    ) -> Option<usize> {
        let ret = self
            .inner
            .alloc_contiguous(block_count, align_log2)
            .map(|idx| idx * BLOCK_SIZE + self.base);
        trace!(
            "Allocate {} DMA blocks with alignment {}: {:x?}\n",
            block_count,
            1 << align_log2,
            ret
        );
        ret
    }

    /// # Safety
    ///
    /// This function is unsafe because the DMA must have been allocated.
    #[allow(unused)]
    pub unsafe fn dealloc(&mut self, target: usize) {
        trace!("Deallocate DMA block: {:x}\n", target);
        self.inner.dealloc((target - self.base) / BLOCK_SIZE)
    }

    /// # Safety
    ///
    /// This function is unsafe because the DMA must have been allocated.
    unsafe fn dealloc_contiguous(&mut self, target: usize, block_count: usize) {
        trace!("Deallocate {} DMA blocks: {:x}\n", block_count, target);
        let start_idx = (target - self.base) / BLOCK_SIZE;
        for i in start_idx..start_idx + block_count {
            self.inner.dealloc(i)
        }
    }
}
