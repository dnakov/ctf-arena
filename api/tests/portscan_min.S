.global _start
_start:
    sub $16, %rsp
    movw $2, (%rsp)              # AF_INET
    movl $0x0100007f, 4(%rsp)    # 127.0.0.1
    mov %rsp, %r12               # save sockaddr ptr

.macro SCAN port
    movw $\port, 2(%r12)
    mov $41, %eax
    mov $2, %edi
    mov $1, %esi
    xor %edx, %edx
    syscall
    mov %eax, %r8d               # save fd
    mov $42, %eax
    mov %r8d, %edi
    mov %r12, %rsi
    mov $16, %edx
    syscall
    mov $3, %eax
    mov %r8d, %edi
    syscall
.endm

    SCAN 0x1600
    SCAN 0x5000
    SCAN 0xbb01

    mov $60, %eax
    xor %edi, %edi
    syscall
