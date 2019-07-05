%include "config.asm"
%include "ext/x86/x86inc.asm"

%if ARCH_X86_64

SECTION_RODATA 32
maddubsw_hsub: times 16 db 1, -1

SECTION .text

%define m(x) mangle(private_prefix %+ _ %+ x %+ SUFFIX)

; TODO: provide registers in parameters
; Perform 4x4 hadamard transform on input with 2 rows per register.
; Rows 0 and 2 are in m0 and rows 1 and 3 are in m1.
; A second set of packed input can also be taken in m2 and m3.
; Ends with sums in every other entry (i.e. already reduced horizontally).
%macro HADAMARD_4x4_PACKED 1
%if %1 == 1
    %define tmp m2
    ; 0->1, 1->2, 2->0
    %define ROTATE SWAP 2, 1, 0
%elif %1 == 2
    %define tmp m4
    ; 0->1, 1->2, 2->3, 3->4, 4->0
    %define ROTATE SWAP 4, 3, 2, 1, 0
%endif

    ; Stage 1
    ; 0, 2
    paddw              tmp, m0, m1
    ; 1, 3
    psubw               m0, m1
%if %1 == 2
    paddw               m1, m2, m3
    psubw               m2, m3
%endif
    ROTATE

    ; Stage 2
    ; 0, 1, 0, 1, 0, 1, 0, 1
    ; 2, 3, 2, 3, 2, 3, 2, 3
    punpcklwd          tmp, m0, m1
    punpckhwd           m0, m1
%if %1 == 2
    punpcklwd           m1, m2, m3
    punpckhwd           m2, m3
%endif
    ROTATE
    paddw              tmp, m0, m1
    psubw               m0, m1
%if %1 == 2
    paddw               m1, m2, m3
    psubw               m2, m3
%endif
    ROTATE

    ; Stage 1
    ; 0,    2,    0,    2
    ; 0, 1, 0, 1, 2, 3, 2, 3
    ; 1,    3,    1,    3
    ; 0, 1, 0, 1, 2, 3, 2, 3
    shufps             tmp, m0, m1, q2020
    shufps              m0, m1, q3131
%if %1 == 2
    shufps              m1, m2, m3, q2020
    shufps              m2, m3, q3131
%endif
    ROTATE
    paddw              tmp, m0, m1
    psubw               m0, m1
%if %1 == 2
    paddw               m1, m2, m3
    psubw               m2, m3
%endif
    ROTATE

    ; Stage 2
    ; Utilize the equality (abs(a+b)+abs(a-b))/2 = max(abs(a),abs(b)) to merge
    ;  the final butterfly stage, calculation of absolute values, and the
    ;  first stage of accumulation.
    ; Reduces the shift in the normalization step by one.
    pabsw               m0, m0
    pabsw               m1, m1

    ; Reduce horizontally early instead of fully transposing
    ; 2, X, 2, X
    pshufd             tmp, m0, q3311
    pmaxsw              m0, tmp
    ; 3, X, 3, X
    pshufd             tmp, m1, q3311
    pmaxsw              m1, tmp

    paddw               m0, m1
%if %1 == 2
    pabsw               m2, m2
    pabsw               m3, m3

    pshufd             tmp, m2, q3311
    pmaxsw              m2, tmp

    pshufd             tmp, m3, q3311
    pmaxsw              m3, tmp

    paddw               m2, m3

    paddw               m0, m2
%endif
%endmacro

; Load diffs of 4 entries for 2 rows
%macro LOAD_PACK_DIFF_Dx2 7
    movd               m%1, %2
    movd               m%6, %4
    punpckldq          m%1, m%6
    pmovzxbw           m%1, m%1
    movd               m%6, %3
    movd               m%7, %5
    punpckldq          m%6, m%7
    pmovzxbw           m%6, m%6
    psubw              m%1, m%6
%endmacro

; Can only use 128-bit vectors
%macro SATD_4x4_FN 0
cglobal satd_4x4, 4, 6, 4, src, src_stride, dst, dst_stride, \
                           src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]

    ; Load rows 0 and 2 to m0 and 1 and 3 to m1
    LOAD_PACK_DIFF_Dx2 0, [srcq], [dstq], \
                          [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                          2, 3
    LOAD_PACK_DIFF_Dx2 1, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                          [srcq+src_stride3q], [dstq+dst_stride3q], \
                          2, 3

    HADAMARD_4x4_PACKED 1

    ; Reduce horizontally
    movhlps             m1, m0
    paddw               m0, m1
    pshuflw             m1, m0, q1111

    ; Perform normalization during the final stage of accumulation
    pavgw               m0, m1
    movd               eax, m0
    movzx              eax, ax
    RET
%endmacro

INIT_XMM sse4
SATD_4x4_FN

INIT_XMM avx2
SATD_4x4_FN

; Load diffs of 8 entries for 2 row
; Each set of 4 columns share a lane
%macro LOAD_PACK_DIFF_Qx2 7
    movq              xm%1, %2
    movq              xm%6, %4
    punpckldq         xm%1, xm%6
    pmovzxbw           m%1, xm%1
    movq              xm%6, %3
    movq              xm%7, %5
    punpckldq         xm%6, xm%7
    pmovzxbw           m%6, xm%6
    psubw              m%1, m%6
%endmacro

INIT_YMM avx2
cglobal satd_8x4, 4, 6, 4, src, src_stride, dst, dst_stride, \
                           src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    ; Load rows 0 and 2 to m0 and 1 and 3 to m1
    ; Each set of 4 columns share lanes
    LOAD_PACK_DIFF_Qx2 0, [srcq], [dstq], \
                          [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                       2, 3
    LOAD_PACK_DIFF_Qx2 1, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                          [srcq+src_stride3q], [dstq+dst_stride3q], \
                       2, 3

    HADAMARD_4x4_PACKED 1

    ; Reduce horizontally
    vextracti128       xm1, m0, 1
    paddw              xm0, xm1
    movhlps            xm1, xm0
    paddw              xm0, xm1
    pshuflw            xm1, xm0, q1111

    ; Perform normalization during the final stage of accumulation
    pavgw              xm0, xm1
    movd               eax, xm0
    movzx              eax, ax
    RET

; Load diffs of 4 entries for 4 rows
; Each set of two rows share lanes
%macro LOAD_PACK_DIFF_Dx4 12
    movd              xm%1, %2
    movd             xm%10, %4
    punpckldq         xm%1, xm%10
    movd             xm%10, %6
    movd             xm%11, %8
    punpckldq        xm%10, xm%11
    punpcklqdq        xm%1, xm%10
    pmovzxbw           m%1, xm%1
    movd             xm%10, %3
    movd             xm%11, %5
    punpckldq        xm%10, xm%11
    movd             xm%11, %7
    movd             xm%12, %9
    punpckldq        xm%11, xm%12
    punpcklqdq       xm%10, xm%11
    pmovzxbw          m%10, xm%10
    psubw              m%1, m%10
%endmacro

INIT_YMM avx2
cglobal satd_4x8, 4, 8, 5, src, src_stride, dst, dst_stride, \
                           src4, dst4, src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    lea              src4q, [srcq+src_strideq*4]
    lea              dst4q, [dstq+dst_strideq*4]
    ; Load rows 0, 2, 4 and 6 to m0 and 1, 3, 5 and 7 to m1.
    ; Lanes split the low and high rows of m0 and m1.
    LOAD_PACK_DIFF_Dx4 0, [srcq], [dstq], \
                          [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                          [src4q], [dst4q], \
                          [src4q+src_strideq*2], [dst4q+dst_strideq*2], \
                       2, 3, 4
    LOAD_PACK_DIFF_Dx4 1, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                          [srcq+src_stride3q], [dstq+dst_stride3q], \
                          [src4q+src_strideq*1], [dst4q+dst_strideq*1], \
                          [src4q+src_stride3q], [dst4q+dst_stride3q], \
                       2, 3, 4

    HADAMARD_4x4_PACKED 1

    ; Reduce horizontally
    vextracti128       xm1, m0, 1
    paddw              xm0, xm1
    movhlps            xm1, xm0
    paddw              xm0, xm1
    pshuflw            xm1, xm0, q1111

    ; Perform normalization during the final stage of accumulation.
    pavgw              xm0, xm1
    movd               eax, xm0
    movzx              eax, ax
    RET

; Rudimentary hadamard transform
; Two Hadamard transforms share a lane.
%macro HADAMARD_4x4 0
    %define ROTATE SWAP 4, 3, 2, 1, 0

    ; stage 1
    paddw               m0, m1, m2
    psubw               m1, m2
    paddw               m2, m3, m4
    psubw               m3, m4
    ROTATE

    ; stage 2
    paddw               m0, m1, m3
    psubw               m1, m3
    paddw               m3, m2, m4
    psubw               m2, m4
    SWAP                3, 2, 1
    ROTATE

    ; Transpose
    ; Since two transforms share a lane, unpacking results in a single
    ;  transform's values on each register. This has to be resolved later.
    ; A (0, 1)
    punpcklwd           m0, m1, m2
    ; B (0, 1)
    punpckhwd           m1, m2
    ; A (2, 3)
    punpcklwd           m2, m3, m4
    ; B (2, 3)
    punpckhwd           m3, m4
    ROTATE

    ; A (0, 1, 2, 3)
    punpckldq           m0, m1, m3
    punpckhdq           m1, m3
    ; B (0, 1, 2, 3)
    punpckldq           m3, m2, m4
    punpckhdq           m2, m4
    SWAP                3, 2, 1
    ROTATE

    ; Make the transform share lanes again.
    ; A (0, 1, 2, 3) B (0, 1, 2, 3)
    punpcklqdq          m0, m1, m2
    punpckhqdq          m1, m2
    punpcklqdq          m2, m3, m4
    punpckhqdq          m3, m4
    ROTATE

    ; stage 1
    paddw               m0, m1, m2
    psubw               m1, m2
    paddw               m2, m3, m4
    psubw               m3, m4
    ROTATE

    ; Utilize the equality (abs(a+b)+abs(a-b))/2 = max(abs(a),abs(b)) to merge
    ;  the final butterfly stage, calculation of absolute values, and the
    ;  first stage of accumulation.
    ; Reduces the shift in the normalization step by one.
    pabsw               m1, m1
    pabsw               m3, m3
    pmaxsw              m1, m3
    pabsw               m2, m2
    pabsw               m4, m4
    pmaxsw              m2, m4

    paddw               m1, m2
    SWAP                1, 0
%endmacro

; Load diffs of 16 entries for 1 row
%macro LOAD_DIFF_DQ 4
    movu              xm%1, %2
    movu              xm%4, %3
    vpmovzxbw         m%1, xm%1,
    vpmovzxbw         m%4, xm%4,
    psubw             m%1, m%4
%endmacro

INIT_YMM avx2
cglobal satd_16x4, 4, 6, 5, src, src_stride, dst, dst_stride, \
                            src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    LOAD_DIFF_DQ 1, [srcq], [dstq], 0
    LOAD_DIFF_DQ 2, [srcq+src_strideq*1], [dstq+dst_strideq*1], 0
    LOAD_DIFF_DQ 3, [srcq+src_strideq*2], [dstq+dst_strideq*2], 0
    LOAD_DIFF_DQ 4, [srcq+src_stride3q], [dstq+dst_stride3q], 0

    HADAMARD_4x4

    ; Reduce horizontally
    vextracti128       xm1, m0, 1
    paddw              xm0, xm1
    movhlps            xm1, xm0
    paddw              xm0, xm1
    pshufd             xm1, xm0, q1111
    paddw              xm0, xm1
    pshuflw            xm1, xm0, q1111

    ; Perform normalization during the final stage of accumulation
    ; Avoids overflow in this case
    pavgw              xm0, xm1
    movd               eax, xm0
    movzx              eax, ax
    RET

INIT_YMM avx2
cglobal satd_4x16, 4, 8, 7, src, src_stride, dst, dst_stride, \
                            src4, dst4, src_stride3, dst_stride3
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    lea              src4q, [srcq+src_strideq*4]
    lea              dst4q, [dstq+dst_strideq*4]
    LOAD_PACK_DIFF_Dx4 0, [srcq], [dstq], \
                          [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                          [src4q], [dst4q], \
                          [src4q+src_strideq*2], [dst4q+dst_strideq*2], \
                       4, 5, 6
    LOAD_PACK_DIFF_Dx4 1, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                          [srcq+src_stride3q], [dstq+dst_stride3q], \
                          [src4q+src_strideq*1], [dst4q+dst_strideq*1], \
                          [src4q+src_stride3q], [dst4q+dst_stride3q], \
                       4, 5, 6
    lea               srcq, [srcq+src_strideq*8]
    lea               dstq, [dstq+dst_strideq*8]
    lea              src4q, [src4q+src_strideq*8]
    lea              dst4q, [dst4q+dst_strideq*8]
    LOAD_PACK_DIFF_Dx4 2, [srcq], [dstq], \
                          [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                          [src4q], [dst4q], \
                          [src4q+src_strideq*2], [dst4q+dst_strideq*2], \
                       4, 5, 6
    LOAD_PACK_DIFF_Dx4 3, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                          [srcq+src_stride3q], [dstq+dst_stride3q], \
                          [src4q+src_strideq*1], [dst4q+dst_strideq*1], \
                          [src4q+src_stride3q], [dst4q+dst_stride3q], \
                       4, 5, 6
    HADAMARD_4x4_PACKED 2

    ; Reduce horizontally
    vextracti128       xm1, m0, 1
    paddw              xm0, xm1
    movhlps            xm1, xm0
    paddw              xm0, xm1
    pshuflw            xm1, xm0, q1111

    ; Perform normalization during the final stage of accumulation
    pavgw              xm0, xm1
    movd               eax, xm0
    movzx              eax, ax
    RET

; Load diff of 8 entries for 1 row
%macro LOAD_DIFF_Q 4
    movq                %1, %2
    movq                %4, %3
    punpcklbw           %1, %4
    pmaddubsw           %1, hsub
%endmacro

%macro HADAMARD_8_STAGE_1 9
    paddw              m%9, m%1, m%2
    psubw              m%1, m%2
    paddw              m%2, m%3, m%4
    psubw              m%3, m%4
    paddw              m%4, m%5, m%6
    psubw              m%5, m%6
    paddw              m%6, m%7, m%8
    psubw              m%7, m%8
    ; 9->8, 1->9, 2->1, 3->2, 4->3, 5->4, 6->5, 7->6, 8->7
    SWAP                %8, %7, %6, %5, %4, %3, %2, %1, %9
%endmacro

%macro HADAMARD_8_STAGE_2 9
    paddw              m%9, m%1, m%3 ; 0
    psubw              m%1, m%3      ; 2
    paddw              m%3, m%2, m%4 ; 1
    psubw              m%2, m%4      ; 3
    SWAP                %3, %2, %1
    paddw              m%4, m%5, m%7 ; 4
    psubw              m%5, m%7      ; 6
    paddw              m%7, m%6, m%8 ; 5
    psubw              m%6, m%8      ; 7
    SWAP                %7, %6, %5
    ; 9->8, 1->9, 2->1, 3->2, 4->3, 5->4, 6->5, 7->6, 8->7
    SWAP                %8, %7, %6, %5, %4, %3, %2, %1, %9
%endmacro

; TODO: provide registers in parameters
; Rudimentary hadamard transform
%macro HADAMARD_8x8 0
    HADAMARD_8_STAGE_1 1, 2, 3, 4, 5, 6, 7, 8, 0
    HADAMARD_8_STAGE_2 1, 2, 3, 4, 5, 6, 7, 8, 0

    ; Stage 3
    paddw               m0, m1, m5 ; 0
    psubw               m1, m5     ; 4
    paddw               m5, m2, m6 ; 1
    psubw               m2, m6     ; 5
    paddw               m6, m3, m7 ; 2
    psubw               m3, m7     ; 6
    paddw               m7, m4, m8 ; 3
    psubw               m4, m8     ; 7
    SWAP                5, 2, 6, 3, 7, 4, 1
    ; 0->8, 1->0, 2->1, 3->2, 4->3, 5->4, 6->5, 7->6, 8->7
    SWAP                8, 7, 6, 5, 4, 3, 2, 1, 0

    ; transpose
    ; 0, 1
    punpcklwd           m0, m1, m2
    punpckhwd           m1, m2
    ; 2, 3
    punpcklwd           m2, m3, m4
    punpckhwd           m3, m4
    ; 4, 5
    punpcklwd           m4, m5, m6
    punpckhwd           m5, m6
    ; 6, 7
    punpcklwd           m6, m7, m8
    punpckhwd           m7, m8
    ; 0->8, 1->0, 2->1, 3->2, 4->3, 5->4, 6->5, 7->6, 8->7
    SWAP                8, 7, 6, 5, 4, 3, 2, 1, 0

    ; 0, 1, 2, 3
    punpckldq           m0, m1, m3
    punpckhdq           m1, m3
    punpckldq           m3, m2, m4
    punpckhdq           m2, m4
    SWAP                3, 2, 1
    ; 4, 5, 6, 7
    punpckldq           m4, m5, m7 ; 4
    punpckhdq           m5, m7     ; 6
    punpckldq           m7, m6, m8 ; 5
    punpckhdq           m6, m8     ; 7
    SWAP                7, 6, 5
    ; 0->8, 1->0, 2->1, 3->2, 4->3, 5->4, 6->5, 7->6, 8->7
    SWAP                8, 7, 6, 5, 4, 3, 2, 1, 0

    ; 0, 1, 2, 3, 4, 5, 6, 7
    punpcklqdq          m0, m1, m5 ; 0
    punpckhqdq          m1, m5     ; 4
    punpcklqdq          m5, m2, m6 ; 1
    punpckhqdq          m2, m6     ; 5
    punpcklqdq          m6, m3, m7 ; 2
    punpckhqdq          m3, m7     ; 6
    punpcklqdq          m7, m4, m8 ; 3
    punpckhqdq          m4, m8     ; 7
    SWAP                5, 2, 6, 3, 7, 4, 1
    ; 0->8, 1->0, 2->1, 3->2, 4->3, 5->4, 6->5, 7->6, 8->7
    SWAP                8, 7, 6, 5, 4, 3, 2, 1, 0

    HADAMARD_8_STAGE_1 1, 2, 3, 4, 5, 6, 7, 8, 0
    HADAMARD_8_STAGE_2 1, 2, 3, 4, 5, 6, 7, 8, 0

    ; Stage 3
    ; Utilize the equality (abs(a+b)+abs(a-b))/2 = max(abs(a),abs(b)) to merge
    ;  the final butterfly stage, calculation of absolute values, and the
    ;  first stage of accumulation.
    ; Reduces the shift in the normalization step by one.
    pabsw               m1, m1
    pabsw               m5, m5
    pmaxsw              m1, m5
    pabsw               m2, m2
    pabsw               m6, m6
    pmaxsw              m2, m6
    pabsw               m3, m3
    pabsw               m7, m7
    pmaxsw              m3, m7
    pabsw               m4, m4
    pabsw               m8, m8
    pmaxsw              m4, m8

    paddw               m1, m2
    paddw               m3, m4

    paddw               m1, m3
    SWAP                 1, 0
%endmacro

; Only works with 128 bit vectors
%macro SATD_8x8_FN 0
cglobal satd_8x8, 4, 6, 9, src, src_stride, dst, dst_stride, \
                           src_stride3, dst_stride3
    %define           hsub  m0
    mova              hsub, [maddubsw_hsub]
    ; Load rows into m1-m8
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    LOAD_DIFF_Q m1, [srcq], [dstq], m2
    LOAD_DIFF_Q m2, [srcq+src_strideq*1], [dstq+dst_strideq*1], m3
    LOAD_DIFF_Q m3, [srcq+src_strideq*2], [dstq+dst_strideq*2], m4
    LOAD_DIFF_Q m4, [srcq+src_stride3q], [dstq+dst_stride3q], m5
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+src_strideq*4]
    LOAD_DIFF_Q m5, [srcq], [dstq], m6
    LOAD_DIFF_Q m6, [srcq+src_strideq*1], [dstq+dst_strideq*1], m7
    LOAD_DIFF_Q m7, [srcq+src_strideq*2], [dstq+dst_strideq*2], m8
    LOAD_DIFF_Q m8, [srcq+src_stride3q], [dstq+dst_stride3q], m9

    HADAMARD_8x8

    ; Reduce horizontally and convert to 32 bits
    pxor                m2, m2
    punpcklwd           m1, m0, m2
    punpckhwd           m0, m2
    paddd               m0, m1

    movhlps             m1, m0
    paddd               m0, m1
    pshufd              m1, m0, q1111
    paddd               m0, m1
    movd               eax, m0
    add                eax, 2
    shr                eax, 2
    RET
%endmacro

INIT_XMM ssse3
SATD_8x8_FN

INIT_XMM avx2
SATD_8x8_FN

INIT_YMM avx2
cglobal satd_16x8, 4, 6, 9, src, src_stride, dst, dst_stride, \
                            src_stride3, dst_stride3
    ; Load rows into m1-m8
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    LOAD_DIFF_DQ 1, [srcq], [dstq], 0
    LOAD_DIFF_DQ 2, [srcq+src_strideq*1], [dstq+dst_strideq*1], 0
    LOAD_DIFF_DQ 3, [srcq+src_strideq*2], [dstq+dst_strideq*2], 0
    LOAD_DIFF_DQ 4, [srcq+src_stride3q], [dstq+dst_stride3q], 0
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+src_strideq*4]
    LOAD_DIFF_DQ 5, [srcq], [dstq], 0
    LOAD_DIFF_DQ 6, [srcq+src_strideq*1], [dstq+dst_strideq*1], 0
    LOAD_DIFF_DQ 7, [srcq+src_strideq*2], [dstq+dst_strideq*2], 0
    LOAD_DIFF_DQ 8, [srcq+src_stride3q], [dstq+dst_stride3q], 0

    HADAMARD_8x8

    ; Reduce horizontally and convert to 32 bits
    pxor                m2, m2
    punpcklwd           m1, m0, m2
    punpckhwd           m0, m2
    paddd               m0, m1

    vextracti128       xm1, m0, 1
    paddd              xm0, xm1
    movhlps            xm1, xm0
    paddd              xm0, xm1
    pshufd             xm1, xm0, q1111
    paddd              xm0, xm1
    movd               eax, xm0
    add                eax, 2
    shr                eax, 2
    RET

%macro LOAD_DIFF_Qx2 7
    movq              xm%1, %2
    movq              xm%6, %3
    punpcklbw         xm%1, xm%6
    movq              xm%6, %4
    movq              xm%7, %5
    punpcklbw         xm%6, xm%7
    vinserti128        m%1, xm%6, 1
    pmaddubsw          m%1, hsub
%endmacro

INIT_YMM avx2
cglobal satd_8x16, 4, 8, 11, src, src_stride, dst, dst_stride, \
                             src8, dst8, src_stride3, dst_stride3
    %define           hsub  m0
    mova              hsub, [maddubsw_hsub]
    ; Load rows into m1-m8
    lea              src8q, [srcq+src_strideq*8]
    lea              dst8q, [dstq+dst_strideq*8]
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    LOAD_DIFF_Qx2 1, [srcq], [dstq], \
                     [src8q], [dst8q], \
                     9, 10
    LOAD_DIFF_Qx2 2, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                     [src8q+src_strideq*1], [dst8q+dst_strideq*1], \
                     9, 10
    LOAD_DIFF_Qx2 3, [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                     [src8q+src_strideq*2], [dst8q+dst_strideq*2], \
                     9, 10
    LOAD_DIFF_Qx2 4, [srcq+src_stride3q], [dstq+dst_stride3q], \
                     [src8q+src_stride3q], [dst8q+dst_stride3q], \
                     9, 10
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+src_strideq*4]
    lea              src8q, [src8q+src_strideq*4]
    lea              dst8q, [dst8q+src_strideq*4]
    LOAD_DIFF_Qx2 5, [srcq], [dstq], \
                     [src8q], [dst8q], \
                     9, 10
    LOAD_DIFF_Qx2 6, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                     [src8q+src_strideq*1], [dst8q+dst_strideq*1], \
                     9, 10
    LOAD_DIFF_Qx2 7, [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                     [src8q+src_strideq*2], [dst8q+dst_strideq*2], \
                     9, 10
    LOAD_DIFF_Qx2 8, [srcq+src_stride3q], [dstq+dst_stride3q], \
                     [src8q+src_stride3q], [dst8q+dst_stride3q], \
                     9, 10

    HADAMARD_8x8

    ; Reduce horizontally and convert to 32 bits
    pxor                m2, m2
    punpcklwd           m1, m0, m2
    punpckhwd           m0, m2
    paddd               m0, m1

    vextracti128       xm1, m0, 1
    paddd              xm0, xm1
    movhlps            xm1, xm0
    paddd              xm0, xm1
    pshufd             xm1, xm0, q1111
    paddd              xm0, xm1
    movd               eax, xm0
    add                eax, 2
    shr                eax, 2
    RET

; Less optimized, boilerplate implementations

INIT_YMM avx2
cglobal satd_8x32, 4, 8, 13, src, src_stride, dst, dst_stride, \
                             src8, dst8, src_stride3, dst_stride3, cnt
    ; ones for converting to 32-bit with pmaddwd
    pcmpeqw            m11, m11
    pabsw              m11, m11
    ; sum
    pxor               m12, m12
    mov               cntd, 1
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
    lea              src8q, [srcq+src_strideq*8]
    lea              dst8q, [dstq+dst_strideq*8]
.loop:
    %define           hsub  m0
    mova              hsub, [maddubsw_hsub]
    ; Load rows into m1-m8
    LOAD_DIFF_Qx2 1, [srcq], [dstq], \
                     [src8q], [dst8q], \
                  9, 10
    LOAD_DIFF_Qx2 2, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                     [src8q+src_strideq*1], [dst8q+dst_strideq*1], \
                  9, 10
    LOAD_DIFF_Qx2 3, [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                     [src8q+src_strideq*2], [dst8q+dst_strideq*2], \
                  9, 10
    LOAD_DIFF_Qx2 4, [srcq+src_stride3q], [dstq+dst_stride3q], \
                     [src8q+src_stride3q], [dst8q+dst_stride3q], \
                  9, 10
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+src_strideq*4]
    lea              src8q, [src8q+src_strideq*4]
    lea              dst8q, [dst8q+src_strideq*4]
    LOAD_DIFF_Qx2 5, [srcq], [dstq], \
                     [src8q], [dst8q], \
                  9, 10
    LOAD_DIFF_Qx2 6, [srcq+src_strideq*1], [dstq+dst_strideq*1], \
                     [src8q+src_strideq*1], [dst8q+dst_strideq*1], \
                  9, 10
    LOAD_DIFF_Qx2 7, [srcq+src_strideq*2], [dstq+dst_strideq*2], \
                     [src8q+src_strideq*2], [dst8q+dst_strideq*2], \
                  9, 10
    LOAD_DIFF_Qx2 8, [srcq+src_stride3q], [dstq+dst_stride3q], \
                     [src8q+src_stride3q], [dst8q+dst_stride3q], \
                  9, 10

    HADAMARD_8x8

    ; Reduce horizontally and convert to 32 bits
    pmaddwd             m0, m11
    paddd              m12, m0

    lea               srcq, [srcq+src_stride3q*4]
    lea               dstq, [dstq+src_stride3q*4]
    lea              src8q, [src8q+src_stride3q*4]
    lea              dst8q, [dst8q+src_stride3q*4]
    dec               cntd
    jge .loop

    vextracti128       xm0, m12, 1
    paddd              xm0, xm12
    movhlps            xm1, xm0
    paddd              xm0, xm1
    pshufd             xm1, xm0, q1111
    paddd              xm0, xm1
    movd               eax, xm0
    add                eax, 2
    shr                eax, 2
    RET

INIT_YMM avx2
cglobal satd_16x8_internal, 0, 0, 0, \
                            dummy1, src_stride, dummy2, dst_stride, \
                            src_stride3, dst_stride3, src, dst
    %define hadd m9
    %define sum m10
    ; Load rows into m1-m8
    LOAD_DIFF_DQ 1, [srcq], [dstq], 0
    LOAD_DIFF_DQ 2, [srcq+src_strideq*1], [dstq+dst_strideq*1], 0
    LOAD_DIFF_DQ 3, [srcq+src_strideq*2], [dstq+dst_strideq*2], 0
    LOAD_DIFF_DQ 4, [srcq+src_stride3q], [dstq+dst_stride3q], 0
    lea               srcq, [srcq+src_strideq*4]
    lea               dstq, [dstq+src_strideq*4]
    LOAD_DIFF_DQ 5, [srcq], [dstq], 0
    LOAD_DIFF_DQ 6, [srcq+src_strideq*1], [dstq+dst_strideq*1], 0
    LOAD_DIFF_DQ 7, [srcq+src_strideq*2], [dstq+dst_strideq*2], 0
    LOAD_DIFF_DQ 8, [srcq+src_stride3q], [dstq+dst_stride3q], 0

    HADAMARD_8x8

    pmaddwd             m0, hadd
    paddd              sum, m0
    ret

%macro SATD_NXM 2
%if %1 > 16
%if %2 > 8
cglobal satd_%1x%2, 4, 10, 11, src, src_stride, dst, dst_stride, \
                              src_stride3, dst_stride3, call_src, call_dst, \
                              w, h
%else
cglobal satd_%1x%2, 4, 9, 11, src, src_stride, dst, dst_stride, \
                              src_stride3, dst_stride3, call_src, call_dst, \
                              w
%endif
%else ; %2 > 8
cglobal satd_%1x%2, 4, 9, 11, src, src_stride, dst, dst_stride, \
                              src_stride3, dst_stride3, call_src, call_dst, \
                              h
%endif
    ; ones for converting to 32-bit with pmaddwd
    pcmpeqw             m9, m9
    pabsw               m9, m9
    ; sum
    pxor               m10, m10
    lea       src_stride3q, [src_strideq*3]
    lea       dst_stride3q, [dst_strideq*3]
%if %2 > 8
    mov                 hd, %2/8 - 1
.looph:
%endif
%if %1 > 16
    mov                 wd, %1/16 - 1
.loopv:
%endif
    mov          call_srcq, srcq
    mov          call_dstq, dstq
    call m(satd_16x8_internal)
%if %1 > 16
    add               srcq, 16
    add               dstq, 16
    dec                 wd
    jge .loopv
    sub               srcq, %1
    sub               dstq, %1
%endif
%if %2 > 8
    lea               srcq, [srcq+src_strideq*8]
    lea               dstq, [dstq+dst_strideq*8]
    dec                 hd
    jge .looph
%endif

    ; Reduce horizontally
    vextracti128       xm0, m10, 1
    paddd              xm0, xm10
    movhlps            xm1, xm0
    paddd              xm0, xm1
    pshufd             xm1, xm0, q1111
    paddd              xm0, xm1
    movd               eax, xm0
    add                eax, 2
    shr                eax, 2
    RET
%endmacro

INIT_YMM avx2
SATD_NXM 16, 16
SATD_NXM 32, 32
SATD_NXM 64, 64
SATD_NXM 128, 128

SATD_NXM 16, 32
SATD_NXM 32, 16
SATD_NXM 32, 64
SATD_NXM 64, 32
SATD_NXM 64, 128
SATD_NXM 128, 64

SATD_NXM 32, 8
SATD_NXM 16, 64
SATD_NXM 64, 16
SATD_NXM 32, 128
SATD_NXM 128, 32

%endif ; ARCH_X86_64
