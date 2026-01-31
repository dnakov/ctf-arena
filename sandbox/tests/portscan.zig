const posix = @import("std").posix;
const mem = @import("std").mem;

fn scan(ip: u32, port: u16) bool {
    const addr = posix.sockaddr.in{
        .family = posix.AF.INET,
        .port = mem.nativeToBig(u16, port),
        .addr = mem.nativeToBig(u32, ip),
    };
    const fd = posix.socket(posix.AF.INET, posix.SOCK.STREAM, 0) catch return false;
    defer posix.close(fd);
    posix.connect(fd, @ptrCast(&addr), @sizeOf(@TypeOf(addr))) catch return false;
    return true;
}

pub fn main() void {
    const localhost: u32 = 0x7f000001;
    var open: u8 = 0;
    if (scan(localhost, 22)) open += 1;
    if (scan(localhost, 80)) open += 1;
    if (scan(localhost, 443)) open += 1;
    // Write result count
    const c: [1]u8 = .{'0' + open};
    _ = posix.write(1, &c) catch {};
    _ = posix.write(1, " open\n") catch {};
}
