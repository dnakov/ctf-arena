#lang racket
(require racket/tcp)

(define (scan port)
  (with-handlers ([exn:fail? (lambda (_) (void))])
    (define-values (in out) (tcp-connect "127.0.0.1" port))
    (displayln (format "~a open" port))
    (close-input-port in)
    (close-output-port out)))

(for-each scan '(22 80 443))
