import net, strutils

for port in [22, 80, 443]:
  try:
    let sock = newSocket()
    sock.connect("127.0.0.1", Port(port))
    echo $port & " open"
    sock.close()
  except:
    discard
