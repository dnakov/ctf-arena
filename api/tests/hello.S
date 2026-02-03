.global _start
.section .data
msg: .ascii "Hello, World!\n"
.section .text
_start:
    mov $1, %rax
    mov $1, %rdi
    lea msg(%rip), %rsi
    mov $14, %rdx
    syscall
    mov $60, %rax
    xor %rdi, %rdi
    syscall
