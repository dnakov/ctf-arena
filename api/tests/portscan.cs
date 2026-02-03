using System.Net.Sockets;

foreach (var port in new[] { 22, 80, 443 }) {
    try {
        using var client = new TcpClient();
        client.Connect("127.0.0.1", port);
        Console.WriteLine($"{port} open");
    } catch { }
}
