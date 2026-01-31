import java.net.Socket

fun main() {
    for (port in listOf(22, 80, 443)) {
        try {
            Socket("127.0.0.1", port).use { println("$port open") }
        } catch (_: Exception) {}
    }
}
