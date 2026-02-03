#include <stdint.h>

static long syscall3(long n, long a, long b, long c) {
    long ret;
    __asm__ volatile("syscall" : "=a"(ret) : "a"(n), "D"(a), "S"(b), "d"(c) : "rcx", "r11", "memory");
    return ret;
}

static int scan(uint32_t ip, uint16_t port) {
    // socket(AF_INET=2, SOCK_STREAM=1, 0)
    long fd = syscall3(41, 2, 1, 0);
    if (fd < 0) return 0;

    struct { uint16_t family; uint16_t port; uint32_t addr; uint64_t pad; } addr = {2, __builtin_bswap16(port), ip, 0};

    // connect(fd, &addr, 16)
    long ret = syscall3(42, fd, (long)&addr, 16);

    // close(fd)
    syscall3(3, fd, 0, 0);

    return ret == 0;
}

int main(void) {
    uint32_t ip = __builtin_bswap32(0x7f000001);
    scan(ip, 22);
    scan(ip, 80);
    scan(ip, 443);
    return 0;
}
