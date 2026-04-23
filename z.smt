  ; to_integer (floor)
  (assert (= (to_int (fp.to_real ((_ to_fp 11 53) RNE 3.7))) 3))
  (assert (= (to_int (fp.to_real ((_ to_fp 11 53) RNE (- 3.7)))) (- 4)))                                                                                                         
  (assert (= (to_int (fp.to_real ((_ to_fp 11 53) RNE 0.0))) 0))
                                                                                                                                                                                 
  ; to_real       
  (assert (= (fp.to_real ((_ to_fp 11 53) RNE 3.5)) (/ 7 2)))                                                                                                                    
                                                                                                                                                                                 
  ; to_i32 (RTZ = truncation)                                                                                                                                                    
  (assert (= ((_ fp.to_sbv 32) RTZ ((_ to_fp 11 53) RNE 42.9)) (_ bv42 32)))                                                                                                     
  (assert (= ((_ fp.to_sbv 32) RTZ ((_ to_fp 11 53) RNE (- 42.9))) (_ bv4294967254 32)))                                                                                         
                                                                                                                                                                                 
  ; to_i64                                                                                                                                                                       
  (assert (= ((_ fp.to_sbv 64) RTZ ((_ to_fp 11 53) RNE 42.9)) (_ bv42 64)))                                                                                                     
                                                                                                                                                                                 
  ; to_u32
  (assert (= ((_ fp.to_ubv 32) RTZ ((_ to_fp 11 53) RNE 42.9)) (_ bv42 32)))                                                                                                     
                  
  ; to_u64
  (assert (= ((_ fp.to_ubv 64) RTZ ((_ to_fp 11 53) RNE 42.9)) (_ bv42 64)))
                                                                                                                                                                                 
  (check-sat)
                