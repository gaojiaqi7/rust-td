# Copyright (c) 2020 Intel Corporation
# SPDX-License-Identifier: BSD-2-Clause-Patent

.section .text

#  switch_stack_call(
#       entry_point: usize, // rcx
#       stack_top: usize,   // rdx
#       P1: usize,          // r8
#       INIT: usize         // r9
#       INIT_SIZE: usize     // 0x28(%rsp)
#       );
.global switch_stack_call
switch_stack_call:
        movq 0x28(%rsp), %r10

        subq $32, %rdx
        movq %rdx, %rsp
        movq %rcx, %rax
        movq %r8, %rcx
        movq %r9, %rdx
        movq %r10, %r8
        call *%rax

        int $3
        jmp .
        ret
