import scala.scalanative.posix.sys.socket._
import scala.scalanative.posix.netinet.in._
import scala.scalanative.posix.arpa.inet._
import scala.scalanative.posix.unistd._
import scala.scalanative.unsafe._
import scala.scalanative.unsigned._

object Main {
  def main(args: Array[String]): Unit = {
    val ports = Array(22, 80, 443)
    for (port <- ports) {
      Zone { implicit z =>
        val fd = socket(AF_INET, SOCK_STREAM, 0)
        if (fd >= 0) {
          val addr = alloc[sockaddr_in]()
          addr.sin_family = AF_INET.toUShort
          addr.sin_port = htons(port.toUShort)
          inet_pton(AF_INET, c"127.0.0.1", addr.sin_addr.at1.asInstanceOf[Ptr[Byte]])
          val result = connect(fd, addr.asInstanceOf[Ptr[sockaddr]], sizeof[sockaddr_in].toUInt)
          if (result == 0) {
            println(s"$port open")
          }
          close(fd)
        }
      }
    }
  }
}
