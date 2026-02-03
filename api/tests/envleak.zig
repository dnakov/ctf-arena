const std = @import("std");
pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    if (std.posix.getenv("FLAG")) |flag| {
        try stdout.print("{s}\n", .{flag});
    }
}
