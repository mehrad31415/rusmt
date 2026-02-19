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

; Define user-defined functions
(define-fun bool_ops ((a Bool) (b Bool)) Bool (and (or a b) (not (xor a b))))
(define-fun int_arithmetic ((x Int) (y Int)) Int (+ (* x 2) (div y 3)))
(define-fun is_in_range ((x Int) (low Int) (high Int)) Bool (and (>= x low) (<= x high)))
(define-fun quadratic_discriminant ((a Real) (b Real) (c Real)) Real (- (* b b) (* (/ 4 1) (* a c))))
(define-fun real_ops ((a Real) (b Real)) Real (+ (* a (/ 2 1)) (/ b (/ 2 1))))
(define-fun string_ops ((s1 String) (s2 String)) Bool (and (str.prefixof """hello""" s1) (str.contains s2 """world""")))

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

; Base SMT-LIB definitions complete
; Add error-specific assertions below

