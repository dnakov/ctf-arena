program portscan;
uses sockets, baseunix;
var
  sock: longint;
  addr: TInetSockAddr;
  ports: array[0..2] of integer = (22, 80, 443);
  i: integer;
begin
  for i := 0 to 2 do begin
    sock := fpsocket(AF_INET, SOCK_STREAM, 0);
    if sock < 0 then continue;
    addr.sin_family := AF_INET;
    addr.sin_port := htons(ports[i]);
    addr.sin_addr.s_addr := StrToNetAddr('127.0.0.1').s_addr;
    if fpconnect(sock, @addr, sizeof(addr)) = 0 then
      writeln(ports[i], ' open');
    fpclose(sock);
  end;
end.
