.global _start
.section .text
_start:
    # Stack at _start: (%rsp)=argc, 8(%rsp)=argv[0], ...
    # envp starts after argv NULL terminator
    mov (%rsp), %rdi        # argc
    lea 8(%rsp), %rsi       # argv
    lea 8(%rsi,%rdi,8), %r12  # envp = argv + (argc+1)*8
find_flag:
    mov (%r12), %rdi
    test %rdi, %rdi
    jz exit
    cmpb $'F', (%rdi)
    jne next
    cmpb $'L', 1(%rdi)
    jne next
    cmpb $'A', 2(%rdi)
    jne next
    cmpb $'G', 3(%rdi)
    jne next
    cmpb $'=', 4(%rdi)
    jne next
    add $5, %rdi
    mov %rdi, %rsi
    xor %rdx, %rdx
strlen:
    cmpb $0, (%rsi,%rdx)
    je print
    inc %rdx
    jmp strlen
print:
    mov $1, %rax
    mov $1, %rdi
    syscall
    push $10
    mov $1, %rax
    mov $1, %rdi
    mov %rsp, %rsi
    mov $1, %rdx
    syscall
    pop %rax
    jmp exit
next:
    add $8, %r12
    jmp find_flag
exit:
    mov $60, %rax
    xor %rdi, %rdi
    syscall
