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

SECTION .text

%define m(x) mangle(private_prefix %+ _ %+ x %+ SUFFIX)

INIT_XMM avx2

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
.loop
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