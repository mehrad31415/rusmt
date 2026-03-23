use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{I32, smt::SMT};

#[smt_type]                                                                                                                                                                    
  struct Wrapper<T: SMT> {                                                                                                                                                            
      inner: T,   
  } 
                                                                                                                                                                               
#[smt_fn]
  fn get_inner(w: Wrapper<I32>) -> I32 {
      w.inner  // field access on generic struct
  }