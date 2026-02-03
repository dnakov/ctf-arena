package main

import (
	"fmt"
	"net"
	"time"
)

func scan(port int) bool {
	addr := fmt.Sprintf("127.0.0.1:%d", port)
	conn, err := net.DialTimeout("tcp", addr, time.Second)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

func main() {
	for _, port := range []int{22, 80, 443} {
		if scan(port) {
			fmt.Printf("%d open\n", port)
		}
	}
}
