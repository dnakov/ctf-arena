int main(void) {
    __asm__ volatile (
        "mov $1, %%eax\n"   // syscall: write
        "mov $1, %%edi\n"   // fd: stdout
        "lea msg(%%rip), %%rsi\n"
        "mov $6, %%edx\n"   // count
        "syscall\n"
        ::: "rax", "rdi", "rsi", "rdx", "rcx", "r11", "memory"
    );
    return 0;
}
__asm__(".section .rodata\nmsg: .ascii \"hello\\n\"");
