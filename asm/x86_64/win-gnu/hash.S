/* skw_hash64 — Windows x86_64 GAS (MinGW)
 * ABI: rcx=data, rdx=len; preserve rdi, rsi, rbx, rbp, r12-r15
 */
    .text
    .globl skw_hash64
    .def skw_hash64; .scl 2; .type 32; .endef

#define SEED  0x243F6A8885A308D3
#define MUL   0x9E3779B97F4A7C15
#define MIX1  0xFF51AFD7ED558CCD
#define MIX2  0xC4CEB9FE1A85EC53

skw_hash64:
    push   %rdi
    push   %rsi
    mov    %rcx, %rdi
    mov    %rdx, %rsi
    mov    %rsi, %r8
    mov    $SEED, %rax
    xor    %r8, %rax
    test   %rsi, %rsi
    je     .Lfinalize

.Lloop8:
    cmp    $8, %rsi
    jb     .Ltail
    mov    (%rdi), %r9
    xor    %r9, %rax
    movabs $MUL, %r10
    imul   %r10, %rax
    add    $8, %rdi
    sub    $8, %rsi
    jmp    .Lloop8

.Ltail:
    test   %rsi, %rsi
    je     .Lfinalize
    xor    %r9, %r9
    xor    %rdx, %rdx
.Ltail_build:
    movzbl (%rdi), %r8d
    mov    %edx, %ecx
    shl    %cl, %r8
    or     %r8, %r9
    inc    %rdi
    add    $8, %edx
    dec    %rsi
    jnz    .Ltail_build
    xor    %r9, %rax
    movabs $MUL, %r10
    imul   %r10, %rax

.Lfinalize:
    mov    %rax, %rcx
    shr    $33, %rcx
    xor    %rcx, %rax
    movabs $MIX1, %rcx
    imul   %rcx, %rax
    mov    %rax, %rcx
    shr    $33, %rcx
    xor    %rcx, %rax
    movabs $MIX2, %rcx
    imul   %rcx, %rax
    mov    %rax, %rcx
    shr    $33, %rcx
    xor    %rcx, %rax
    pop    %rsi
    pop    %rdi
    ret
