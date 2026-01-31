const ports = [22, 80, 443];

for (const port of ports) {
  try {
    const socket = await Bun.connect({
      hostname: "127.0.0.1",
      port,
    });
    console.log(`${port} open`);
    socket.end();
  } catch {
    // closed
  }
}
