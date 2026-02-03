package main
import ("encoding/base64"; "fmt"; "io"; "os"; "strings")
func main() {
    data, _ := io.ReadAll(os.Stdin)
    decoded, _ := base64.StdEncoding.DecodeString(strings.TrimSpace(string(data)))
    fmt.Print(string(decoded))
}
