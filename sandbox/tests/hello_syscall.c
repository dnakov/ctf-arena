#include <sys/syscall.h>
#include <unistd.h>

int main(void) {
    syscall(SYS_write, 1, "hello\n", 6);
    return 0;
}
