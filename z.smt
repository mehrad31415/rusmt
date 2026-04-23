(set-option :parallel.enable true)
(set-option :parallel.threads.max 8)
(set-option :parallel.conquer.delay 6000)

(declare-const a Bool)
(declare-const b Bool)
(declare-const c Bool)
(declare-const d Bool)
(declare-const e Bool)
(declare-const f Bool)
(declare-const g Bool)
(declare-const h Bool)

;; A somewhat tangled Boolean structure
(assert (or a b c d))
(assert (or (not a) e))
(assert (or (not b) f))
(assert (or (not c) g))
(assert (or (not d) h))

(assert (or (not e) (not f)))
(assert (or (not f) (not g)))
(assert (or (not g) (not h)))
(assert (or (not h) (not e)))

(check-sat)