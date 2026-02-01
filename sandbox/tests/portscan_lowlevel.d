import core.sys.posix.sys.socket;
import core.sys.posix.netinet.in_;
import core.sys.posix.arpa.inet;
import core.sys.posix.unistd;
import core.stdc.stdio;

bool isOpen(ushort port)
{
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0)
        return false;

    sockaddr_in addr;
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = inet_addr("127.0.0.1");

    bool ok = connect(fd,
        cast(sockaddr*)&addr,
        addr.sizeof) == 0;

    close(fd);
    return ok;
}

static ushort* ports = [22, 80, 443];

extern (C) void main()
{
    for (ushort i = 0; i < 3; i++)
        if (isOpen(ports[i]))
            printf("%d open\n", ports[i]);
}
