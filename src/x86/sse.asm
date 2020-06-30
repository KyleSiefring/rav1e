; Copyright (c) 2020, The rav1e contributors. All rights reserved
;
; This source code is subject to the terms of the BSD 2 Clause License and
; the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
; was not distributed with this source code in the LICENSE file, you can
; obtain it at www.aomedia.org/license/software. If the Alliance for Open
; Media Patent License 1.0 was not distributed with this source code in the
; PATENTS file, you can obtain it at www.aomedia.org/license/patent.

%include "ext/x86/x86inc.asm"

SECTION_RODATA 32
addsub: times 16 db 1, -1
rounding: dq 0x800

SECTION .text

%define m(x) mangle(private_prefix %+ _ %+ x %+ SUFFIX)

; TODO: Move all weighting arith to one place to make it easier to change.

INIT_XMM avx2

; Use scale stride to store src_stride3
; TODO: prevent loading of scale_stride by assembly
cglobal weighted_sse_4x4, 6, 7, 5, \
        src, src_stride, dst, dst_stride, scale, src_stride3, \
        dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    movq                m0, [addsub]
    movd                m1, [srcq]
    movd                m2, [dstq]
    punpcklbw           m1, m2
    movd                m2, [srcq+src_strideq]
    movd                m3, [dstq+dst_strideq]
    punpcklbw           m2, m3
    pmaddubsw           m1, m0
    pmaddubsw           m2, m0
    pmaddwd             m1, m1
    pmaddwd             m2, m2
    paddd               m1, m2
    movd                m2, [srcq+src_strideq*2]
    movd                m3, [dstq+dst_strideq*2]
    punpcklbw           m2, m3
    movd                m3, [srcq+src_stride3q]
    movd                m4, [dstq+dst_stride3q]
    punpcklbw           m3, m4
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    paddd               m1, m2

    pshuflw             m0, m1, q3232
    paddd               m0, m1
    movd               eax, m0

    mov             scaled, [scaleq]
    mul             scaleq
    add                rax, 0x800
    shr                rax, 12
    RET

cglobal weighted_sse_4x8, 6, 6, 6, \
        src, src_stride, dst, dst_stride, scale, scale_stride
    mova                m0, [addsub]
    movd                m1, [srcq]
    movd                m2, [srcq+src_strideq*4]
    punpckldq           m1, m2
    movd                m2, [dstq]
    movd                m3, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m2, m3
    punpcklbw           m1, m2
    movd                m2, [srcq]
    movd                m3, [srcq+src_strideq*4]
    punpckldq           m2, m3
    movd                m3, [dstq]
    movd                m4, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m3, m4
    punpcklbw           m2, m3
    pmaddubsw           m1, m0
    pmaddubsw           m2, m0
    pmaddwd             m1, m1
    pmaddwd             m2, m2
    paddd               m1, m2
    movd                m2, [srcq]
    movd                m3, [srcq+src_strideq*4]
    punpckldq           m2, m3
    movd                m3, [dstq]
    movd                m4, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m3, m4
    punpcklbw           m2, m3
    movd                m3, [srcq]
    movd                m4, [srcq+src_strideq*4]
    punpckldq           m3, m4
    movd                m4, [dstq]
    movd                m5, [dstq+dst_strideq*4]
    punpckldq           m4, m5
    punpcklbw           m3, m4
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    paddd               m1, m2

    pshufd              m2, m1, q3311
    paddd               m1, m2

    movd                m2, [scaleq]
    movd                m3, [scaleq+scale_strideq]
    punpcklqdq          m2, m3
    pmuludq             m1, m2
    vpbroadcastq        m2, [rounding]
    paddq               m1, m2
    psrlq               m1, 12

    pshufd              m0, m1, q3232
    paddq               m1, m0
    movq               rax, m1
    RET

cglobal weighted_sse_4x16, 6, 7, 7, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        h
    mova                m0, [addsub]
    mov                 hd, 1
    pxor                m6, m6
.loop
    movd                m1, [srcq]
    movd                m2, [srcq+src_strideq*4]
    punpckldq           m1, m2
    movd                m2, [dstq]
    movd                m3, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m2, m3
    punpcklbw           m1, m2
    movd                m2, [srcq]
    movd                m3, [srcq+src_strideq*4]
    punpckldq           m2, m3
    movd                m3, [dstq]
    movd                m4, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m3, m4
    punpcklbw           m2, m3
    pmaddubsw           m1, m0
    pmaddubsw           m2, m0
    pmaddwd             m1, m1
    pmaddwd             m2, m2
    paddd               m1, m2
    movd                m2, [srcq]
    movd                m3, [srcq+src_strideq*4]
    punpckldq           m2, m3
    movd                m3, [dstq]
    movd                m4, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m3, m4
    punpcklbw           m2, m3
    movd                m3, [srcq]
    movd                m4, [srcq+src_strideq*4]
    punpckldq           m3, m4
    movd                m4, [dstq]
    movd                m5, [dstq+dst_strideq*4]
    add               srcq, src_strideq
    add               dstq, dst_strideq
    punpckldq           m4, m5
    punpcklbw           m3, m4
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    paddd               m1, m2

    pshufd              m2, m1, q3311
    paddd               m1, m2

    movd                m2, [scaleq]
    movd                m3, [scaleq+scale_strideq]
    punpcklqdq          m2, m3
    pmuludq             m1, m2
    vpbroadcastq        m2, [rounding]
    paddq               m1, m2
    psrlq               m1, 12
    paddq               m6, m1
    lea             scaleq, [scaleq+scale_strideq*2]
    ; Already incremented be stride 4 times, but must go up 4 more to get to 8
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]

    dec                 hq
    jge .loop

    pshufd              m0, m6, q3232
    paddq               m6, m0
    movq               rax, m6
    RET

%macro WEIGHTED_SSE 2
%if %1 == 4
%elif %1 == 8
%if %2 == 4
; Overwrite scale_stride since it isn't used.
cglobal weighted_sse_%1x%2, 6, 7, 4, \
        src, src_stride, dst, dst_stride, scale, \
        src_stride3, dst_stride3
%else
cglobal weighted_sse_%1x%2, 6, 9, 5, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3, h
%endif
%elif %1 == 16
%if %2 == 4
; Overwrite scale_stride since it isn't used.
; TODO: Does scale_stride have to be popped from the stack?
cglobal weighted_sse_%1x%2, 6, 7, 4, \
        src, src_stride, dst, dst_stride, scale, \
        src_stride3, dst_stride3
%else
cglobal weighted_sse_%1x%2, 6, 9, 5, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3, h
%endif
%elif %1 == 32
cglobal weighted_sse_%1x%2, 6, 9, 9, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3, h
%else ; > 32
cglobal weighted_sse_%1x%2, 6, 10, 9, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3, h, w
%endif
%if %1 == 8
    mova                m0, [addsub]
    %define LOAD_SCALES LOAD_SCALES_8
    %define SSE_KERNEL SSE_KERNEL_8
    %define sum          1
    %define tmp          0
%if %2 == 4
    %define SSE_SCALE SSE_SCALE_ONCE
    %define working_regs 0, 1, 2, 3
%elif
    %define SSE_SCALE SSE_SCALE_ADD
    %define working_regs 0, 2, 3, 4
%endif
%endif

%if %1 == 16
    %define LOAD_SCALES LOAD_SCALES_16
    %define SSE_KERNEL SSE_KERNEL_16
    %define sum          0
    %define tmp          1
%if %2 == 4
    %define SSE_SCALE SSE_SCALE_ONCE
    %define working_regs 0, 1, 2, 3
%elif
    %define SSE_SCALE SSE_SCALE_ADD
    %define working_regs 1, 2, 3, 4
%endif
%endif

; Default the kernel width to the width of this function
%define kernel_width %1
%if %1 >= 32
    mova                m0, [addsub]
    %define sum          1
    %define tmp          2
    %define use_func
    %define kernel_width 32
%endif

    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
%if %2 != 4
    pxor           m%[sum], m%[sum]
    mov                 hd, %2/4-1
.loop:
%endif

%if %1 > kernel_width
    mov                 wd, %1/kernel_width-1
.loop_horiz:
%endif

%ifdef use_func
    call m(weighted_sse_%[kernel_width]x4_internal)
%else
    SSE_KERNEL sum, working_regs
%endif

%if %1 > kernel_width
    add             scaleq, kernel_width*4/4
    add               srcq, kernel_width
    add               dstq, kernel_width
    dec                 wq
    jge .loop_horiz
%endif

%if %2 != 4
%if %1 > kernel_width
    ; src/dst is incremented to width. To quickly move down 4 rows, 4 times
    ; stride minus width is used.
    lea               srcq, [srcq+src_strideq*4 - %1]
    lea               dstq, [dstq+dst_strideq*4 - %1]
    ; The behaviour for scale is similar
    lea             scaleq, [scaleq+scale_strideq - %1*4/4]
%elif
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    add             scaleq, scale_strideq
%endif
    dec                 hq
    jge .loop
%endif

%if mmsize == 16
    pshufd         m%[tmp], m%[sum], q3232
    paddq          m%[sum], m%[tmp]
    movq               rax, m%[sum]
%elif mmsize == 32
    vextracti128  xm%[tmp], m%[sum], 1
    paddq         xm%[sum], xm%[tmp]
    pshufd        xm%[tmp], xm%[sum], q3232
    paddq         xm%[sum], xm%[tmp]
    movq               rax, xm%[sum]
%endif

    ; Apply rounding outside simd for 4x4
%if %1 == 4 && %2 == 4
    mov             scaled, [scaleq]
    mul             scaleq
    ; TODO: Alter rust source so that rounding is always done at the end (i.e.
    ; only do it once)
    add                rax, 0x800
    shr                rax, 12
%endif
    RET

    ; Shared between inline and function call version
    %undef sum, tmp, kernel_width
    ; Inline defines
    %undef working_regs, LOAD_SCALES, SSE_KERNEL, SSE_SCALE
    ; function defines
    %undef use_func, func_width
%endmacro

%macro SSE_SCALE_ONCE 3
    pshufd             m%3, m%2, q3311
    paddd              m%2, m%3

    LOAD_SCALES %3

    pmuludq            m%2, m%3
    vpbroadcastq       m%3, [rounding]
    paddq              m%2, m%3
    psrlq              m%2, 12
%endmacro

%macro SSE_SCALE_ADD 3
    pshufd             m%3, m%2, q3311
    paddd              m%2, m%3

    LOAD_SCALES %3

    pmuludq            m%2, m%3
    vpbroadcastq       m%3, [rounding]
    paddq              m%2, m%3
    psrlq              m%2, 12

    paddq              m%1, m%2
%endmacro



; ===SSE_SETUP===
;   Defines variables and sets up a select few registers.
; sum: vector sum (Optional; will check if defined)
; working_mreg: working registers to provide the kernel
; res: vector results from kernel
; tmp: spare vector register to use
; ... others: implementation defined

%macro LOAD_SCALES_8 1
    pmovzxdq           m%1, [scaleq]
%endmacro

%macro SSE_KERNEL_8 5
    movq               m%3, [srcq]
    punpcklbw          m%3, [dstq]
    movq               m%4, [srcq+src_strideq]
    punpcklbw          m%4, [dstq+dst_strideq]
    pmaddubsw          m%3, m%2
    pmaddubsw          m%4, m%2
    pmaddwd            m%3, m%3
    pmaddwd            m%4, m%4
    paddd              m%3, m%4
    movq               m%4, [srcq+src_strideq*2]
    punpcklbw          m%4, [dstq+dst_strideq*2]
    movq               m%5, [srcq+src_stride3q]
    punpcklbw          m%5, [dstq+dst_stride3q]
    pmaddubsw          m%4, m%2
    pmaddubsw          m%5, m%2
    pmaddwd            m%4, m%4
    pmaddwd            m%5, m%5
    paddd              m%4, m%5
    paddd              m%3, m%4

    ; global sum, local sum, and tmp register
    SSE_SCALE %1, %3, %4
%endmacro

%macro LOAD_SCALES_16 1
    mova              xm%1, [scaleq]
    pmovzxdq           m%1, xm%1
%endmacro

%macro SSE_KERNEL_16 5
    pmovzxbw           m%2, [srcq]
    pmovzxbw           m%3, [dstq]
    psubw              m%2, m%3
    pmaddwd            m%2, m%2
    pmovzxbw           m%3, [srcq+src_strideq]
    pmovzxbw           m%4, [dstq+dst_strideq]
    psubw              m%3, m%4
    pmaddwd            m%3, m%3
    paddd              m%2, m%3
    pmovzxbw           m%3, [srcq+src_strideq*2]
    pmovzxbw           m%4, [dstq+dst_strideq*2]
    psubw              m%3, m%4
    pmaddwd            m%3, m%3
    pmovzxbw           m%4, [srcq+src_stride3q]
    pmovzxbw           m%5, [dstq+dst_stride3q]
    psubw              m%4, m%5
    pmaddwd            m%4, m%4
    paddd              m%3, m%4
    paddd              m%2, m%3

    SSE_SCALE %1, %2, %3
%endmacro

; FIXME: Doesn't need avx2
INIT_XMM avx2
WEIGHTED_SSE 8, 4
WEIGHTED_SSE 8, 8
WEIGHTED_SSE 8, 16
WEIGHTED_SSE 8, 32

INIT_YMM avx2
WEIGHTED_SSE 16, 4
WEIGHTED_SSE 16, 8
WEIGHTED_SSE 16, 16
WEIGHTED_SSE 16, 32
WEIGHTED_SSE 16, 64

WEIGHTED_SSE 32, 8
WEIGHTED_SSE 32, 16
WEIGHTED_SSE 32, 32
WEIGHTED_SSE 32, 64

WEIGHTED_SSE 64, 16
WEIGHTED_SSE 64, 32
WEIGHTED_SSE 64, 64
WEIGHTED_SSE 64, 128

WEIGHTED_SSE 128, 64
WEIGHTED_SSE 128, 128

cglobal weighted_sse_32x4_internal, 0, 0, 0, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3
    mova                m3, [srcq]
    mova                m4, [dstq]
    punpcklbw           m2, m3, m4
    punpckhbw           m3, m4
    mova                m5, [srcq+src_strideq]
    mova                m6, [dstq+dst_strideq]
    punpcklbw           m4, m5, m6
    punpckhbw           m5, m6
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddubsw           m4, m0
    pmaddubsw           m5, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    pmaddwd             m4, m4
    pmaddwd             m5, m5
    ; two separate accumulators
    paddd               m2, m4
    paddd               m3, m5
    mova                m5, [srcq+src_strideq*2]
    mova                m6, [dstq+dst_strideq*2]
    punpcklbw           m4, m5, m6
    punpckhbw           m5, m6
    mova                m7, [srcq+src_stride3q]
    mova                m8, [dstq+dst_stride3q]
    punpcklbw           m6, m7, m8
    punpckhbw           m7, m8
    pmaddubsw           m4, m0
    pmaddubsw           m5, m0
    pmaddubsw           m6, m0
    pmaddubsw           m7, m0
    pmaddwd             m4, m4
    pmaddwd             m5, m5
    pmaddwd             m6, m6
    pmaddwd             m7, m7
    paddd               m4, m6
    paddd               m5, m7
    paddd               m2, m4
    paddd               m3, m5

    pshufd              m4, m2, q3311
    paddd               m2, m4
    pshufd              m4, m3, q3311
    paddd               m3, m4

    ; load scale for 4x4 blocks and convert to 64-bits
    ; raw load:    0, 1, 2, 3 | 4, 5, 6, 7
    ; unpack low:  0,    1    | 4,    5
    ; unpack high: 2,    3,   | 6,    7
    pxor                m6, m6
    mova                m5, [scaleq]
    punpckldq           m4, m5, m6
    punpckhdq           m5, m6

    pmuludq             m2, m4
    pmuludq             m3, m5
    vpbroadcastq        m4, [rounding]
    paddq               m2, m4
    paddq               m3, m4
    psrlq               m2, 12
    psrlq               m3, 12
    paddq               m2, m3
    paddq               m1, m2
    ret

INIT_XMM avx2

cglobal weighted_sse_4x4_hbd, 6, 8, 4, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    movq                m0, [srcq]
    movq                m1, [dstq]
    psubw               m0, m1
    pmaddwd             m0, m0
    movq                m1, [srcq+src_strideq]
    movq                m2, [dstq+dst_strideq]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    movq                m1, [srcq+src_strideq*2]
    movq                m2, [dstq+dst_strideq*2]
    psubw               m1, m2
    pmaddwd             m1, m1
    movq                m2, [srcq+src_stride3q]
    movq                m3, [dstq+dst_stride3q]
    psubw               m2, m3
    pmaddwd             m2, m2
    paddd               m1, m2
    paddd               m0, m1

    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0

    mov             scaled, [scaleq]
    mul             scaleq
    add                rax, 0x800
    shr                rax, 12
    RET




cglobal sse_4x4_internal, 0, 0, 0, src, src_stride, dst, dst_stride, \
                                   src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    movq                m0, [addsub]
    movd                m1, [srcq]
    movd                m2, [dstq]
    punpcklbw           m1, m2
    movd                m2, [srcq+src_strideq]
    movd                m3, [dstq+dst_strideq]
    punpcklbw           m2, m3
    pmaddubsw           m1, m0
    pmaddubsw           m2, m0
    pmaddwd             m1, m1
    pmaddwd             m2, m2
    paddd               m1, m2
    movd                m2, [srcq+src_strideq*2]
    movd                m3, [dstq+dst_strideq*2]
    punpcklbw           m2, m3
    movd                m3, [srcq+src_stride3q]
    movd                m4, [dstq+dst_stride3q]
    punpcklbw           m3, m4
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    paddd               m1, m2
    ret

cglobal sse_4x4, 4, 6, 5, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    ; Consider inlining this
    call m(sse_4x4_internal)

    pshuflw             m0, m1, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_4x8, 4, 6, 6, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    call m(sse_4x4_internal)
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    movd                m2, [srcq]
    movd                m3, [dstq]
    punpcklbw           m2, m3
    movd                m3, [srcq+src_strideq]
    movd                m4, [dstq+dst_strideq]
    punpcklbw           m3, m4
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    movd                m3, [srcq+src_strideq*2]
    movd                m4, [dstq+dst_strideq*2]
    punpcklbw           m3, m4
    movd                m4, [srcq+src_stride3q]
    movd                m5, [dstq+dst_stride3q]
    punpcklbw           m4, m5
    pmaddubsw           m3, m0
    pmaddubsw           m4, m0
    pmaddwd             m3, m3
    pmaddwd             m4, m4
    paddd               m3, m4
    paddd               m2, m3
    paddd               m1, m2

    pshuflw             m0, m1, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_4x16, 4, 7, 5, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3, h
    call m(sse_4x4_internal)
    mov                 hd, 2
.loop:
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    movd                m2, [srcq]
    movd                m3, [dstq]
    punpcklbw           m2, m3
    movd                m3, [srcq+src_strideq]
    movd                m4, [dstq+dst_strideq]
    punpcklbw           m3, m4
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    movd                m3, [srcq+src_strideq*2]
    movd                m4, [dstq+dst_strideq*2]
    punpcklbw           m3, m4
    movd                m4, [srcq+src_stride3q]
    movd                m5, [dstq+dst_stride3q]
    punpcklbw           m4, m5
    pmaddubsw           m3, m0
    pmaddubsw           m4, m0
    pmaddwd             m3, m3
    pmaddwd             m4, m4
    paddd               m3, m4
    paddd               m2, m3
    paddd               m1, m2
    dec                 hq
    jge .loop

    pshuflw             m0, m1, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_8x4_internal, 0, 0, 0, src, src_stride, dst, dst_stride, \
                                   src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    mova                m0, [addsub]
    movq                m1, [srcq]
    punpcklbw           m1, [dstq]
    movq                m2, [srcq+src_strideq]
    punpcklbw           m2, [dstq+dst_strideq]
    pmaddubsw           m1, m0
    pmaddubsw           m2, m0
    pmaddwd             m1, m1
    pmaddwd             m2, m2
    paddd               m1, m2
    movq                m2, [srcq+src_strideq*2]
    punpcklbw           m2, [dstq+dst_strideq*2]
    movq                m3, [srcq+src_stride3q]
    punpcklbw           m3, [dstq+dst_stride3q]
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    paddd               m1, m2
    ret

cglobal sse_8x4, 4, 6, 5, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    call m(sse_8x4_internal)

    pshufd              m0, m1, q3232
    paddd               m0, m1
    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_8x8, 4, 6, 5, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    call m(sse_8x4_internal)
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    movq                m2, [srcq]
    punpcklbw           m2, [dstq]
    movq                m3, [srcq+src_strideq]
    punpcklbw           m3, [dstq+dst_strideq]
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    movq                m3, [srcq+src_strideq*2]
    punpcklbw           m3, [dstq+dst_strideq*2]
    movq                m4, [srcq+src_stride3q]
    punpcklbw           m4, [dstq+dst_stride3q]
    pmaddubsw           m3, m0
    pmaddubsw           m4, m0
    pmaddwd             m3, m3
    pmaddwd             m4, m4
    paddd               m3, m4
    paddd               m2, m3
    paddd               m1, m2

    pshufd              m0, m1, q3232
    paddd               m0, m1
    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_8x16, 4, 7, 5, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3, h
    call m(sse_8x4_internal)
    mov                 hd, 2
.loop:
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    movq                m2, [srcq]
    punpcklbw           m2, [dstq]
    movq                m3, [srcq+src_strideq]
    punpcklbw           m3, [dstq+dst_strideq]
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    movq                m3, [srcq+src_strideq*2]
    punpcklbw           m3, [dstq+dst_strideq*2]
    movq                m4, [srcq+src_stride3q]
    punpcklbw           m4, [dstq+dst_stride3q]
    pmaddubsw           m3, m0
    pmaddubsw           m4, m0
    pmaddwd             m3, m3
    pmaddwd             m4, m4
    paddd               m3, m4
    paddd               m2, m3
    paddd               m1, m2
    dec                 hq
    jge .loop

    pshufd              m0, m1, q3232
    paddd               m0, m1
    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_8x32, 4, 7, 5, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3, h
    call m(sse_8x4_internal)
    mov                 hd, 6
.loop:
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    movq                m2, [srcq]
    punpcklbw           m2, [dstq]
    movq                m3, [srcq+src_strideq]
    punpcklbw           m3, [dstq+dst_strideq]
    pmaddubsw           m2, m0
    pmaddubsw           m3, m0
    pmaddwd             m2, m2
    pmaddwd             m3, m3
    paddd               m2, m3
    movq                m3, [srcq+src_strideq*2]
    punpcklbw           m3, [dstq+dst_strideq*2]
    movq                m4, [srcq+src_stride3q]
    punpcklbw           m4, [dstq+dst_stride3q]
    pmaddubsw           m3, m0
    pmaddubsw           m4, m0
    pmaddwd             m3, m3
    pmaddwd             m4, m4
    paddd               m3, m4
    paddd               m2, m3
    paddd               m1, m2
    dec                 hq
    jge .loop

    pshufd              m0, m1, q3232
    paddd               m0, m1
    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_4x4_hbd, 4, 6, 4, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    movq                m0, [srcq]
    movq                m1, [dstq]
    psubw               m0, m1
    pmaddwd             m0, m0
    movq                m1, [srcq+src_strideq]
    movq                m2, [dstq+dst_strideq]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    movq                m1, [srcq+src_strideq*2]
    movq                m2, [dstq+dst_strideq*2]
    psubw               m1, m2
    pmaddwd             m1, m1
    movq                m2, [srcq+src_stride3q]
    movq                m3, [dstq+dst_stride3q]
    psubw               m2, m3
    pmaddwd             m2, m2
    paddd               m1, m2
    paddd               m0, m1

    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET