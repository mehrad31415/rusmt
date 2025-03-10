; verification of impl-spec pair: add ~> add_spec
; finding satisfiability of add ~> add_spec
(set-option :print-success false)
(set-option :produce-models true)
(set-logic ALL)

; Define uninterpreted sort (Type parameters):

; Define user-defined types:
(declare-datatypes () ((Point (mk-Point (x Int) (y Int)))))

(define-fun-rec add ((lhs Point) (rhs Point)) Point (mk-Point (+ (x lhs) (x rhs)) (+ (y lhs) (y rhs))))
(declare-fun add_spec (Point Point) Point)

; (declare-fun add_axiom (Point Point) Bool)

; (assert (forall ((lhs Point) (rhs Point)) (=> (add_axiom lhs rhs) (= (add_spec lhs rhs) (add lhs rhs)))))

(assert (forall ((lhs Point) (rhs Point)) (= 
    (add_spec lhs rhs)
    (and 
        (= (lhs.x rhs.x) (add lhs rhs).x)
        (= (lhs.y rhs.y) (add lhs rhs).y)
    )
)))

Prove:
forall (lhs Point) (rhs Point) (= (add_spec lhs rhs) (add lhs rhs))

(declare-var lhs Point)
(declare-var rhs Point)

(assert (!= (add_spec lhs rhs) (add lhs rhs)))


; Universal negation check: If there exists (lhs, rhs) where add_axiom does NOT hold

(assert (exists ((lhs Point) (rhs Point)) (not (add_axiom lhs rhs))))

(check-sat)


