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

; Define user-defined types
(declare-datatypes

                    	((EvalResult 0))

                    	(

                    		((Error) (Ok (field_EvalResult_Ok_1_ Int)))

                    	)

                    )
(declare-datatypes

                    	((Expr 0))

                    	(

                    		((Add (field_Expr_Add_1_ Expr) (field_Expr_Add_2_ Expr)) (Const (field_Expr_Const_1_ Int)) (Mul (field_Expr_Mul_1_ Expr) (field_Expr_Mul_2_ Expr)) (Neg (field_Expr_Neg_1_ Expr)) (Sub (field_Expr_Sub_1_ Expr) (field_Expr_Sub_2_ Expr)))

                    	)

                    )

; Define user-defined functions
(define-funs-rec ((eval_expr ((e Expr)) Int) (expr_depth ((e Expr)) Int)) ((ite (is-Add e) (+ (eval_expr (field_Expr_Add_1_ e)) (eval_expr (field_Expr_Add_2_ e))) (ite (is-Const e) (field_Expr_Const_1_ e) (ite (is-Mul e) (* (eval_expr (field_Expr_Mul_1_ e)) (eval_expr (field_Expr_Mul_2_ e))) (ite (is-Neg e) (- (eval_expr (field_Expr_Neg_1_ e))) (- (eval_expr (field_Expr_Sub_1_ e)) (eval_expr (field_Expr_Sub_2_ e))))))) (ite (is-Add e) (+ 1 (ite (> (expr_depth (field_Expr_Add_1_ e)) (expr_depth (field_Expr_Add_2_ e))) (expr_depth (field_Expr_Add_1_ e)) (expr_depth (field_Expr_Add_2_ e)))) (ite (is-Const e) 1 (ite (is-Mul e) (+ 1 (ite (> (expr_depth (field_Expr_Mul_1_ e)) (expr_depth (field_Expr_Mul_2_ e))) (expr_depth (field_Expr_Mul_1_ e)) (expr_depth (field_Expr_Mul_2_ e)))) (ite (is-Neg e) (+ 1 (expr_depth (field_Expr_Neg_1_ e))) (+ 1 (ite (> (expr_depth (field_Expr_Sub_1_ e)) (expr_depth (field_Expr_Sub_2_ e))) (expr_depth (field_Expr_Sub_1_ e)) (expr_depth (field_Expr_Sub_2_ e))))))))))
(define-fun is_positive_expr ((e Expr)) Bool (> (eval_expr e) 0))

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

