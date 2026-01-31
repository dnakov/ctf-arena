const posix = @import("std").posix;

pub fn main() void {
    _ = posix.write(1, "hello\n") catch {};
}
