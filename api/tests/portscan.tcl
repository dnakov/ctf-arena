foreach port {22 80 443} {
    if {![catch {socket 127.0.0.1 $port} sock]} {
        puts "$port open"
        close $sock
    }
}
