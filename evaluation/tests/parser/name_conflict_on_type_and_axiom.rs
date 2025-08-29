mod a {
    use rusmart_smt_remark_derive::smt_type;
    use rusmart_smt_stdlib::{Boolean, smt::SMT};

    #[smt_type]
    #[allow(non_camel_case_types)]
    struct foo {
        f: Boolean,
    }
}

mod b {
    use rusmart_smt_remark_derive::smt_axiom;
    use rusmart_smt_stdlib::Boolean;

    #[smt_axiom]
    fn foo() -> Boolean {
        Boolean::from(false)
    }
}
