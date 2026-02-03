.global _start
_start:
    sub $16, %rsp
    movw $2, (%rsp)
    movl $0x0100007f, 4(%rsp)

    # Keep constants in callee-saved regs
    mov $2, %r12d                # AF_INET
    mov $1, %r13d                # SOCK_STREAM
    mov $16, %r14d               # sizeof sockaddr
    mov %rsp, %r15               # sockaddr ptr

.macro SCAN port
    movw $\port, 2(%r15)
    mov $41, %eax                # socket
    mov %r12d, %edi
    mov %r13d, %esi
    xor %edx, %edx
    syscall
    mov %eax, %ebx               # save fd
    mov $42, %eax                # connect
    mov %ebx, %edi
    mov %r15, %rsi
    mov %r14d, %edx
    syscall
    mov $3, %eax                 # close
    mov %ebx, %edi
    syscall
.endm

    SCAN 0x1600
    SCAN 0x5000
    SCAN 0xbb01

    mov $60, %eax
    xor %edi, %edi
    syscall
