(module
  (import "wasi_snapshot_preview1" "sock_open" (func $sock_open (param i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 1)
  (func $main (export "_start")
    ;; WASI sockets are complex - this is a stub
    ;; Real implementation would need full WASI socket support
  )
)
