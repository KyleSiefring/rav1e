; Copyright (c) 2020, The rav1e contributors. All rights reserved
;
; This source code is subject to the terms of the BSD 2 Clause License and
; the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
; was not distributed with this source code in the LICENSE file, you can
; obtain it at www.aomedia.org/license/software. If the Alliance for Open
; Media Patent License 1.0 was not distributed with this source code in the
; PATENTS file, you can obtain it at www.aomedia.org/license/patent.

%include "ext/x86/x86inc.asm"

SECTION .text

%define m(x) mangle(private_prefix %+ _ %+ x %+ SUFFIX)

INIT_XMM avx2

cglobal sse_4x4, 4, 6, 3, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    movd                m0, [srcq]
    movd                m1, [dstq]
    pmovzxbw            m0, m0
    pmovzxbw            m1, m1
    psubw               m0, m1
    pmaddwd             m0, m0
    movd                m1, [srcq+src_strideq]
    movd                m2, [dstq+dst_strideq]
    pmovzxbw            m1, m1
    pmovzxbw            m2, m2
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    movd                m1, [srcq+src_strideq*2]
    movd                m2, [dstq+dst_strideq*2]
    pmovzxbw            m1, m1
    pmovzxbw            m2, m2
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    movd                m1, [srcq+src_stride3q]
    movd                m2, [dstq+dst_stride3q]
    pmovzxbw            m1, m1
    pmovzxbw            m2, m2
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1

    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_4x4_hbd, 4, 6, 3, src, src_stride, dst, dst_stride, \
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
    paddd               m0, m1
    movq                m1, [srcq+src_stride3q]
    movq                m2, [dstq+dst_stride3q]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1

    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_8x4_internal, 0, 0, 0, src, src_stride, dst, dst_stride, \
                                   src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    pmovzxbw            m0, [srcq]
    pmovzxbw            m1, [dstq]
    psubw               m0, m1
    pmaddwd             m0, m0
.x3
    pmovzxbw            m1, [srcq+src_strideq]
    pmovzxbw            m2, [dstq+dst_strideq]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    pmovzxbw            m1, [srcq+src_strideq*2]
    pmovzxbw            m2, [dstq+dst_strideq*2]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    pmovzxbw            m1, [srcq+src_stride3q]
    pmovzxbw            m2, [dstq+dst_stride3q]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m0
    ret

cglobal sse_8x4, 4, 6, 3, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    call m(sse_8x4_internal)

    pshufd              m1, m0, q3232
    paddd               m0, m1
    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

cglobal sse_8x8, 4, 6, 3, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    call m(sse_8x4_internal)
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    pmovzxbw            m1, [srcq]
    pmovzxbw            m2, [dstq]
    psubw               m1, m2
    pmaddwd             m1, m1
    paddd               m0, m1
    call m(sse_8x4_internal).x3

    pshufd              m1, m0, q3232
    paddd               m0, m1
    pshuflw             m1, m0, q3232
    paddd               m0, m1
    movd               eax, m0
    RET