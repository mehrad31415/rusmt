 (declare-datatype Value                                                                                                                                                      
      ((Value_String (field_Value_String_1_ String))                                                                                                                           
       (Value_Integer (field_Value_Integer_1_ Int))                                                                                                                            
       (Value_Boolean (field_Value_Boolean_1_ Bool))))                                                                                                                         
                                                                                                                                                                               
  (declare-const v Value)
  (assert (= v (Value_String "hi")))                                                                                                                                           
  (assert (is-Value_String v))                
  (check-sat)       