const std = @import("std");
pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    var buf: [65536]u8 = undefined;
    const stdin = std.io.getStdIn();
    const n = try stdin.read(&buf);
    var end = n;
    while (end > 0 and (buf[end-1] == '\n' or buf[end-1] == '\r')) end -= 1;
    const decoder = std.base64.standard.Decoder;
    var out: [65536]u8 = undefined;
    const decoded = decoder.decode(&out, buf[0..end]) catch return;
    try stdout.writeAll(decoded);
}
