; skw_hash64 — Windows x86_64 MASM (ml64)

SEED EQU 0243F6A8885A308D3h
MUL  EQU 09E3779B97F4A7C15h
MIX1 EQU 0FF51AFD7ED558CCDh
MIX2 EQU 0C4CEB9FE1A85EC53h

.code

skw_hash64 PROC
    mov    r8, rdx
    mov    rax, SEED
    xor    rax, r8
    test   rdx, rdx
    je     finalize

loop8:
    cmp    rdx, 8
    jb     tail
    mov    r9, [rcx]
    xor    rax, r9
    mov    r10, MUL
    imul   rax, r10
    add    rcx, 8
    sub    rdx, 8
    jmp    loop8

tail:
    test   rdx, rdx
    je     finalize
tail_loop:
    movzx  r9, BYTE PTR [rcx]
    xor    rax, r9
    mov    r10, MUL
    imul   rax, r10
    inc    rcx
    dec    rdx
    jnz    tail_loop

finalize:
    mov    rcx, rax
    shr    rcx, 33
    xor    rax, rcx
    mov    rcx, MIX1
    imul   rax, rcx
    mov    rcx, rax
    shr    rcx, 33
    xor    rax, rcx
    mov    rcx, MIX2
    imul   rax, rcx
    mov    rcx, rax
    shr    rcx, 33
    xor    rax, rcx
    ret
skw_hash64 ENDP

END
