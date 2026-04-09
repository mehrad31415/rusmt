(define-fun my_const () Int 42)                                                                                                                                                
  (define-fun my_func ((x Int)) Int (+ x 1))                                                                                                                                     
                                              
  (declare-const a Int)                                                                                                                                                          
  (declare-const b Int)                                                                                                                                                          
  (assert (= a my_const))                                                                                                                                                      
  (assert (= b (my_func 5)))                                                                                                                                                     
  (check-sat)                                                                                                                                                                    
  (get-value (a b))