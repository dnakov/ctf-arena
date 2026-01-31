const net = require('net');

async function scan(port) {
  return new Promise((resolve) => {
    const socket = new net.Socket();
    socket.setTimeout(1000);
    socket.on('connect', () => { socket.destroy(); resolve(true); });
    socket.on('error', () => resolve(false));
    socket.on('timeout', () => { socket.destroy(); resolve(false); });
    socket.connect(port, '127.0.0.1');
  });
}

(async () => {
  for (const port of [22, 80, 443]) {
    if (await scan(port)) console.log(`${port} open`);
  }
})();
