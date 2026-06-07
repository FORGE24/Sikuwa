/* skw_i64_add_checked — Windows x86_64 GAS (MinGW) */
    .text
    .globl skw_i64_add_checked
    .def skw_i64_add_checked; .scl 2; .type 32; .endef

skw_i64_add_checked:
    mov    %rcx, %rax
    add    %rdx, %rax
    jo     .Loverflow
    test   %r8, %r8
    je     .Ldone
    movl   $0, (%r8)
    ret
.Loverflow:
    test   %r8, %r8
    je     .Lzero
    movl   $2, (%r8)
.Lzero:
    xor    %rax, %rax
.Ldone:
    ret
