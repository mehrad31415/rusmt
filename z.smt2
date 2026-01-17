(define-fun inc ((x Int)) Int
  (+ x 1))

(define-fun-rec fact ((n Int)) Int
  (ite (<= n 0)
       1
       (* n (fact (- n 1)))))


(define-funs-rec
  ((isEven ((n Int)) Bool)
   (isOdd  ((n Int)) Bool))
  ((ite (= n 0) true  (isOdd (- n 1)))
   (ite (= n 0) false (isEven (- n 1)))))