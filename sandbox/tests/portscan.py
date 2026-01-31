import socket

def scan(port):
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(1)
        s.connect(("127.0.0.1", port))
        s.close()
        return True
    except:
        return False

for port in [22, 80, 443]:
    if scan(port):
        print(f"{port} open")
