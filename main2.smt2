

(declare-datatypes () ((Point (mk-Point (x Int) (y Int)))))

(assert (forall ((lhs Point) (rhs Point)) (= 
    (add_spec lhs rhs)
    (and 
        (= (lhs.x rhs.x) (add lhs rhs).x)
        (= (lhs.y rhs.y) (add lhs rhs).y)
    )
)))