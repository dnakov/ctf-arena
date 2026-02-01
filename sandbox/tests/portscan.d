import std.socket : TcpSocket, InternetAddress, SocketException;
import std.stdio : writeln;

bool isOpen(ushort port)
{
    auto s = new TcpSocket();
    scope (exit)
        s.close();

    try
    {
        s.connect(new InternetAddress("127.0.0.1", port));
        return true;
    }
    catch (SocketException)
    {
        return false;
    }
}

static ushort[] ports = [22, 80, 443];

void main()
{
    foreach (port; ports)
        if (port.isOpen)
            writeln(i"$(port) open");
}
