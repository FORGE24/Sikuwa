; skw_i64_add_checked — Windows x86_64 MASM (ml64)
; int64_t __cdecl skw_i64_add_checked(int64_t a, int64_t b, skw_status_t *st);
; rcx=a, rdx=b, r8=st, rax=return

.code

skw_i64_add_checked PROC
    mov    rax, rcx
    add    rax, rdx
    jo     overflow
    test   r8, r8
    je     done
    mov    DWORD PTR [r8], 0
    ret
overflow:
    test   r8, r8
    je     zero
    mov    DWORD PTR [r8], 2
zero:
    xor    rax, rax
done:
    ret
skw_i64_add_checked ENDP

END
