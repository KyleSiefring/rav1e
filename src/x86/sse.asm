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

cglobal weighted_sse_8x4_internal, 0, 0, 0, \
                                   src, src_stride, dst, dst_stride, scale, scale_stride, \
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

; Apply weights
    pshufd              m2, m1, q3311
    paddd               m1, m2
    pmovzxdq            m2, [scaleq]
    pmuludq             m1, m2
    vpbroadcastq        m2, [rounding]
    paddq               m1, m2
    psrlq               m1, 12
    ret

cglobal weighted_sse_8x8, 6, 8, 5, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        src_stride3, dst_stride3
    call m(weighted_sse_8x4_internal)
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

    pshufd              m3, m2, q3311
    paddd               m2, m3
    pmovzxdq            m3, [scaleq+scale_strideq]
    pmuludq             m2, m3
    vpbroadcastq        m3, [rounding]
    paddq               m2, m3
    psrlq               m2, 12
    paddd               m1, m2

    pshufd              m0, m1, q3232
    paddd               m0, m1
    movd               eax, m0
    RET

INIT_YMM avx2

cglobal weighted_sse_16x16, 6, 8, 7, \
        src, src_stride, dst, dst_stride, scale, scale_stride, \
        j, k
    mova                m0, [addsub]
    pxor                m1, m1
    mov                 jd, 1
.loop:
    ;pmovzxbw            m2, [srcq]
    ;pmovzxbw            m3, [dstq]
    ;psubw               m2, m3
    ;pmaddwd             m2, m2
    ;pmovzxbw            m3, [srcq+src_strideq]
    ;pmovzxbw            m4, [dstq+dst_strideq]
    ;psubw               m3, m4
    ;pmaddwd             m3, m3
    ;pmovzxbw            m3, [srcq+src_strideq]
    ;pmovzxbw            m4, [dstq+dst_strideq]

    mov                 kd, 3
    pxor                m2, m2
    pxor                m3, m3
.loop_inner:
    mova               xm5, [srcq]
    vinserti128         m5, [srcq+src_strideq*4], 1
    mova               xm6, [dstq]
    vinserti128         m6, [dstq+dst_strideq*4], 1
    punpcklbw           m4, m5, m6
    punpckhbw           m5, m6
    pmaddubsw           m4, m0
    pmaddubsw           m5, m0
    pmaddwd             m4, m4
    pmaddwd             m5, m5
    paddd               m2, m4
    paddd               m3, m5
    lea               srcq, [srcq+src_strideq]
    lea               dstq, [dstq+dst_strideq]
    dec                 kq
    jge .loop_inner

    pshufd              m4, m2, q3311
    paddd               m2, m4
    pshufd              m4, m3, q3311
    paddd               m3, m4

    mova               xm5, [scaleq]
    mova               xm6, [scaleq+scale_strideq]
    lea             scaleq, [scaleq+scale_strideq*2]
    punpcklqdq         xm4, xm5, xm6
    punpckhqdq         xm5, xm6
    pmovzxdq            m4, xm4
    pmovzxdq            m5, xm5

    pmuludq             m2, m4
    pmuludq             m3, m5
    vpbroadcastq        m4, [rounding]
    paddq               m2, m4
    paddq               m3, m4
    psrlq               m2, 12
    psrlq               m3, 12
    paddd               m1, m2
    paddd               m1, m3

    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+dst_strideq*4]
    dec                 jq
    jge .loop

    vextracti128       xm0, m1, 1
    paddq              xm0, xm1
    pshufd             xm1, xm0, q3232
    paddq              xm0, xm1
    movd               eax, xm0
    RET

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