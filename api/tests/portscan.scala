import java.net.Socket
import scala.util.{Try, Using}

object portscan {
  def main(args: Array[String]): Unit = {
    for (port <- List(22, 80, 443)) {
      Try(Using.resource(new Socket("127.0.0.1", port))(_ => println(s"$port open")))
    }
  }
}
