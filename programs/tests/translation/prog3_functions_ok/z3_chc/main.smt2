(set-option :print-success false)
(set-option :produce-models true)
(set-option :produce-proofs false)
(set-option :produce-unsat-cores false)
(set-option :sat.random_seed 42)
(set-option :smt.random_seed 42)
(set-option :parallel.enable true)
(set-option :parallel.threads.max 8)
(set-option :parallel.conquer.delay 10)
(set-option :sat.restart.max 100000)
(set-option :smt.arith.solver 6)
(set-option :smt.case_split 3)
(set-option :smt.phase_selection 3)
(set-option :smt.mbqi true)
(set-option :smt.qi.eager_threshold 10.0)
(set-option :smt.qi.max_multi_patterns 1000)
(set-option :smt.ematching true)
(set-option :smt.arith.nl false)
(set-option :smt.auto_config false)

; Define Error type (set of error markers)
(declare-datatypes
	((Error 0))
	(
		((ErrEmpty)
		 (ErrSingle (err_id Int))
		 (ErrMerge (err_left Error) (err_right Error)))
	)
)

; Define Type Parameters of Function Signatures:
(declare-sort list_length_T 0)

; Define user-defined types
(declare-datatypes

                    	((Listx 1))

                    	(

                    		(par (T) ((Cons (field_Listx_Cons_1_ T) (field_Listx_Cons_2_ (Listx T))) (Nil)))

                    	)

                    )

; Define user-defined functions
(define-fun max_of_three ((a Int) (b Int) (c Int)) Int (ite (> a b) (ite (> a c) a c) (ite (> b c) b c)))
(define-fun add_two ((x Int)) Int (+ x 2))
(define-fun abs_val ((x Int)) Int (ite (< x 0) (- x) x))
(define-fun-rec list_length ((lst Listx)) Int (ite (is-Cons lst) (+ 1 (list_length_list_length_T (field_Listx_Cons_2_ lst))) 0))

; Helper functions for error handling
(define-fun err-fresh ((id Int)) Error
	(ErrSingle id))

(define-fun err-merge ((e1 Error) (e2 Error)) Error
	(ErrMerge e1 e2))

(define-fun err-is-empty ((e Error)) Bool
	(is-ErrEmpty e))

(define-fun-rec err-contains ((e Error) (id Int)) Bool
	(or
		(and (is-ErrSingle e) (= (err_id e) id))
		(and (is-ErrMerge e)
			(or (err-contains (err_left e) id)
			    (err-contains (err_right e) id)))))

; Check satisfiability and get model
; (assert <your-condition-here>)
(check-sat)
(get-model)

