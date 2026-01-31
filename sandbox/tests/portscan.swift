#if os(Linux)
import Glibc
#else
import Darwin
#endif

for port: Int32 in [22, 80, 443] {
    let fd = socket(AF_INET, Int32(SOCK_STREAM.rawValue), 0)
    guard fd >= 0 else { continue }
    var addr = sockaddr_in()
    addr.sin_family = sa_family_t(AF_INET)
    addr.sin_port = UInt16(port).bigEndian
    inet_pton(AF_INET, "127.0.0.1", &addr.sin_addr)
    let result = withUnsafePointer(to: &addr) {
        $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
            connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    if result == 0 { print("\(port) open") }
    close(fd)
}
