#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>

int scan(unsigned int ip, int port) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = __builtin_bswap16(port),
        .sin_addr.s_addr = ip
    };

    int result = connect(fd, (struct sockaddr*)&addr, sizeof(addr));
    close(fd);
    return result == 0 ? 1 : 0;
}

int main(int argc, char **argv) {
    // Scan localhost ports 22, 80, 443
    unsigned int localhost = 0x7f000001; // 127.0.0.1 in host order
    int ports[] = {22, 80, 443};

    for (int i = 0; i < 3; i++) {
        int open = scan(__builtin_bswap32(localhost), ports[i]);
        if (open == 1) {
            printf("%d open\n", ports[i]);
        }
    }
    return 0;
}
