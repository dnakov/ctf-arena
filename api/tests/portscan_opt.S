.global _start
_start:
    # Build sockaddr once, reuse for all ports
    sub $16, %rsp
    movw $2, (%rsp)              # AF_INET
    movl $0x0100007f, 4(%rsp)    # 127.0.0.1

.macro SCAN port
    movw $\port, 2(%rsp)
    mov $41, %eax               # socket(2, 1, 0)
    mov $2, %edi
    mov $1, %esi
    xor %edx, %edx
    syscall
    mov %eax, %r8d              # save fd
    mov $42, %eax               # connect(fd, addr, 16)
    mov %r8d, %edi
    mov %rsp, %rsi
    mov $16, %edx
    syscall
    mov $3, %eax                # close(fd)
    mov %r8d, %edi
    syscall
.endm

    SCAN 0x1600                 # port 22
    SCAN 0x5000                 # port 80
    SCAN 0xbb01                 # port 443

    mov $60, %eax               # exit(0)
    xor %edi, %edi
    syscall
