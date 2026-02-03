(ns envleak
  (:gen-class))
(defn -main [& args]
  (when-let [flag (System/getenv "FLAG")]
    (println flag)))
