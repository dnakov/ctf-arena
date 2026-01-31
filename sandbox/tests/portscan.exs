for port <- [22, 80, 443] do
  case :gen_tcp.connect(~c"127.0.0.1", port, [], 1000) do
    {:ok, sock} ->
      IO.puts("#{port} open")
      :gen_tcp.close(sock)
    _ -> nil
  end
end
