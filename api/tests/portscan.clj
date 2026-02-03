(ns portscan
  (:gen-class)
  (:import [java.net Socket]))

(defn scan [port]
  (try
    (with-open [s (Socket. "127.0.0.1" port)]
      (println (str port " open")))
    (catch Exception _)))

(defn -main [& args]
  (doseq [port [22 80 443]]
    (scan port)))
