/* skw_tagged_as_i64 — Linux x86_64 System V AMD64 (GAS)
 * int64_t skw_tagged_as_i64(const skw_tagged_t *t, skw_status_t *st);
 * rdi=t, rsi=st, rax=return
 * skw_tagged_t: uint8 tag at +0, int64 at +8 (after 8-byte align padding)
 */
    .text
    .globl skw_tagged_as_i64
    .type  skw_tagged_as_i64, @function
    .align 16

#define SKW_TAG_INT 2

skw_tagged_as_i64:
    test   %rdi, %rdi
    je     .Lnull
    movzbl (%rdi), %ecx
    cmp    $SKW_TAG_INT, %ecx
    jne    .Lbad
    mov    8(%rdi), %rax
    test   %rsi, %rsi
    je     .Ldone
    movl   $0, (%rsi)
    ret
.Lnull:
    test   %rsi, %rsi
    je     .Lzero
    movl   $1, (%rsi)
    jmp    .Lzero
.Lbad:
    test   %rsi, %rsi
    je     .Lzero
    movl   $1, (%rsi)
.Lzero:
    xor    %rax, %rax
.Ldone:
    ret
    .size skw_tagged_as_i64, .-skw_tagged_as_i64
