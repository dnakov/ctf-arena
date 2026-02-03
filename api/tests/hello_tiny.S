.global _start
_start:
    mov $1, %eax        # syscall: write
    mov $1, %edi        # fd: stdout
    lea msg(%rip), %rsi # buf
    mov $6, %edx        # count
    syscall
    mov $60, %eax       # syscall: exit
    xor %edi, %edi      # status: 0
    syscall
msg:
    .ascii "hello\n"
