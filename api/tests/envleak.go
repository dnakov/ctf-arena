package main
import ("fmt"; "os")
func main() {
    if flag := os.Getenv("FLAG"); flag != "" {
        fmt.Println(flag)
    }
}
