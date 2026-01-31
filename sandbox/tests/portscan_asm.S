.global _start

# scan(ip in edi, port in si) -> result in eax
scan:
    push %rbx
    mov %edi, %ebx          # save ip
    mov %si, %r8w           # save port

    # socket(AF_INET=2, SOCK_STREAM=1, 0)
    mov $41, %eax
    mov $2, %edi
    mov $1, %esi
    xor %edx, %edx
    syscall
    test %eax, %eax
    js fail
    mov %eax, %r9d          # fd

    # Build sockaddr on stack
    sub $16, %rsp
    movw $2, (%rsp)         # sin_family = AF_INET
    mov %r8w, 2(%rsp)       # sin_port (already big-endian)
    mov %ebx, 4(%rsp)       # sin_addr

    # connect(fd, addr, 16)
    mov $42, %eax
    mov %r9d, %edi
    mov %rsp, %rsi
    mov $16, %edx
    syscall
    mov %eax, %ebx          # save result

    # close(fd)
    mov $3, %eax
    mov %r9d, %edi
    syscall

    add $16, %rsp
    xor %eax, %eax
    test %ebx, %ebx
    setz %al
    pop %rbx
    ret

fail:
    xor %eax, %eax
    pop %rbx
    ret

_start:
    # scan(127.0.0.1, 22)
    mov $0x0100007f, %edi   # 127.0.0.1 in little-endian
    mov $0x1600, %si        # port 22 big-endian
    call scan

    # scan(127.0.0.1, 80)
    mov $0x0100007f, %edi
    mov $0x5000, %si        # port 80 big-endian
    call scan

    # scan(127.0.0.1, 443)
    mov $0x0100007f, %edi
    mov $0xbb01, %si        # port 443 big-endian
    call scan

    # exit(0)
    mov $60, %eax
    xor %edi, %edi
    syscall
