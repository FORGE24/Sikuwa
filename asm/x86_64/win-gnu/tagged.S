/* skw_tagged_as_i64 — Windows x86_64 GAS (MinGW) */
    .text
    .globl skw_tagged_as_i64
    .def skw_tagged_as_i64; .scl 2; .type 32; .endef

#define SKW_TAG_INT 2

skw_tagged_as_i64:
    test   %rcx, %rcx
    je     .Lnull
    movzbl (%rcx), %r9d
    cmp    $SKW_TAG_INT, %r9d
    jne    .Lbad
    mov    8(%rcx), %rax
    test   %rdx, %rdx
    je     .Ldone
    movl   $0, (%rdx)
    ret
.Lnull:
    test   %rdx, %rdx
    je     .Lzero
    movl   $1, (%rdx)
    jmp    .Lzero
.Lbad:
    test   %rdx, %rdx
    je     .Lzero
    movl   $1, (%rdx)
.Lzero:
    xor    %rax, %rax
.Ldone:
    ret
