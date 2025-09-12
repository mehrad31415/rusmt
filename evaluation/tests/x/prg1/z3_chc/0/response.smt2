; benchmark generated from rust API
(set-info :status unknown)
(declare-fun rhs () Bool)
(declare-fun lhs () Bool)

(declare-fun _and (Bool Bool) Bool)
(declare-fun _and_spec (Bool Bool) Bool)
(declare-fun _and_axiom (Bool Bool) Bool)

(assert
 (let (($x7 (and lhs rhs)))
 (let (($x8 (_and lhs rhs)))
 (= $x8 $x7))))
(assert
 (let (($x32 (_and_axiom lhs rhs)))
 (= $x32 (= (_and_spec lhs rhs) (_and lhs rhs)))))
(assert
 (let (($x37 (_and_spec lhs rhs)))
(let (($x36 (_and lhs rhs)))
(not (= $x36 $x37)))))
(check-sat)
