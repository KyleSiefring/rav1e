; TODO: License
%include "config.asm"
%include "ext/x86/x86inc.asm"

SECTION .text

%macro W_ABS_DIFF 8
    psubw               %1, %5
    psubw               %2, %6
    psubw               %3, %7
    psubw               %4, %8
    pabsw               %1, %1
    pabsw               %2, %2
    pabsw               %3, %3
    pabsw               %4, %4
%endmacro

INIT_XMM ssse3
cglobal sad_4x4, 4, 6, 8, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    movq                m0, [srcq]
    movq                m1, [srcq+src_strideq*1]
    movq                m2, [srcq+src_strideq*2]
    movq                m3, [srcq+src_stride3q]
    movq                m4, [dstq]
    movq                m5, [dstq+dst_strideq*1]
    movq                m6, [dstq+dst_strideq*2]
    movq                m7, [dstq+dst_stride3q]
    W_ABS_DIFF m0, m1, m2, m3, m4, m5, m6, m7 
    paddw               m0, m1
    paddw               m2, m3
    paddw               m0, m2
    pshuflw             m1, m0, q2323
    paddw               m0, m1
    pshuflw             m1, m0, q1111
    paddw               m0, m1
    movd               eax, m0
    movzx              eax, ax
    RET

%if ARCH_X86_64

; this should be a 10-bit version
; 10-bit only
INIT_XMM ssse3
cglobal sad_8x8, 4, 7, 9, src, src_stride, dst, dst_stride, \
                          src_stride3, dst_stride3, cnt
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    mov               cntd, 2
    %define            sum  m0
    pxor               sum, sum
.loop:
    movu                m1, [srcq]
    movu                m2, [srcq+src_strideq*1]
    movu                m3, [srcq+src_strideq*2]
    movu                m4, [srcq+src_stride3q]
    lea               srcq, [srcq+src_strideq*4]
    movu                m5, [dstq]
    movu                m6, [dstq+dst_strideq*1]
    movu                m7, [dstq+dst_strideq*2]
    movu                m8, [dstq+dst_stride3q]
    lea               dstq, [dstq+dst_strideq*4]
    W_ABS_DIFF m1, m2, m3, m4, m5, m6, m7, m8 
    paddw               m1, m2
    paddw               m3, m4
    paddw              sum, m1
    paddw              sum, m3
    dec               cntd
    jg .loop
    movhlps             m1, sum
    paddw              sum, m1
    pshuflw             m1, sum, q2323
    paddw              sum, m1
    pshuflw             m1, sum, q1111
    paddw              sum, m1
    movd               eax, m0
    movzx              eax, ax
    RET

INIT_XMM ssse3
cglobal sad_16x16, 4, 5, 9, src, src_stride, dst, dst_stride, \
                            cnt
    mov               cntd, 8
    %define            sum  m0
    pxor               sum, sum
.loop:
    movu                m1, [srcq]
    movu                m2, [srcq+16]
    movu                m3, [srcq+src_strideq]
    movu                m4, [srcq+src_strideq+16]
    lea               srcq, [srcq+src_strideq*2]
    movu                m5, [dstq]
    movu                m6, [dstq+16]
    movu                m7, [dstq+dst_strideq]
    movu                m8, [dstq+dst_strideq+16]
    lea               dstq, [dstq+dst_strideq*2]
    W_ABS_DIFF m1, m2, m3, m4, m5, m6, m7, m8 
    paddw               m1, m2
    paddw               m3, m4
    paddw              sum, m1
    paddw              sum, m3
    dec               cntd
    jg .loop
; convert to 32-bit
    pxor                m1, m1
    punpcklwd           m2, sum, m1
    punpckhwd          sum, m1
    paddd              sum, m2
    movhlps             m1, sum
    paddd              sum, m1
    pshufd              m1, sum, q1111
    paddd              sum, m1
    movd               eax, sum
    RET

;10 bit only
INIT_XMM ssse3
cglobal sad_32x32, 4, 5, 10, src, src_stride, dst, dst_stride, \
                             cnt
    mov               cntd, 32
    pxor                m0, m0
    pxor                m1, m1
.loop:
    movu                m2, [srcq]
    movu                m3, [srcq+16]
    movu                m4, [srcq+32]
    movu                m5, [srcq+48]
    lea               srcq, [srcq+src_strideq]
    movu                m6, [dstq]
    movu                m7, [dstq+16]
    movu                m8, [dstq+32]
    movu                m9, [dstq+48]
    lea               dstq, [dstq+dst_strideq]
    W_ABS_DIFF m2, m3, m4, m5, m6, m7, m8, m9 
    paddw               m2, m3
    paddw               m4, m5
    paddw               m0, m2
    paddw               m1, m4
    dec               cntd
    jg .loop
; convert to 32-bit
    pxor                m2, m2
    punpcklwd           m3, m0, m2
    punpckhwd           m0, m2
    paddd               m0, m3
    punpcklwd           m3, m1, m2
    punpckhwd           m1, m2
    paddd               m1, m3
    paddd               m0, m1
    movhlps             m1, m0
    paddd               m0, m1
    pshufd              m1, m0, q1111
    paddd               m0, m1
    movd               eax, m0
    RET

%endif
