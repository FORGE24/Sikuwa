; skw_tagged_as_i64 — Windows x86_64 MASM (ml64)
; int64_t __cdecl skw_tagged_as_i64(const skw_tagged_t *t, skw_status_t *st);
; rcx=t, rdx=st, rax=return

SKW_TAG_INT EQU 2

.code

skw_tagged_as_i64 PROC
    test   rcx, rcx
    je     null_ptr
    movzx  r9d, BYTE PTR [rcx]
    cmp    r9d, SKW_TAG_INT
    jne    bad_tag
    mov    rax, [rcx+8]
    test   rdx, rdx
    je     done
    mov    DWORD PTR [rdx], 0
    ret
null_ptr:
    test   rdx, rdx
    je     zero
    mov    DWORD PTR [rdx], 1
    jmp    zero
bad_tag:
    test   rdx, rdx
    je     zero
    mov    DWORD PTR [rdx], 1
zero:
    xor    rax, rax
done:
    ret
skw_tagged_as_i64 ENDP

END
