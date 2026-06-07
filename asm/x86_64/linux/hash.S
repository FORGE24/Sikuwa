/* skw_hash64 — Linux x86_64 System V AMD64 (GAS)
 * uint64_t skw_hash64(const void *data, size_t len);
 */
    .text
    .globl skw_hash64
    .type  skw_hash64, @function
    .align 16

#define SEED  0x243F6A8885A308D3
#define MUL   0x9E3779B97F4A7C15
#define MIX1  0xFF51AFD7ED558CCD
#define MIX2  0xC4CEB9FE1A85EC53

skw_hash64:
    mov    %rsi, %rcx
    mov    $SEED, %rax
    xor    %rcx, %rax
    test   %rsi, %rsi
    je     .Lfinalize

.Lloop8:
    cmp    $8, %rsi
    jb     .Ltail
    mov    (%rdi), %r8
    xor    %r8, %rax
    movabs $MUL, %r9
    imul   %r9, %rax
    add    $8, %rdi
    sub    $8, %rsi
    jmp    .Lloop8

.Ltail:
    test   %rsi, %rsi
    je     .Lfinalize
    xor    %r8, %r8
    xor    %rdx, %rdx
.Ltail_build:
    movzbl (%rdi), %r9d
    mov    %edx, %ecx
    shl    %cl, %r9
    or     %r9, %r8
    inc    %rdi
    add    $8, %edx
    dec    %rsi
    jnz    .Ltail_build
    xor    %r8, %rax
    movabs $MUL, %r9
    imul   %r9, %rax
    jmp    .Lfinalize

.Lfinalize:
    mov    %rax, %rdi
    shr    $33, %rdi
    xor    %rdi, %rax
    movabs $MIX1, %rdi
    imul   %rdi, %rax
    mov    %rax, %rdi
    shr    $33, %rdi
    xor    %rdi, %rax
    movabs $MIX2, %rdi
    imul   %rdi, %rax
    mov    %rax, %rdi
    shr    $33, %rdi
    xor    %rdi, %rax
    ret
    .size skw_hash64, .-skw_hash64
