async function scan(port: number): Promise<boolean> {
  try {
    const conn = await Deno.connect({ hostname: "127.0.0.1", port });
    conn.close();
    return true;
  } catch {
    return false;
  }
}

for (const port of [22, 80, 443]) {
  if (await scan(port)) {
    console.log(`${port} open`);
  }
}
