/* skw_i64_add_checked — Linux x86_64 System V AMD64 (GAS)
 * int64_t skw_i64_add_checked(int64_t a, int64_t b, skw_status_t *st);
 * rdi=a, rsi=b, rdx=st, rax=return
 */
    .text
    .globl skw_i64_add_checked
    .type  skw_i64_add_checked, @function
    .align 16

skw_i64_add_checked:
    mov    %rdi, %rax
    add    %rsi, %rax
    jo     .Loverflow
    test   %rdx, %rdx
    je     .Ldone
    movl   $0, (%rdx)
    ret
.Loverflow:
    test   %rdx, %rdx
    je     .Lzero
    movl   $2, (%rdx)
.Lzero:
    xor    %rax, %rax
.Ldone:
    ret
    .size skw_i64_add_checked, .-skw_i64_add_checked
