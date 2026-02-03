local socket = require('socket')
for _, port in ipairs({22, 80, 443}) do
  local tcp = socket.tcp()
  tcp:settimeout(1)
  if tcp:connect('127.0.0.1', port) then
    print(port .. ' open')
  end
  tcp:close()
end
