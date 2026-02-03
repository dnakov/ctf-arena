object envleak extends App {
  sys.env.get("FLAG").foreach(println)
}
