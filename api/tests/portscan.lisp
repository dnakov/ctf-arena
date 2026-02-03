(require :sb-bsd-sockets)

(defun scan (port)
  (handler-case
      (let ((socket (make-instance 'sb-bsd-sockets:inet-socket :type :stream)))
        (sb-bsd-sockets:socket-connect socket #(127 0 0 1) port)
        (format t "~a open~%" port)
        (sb-bsd-sockets:socket-close socket))
    (error () nil)))

(defun main ()
  (dolist (port '(22 80 443))
    (scan port)))

(main)
