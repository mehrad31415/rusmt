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

                    	((Shape 0))

                    	(

                    		((Circle (record_Shape_Circle_radius_ Real)) (Point) (Rectangle (record_Shape_Rectangle_height_ Real) (record_Shape_Rectangle_width_ Real)))

                    	)

                    )
(declare-datatypes

                    	((Point 0))

                    	(

                    		((mk-Point (record_Point_x_ Int) (record_Point_y_ Int)))

                    	)

                    )
(declare-datatypes

                    	((Person 0))

                    	(

                    		((mk-Person (record_Person_age_ Int) (record_Person_height_ Real) (record_Person_name_ String)))

                    	)

                    )
(declare-datatypes

                    	((Color 0))

                    	(

                    		((Blue) (Custom (field_Color_Custom_1_ Int) (field_Color_Custom_2_ Int) (field_Color_Custom_3_ Int)) (Green) (Red))

                    	)

                    )

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

