//! The unit testing utilities in this file should not be used.
//! This file should eventually be removed. The modules have already been reorganized, thus simply deleting this file will not affect the codebase.
//! The tests should be written in the `testing` package.

#[cfg(test)]
/// Utility rules of making unit tests
/// There are two cases for unit tests:
/// 1. When the test is expected to pass, the test is run and the result is expected to be Ok.
/// 2. When the test is expected to fail, the test is run and the result is expected to be Err.
macro_rules! unit_test {
    ($name:ident, $stream:tt) => {
        #[test]
        fn $name() {
            let code = quote::quote! $stream;
            test_on_stream(code).unwrap();
        }
    };
    ($name:ident, $stream:tt, $msg:expr) => {
        #[test]
        fn $name() {
            let code = quote::quote! $stream;
            match test_on_stream(code) {
                Ok(_) => panic!("expect failure in test {}", stringify!($name)),
                Err(e) => {
                    let err = e.to_string();
                    let exp = $msg;
                    if !err.contains(exp) {
                        panic!("\n==== expected error ====\n{}\n==== actual error ====\n{}", exp, err)
                    }
                }
            };
        }
    };
}
// #[cfg(test)]
// pub(crate) use unit_test; // Export the unit_test macro for use in other modules inside the same crate.
// this is not needed anymore as the tests have been reorganized.

#[cfg(test)]
mod tests {
    use crate::parser::ctxt::Context; // Context manager for holding marked items
    use crate::pipeline; // The pipeline for processing the context
    use proc_macro2::TokenStream; // Only include TokenStream during testing for manipulating Rust code as token streams
    use quote::quote; // for generating Rust code as token streams
    use syn::Result;
    use tempfile::NamedTempFile; // Temporary file creation for testing.
                                 // When choosing between the temporary file variants, prefer `tempfile` unless you either need to know the file’s path or to be able to persist it. We need to know the file's path in this case.
                                 // tempfile will (almost) never fail to cleanup temporary resources. However TempDir and NamedTempFile will fail if their destructors don’t run. This can happen if the program is terminated by a signal, or if the destructor panics.
                                 // 1 ) let temp_file = tempfile().unwrap(); // Temporary file creation for testing
                                 // 2 )  use std::fs::File; // File operations for testing
                                 //      let temp_dir = tempdir().unwrap();
                                 //      let temp_file = temp_dir.path().join("temp.rs");
                                 //      let _ = File::create(&temp_file).expect("failed to create temp file");
                                 // the files are not created after the first three lines, so the last line is necessary to create the file. Otherwise, the test will fail.
                                 // NamedTempFile is more idiomatic than TempDir and tempfile, as the file is created directly and the path is returned.

    // A helper function for testing, allowing tests to run the pipeline on a token stream.
    fn new_from_stream(stream: TokenStream) -> Result<Context> {
        // let mut ctxt = Self {
        //     types: BTreeMap::new(),
        //     impls: BTreeMap::new(),
        //     specs: BTreeMap::new(),
        //     axioms: BTreeMap::new(),
        // };
        // the following two lines are equivalent to the above.
        let temp_file = NamedTempFile::new().expect("failed to create temp file");
        let mut ctxt = Context::new(temp_file)?;

        ctxt.process_syntax(syn::parse2(stream)?)?; // add the parsed stream to the context
        ctxt.sanity_check()?;
        dbg!(&ctxt);
        Ok(ctxt)
    }

    /// A helper function for testing, allowing tests to run the pipeline on a token stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - A `TokenStream` representing parsed Rust code.
    ///
    /// # Returns
    ///
    /// * A `Result` indicating success or failure during the testing pipeline.
    ///
    /// # Errors
    ///
    /// This function propagates errors from parsing and the pipeline processing.
    pub fn test_on_stream(stream: TokenStream) -> Result<()> {
        // Create a new context from the token stream and run the pipeline.
        new_from_stream(stream)
            .and_then(pipeline) // run the pipeline on the context
            .map(|ir_context| {
                // if the pipeline succeeds, return success
                dbg!(&ir_context);
                
            })
    }

    #[test]
    fn test_pipeline() {
        let stream = quote! {
            #[smt_type]
            struct Point {
                x: Integer,
                y: Integer,
            }
        };

        let res = test_on_stream(stream);
        assert!(res.is_ok());
    }

    // this cannot be moved into the integration testing as #[smt_impl] will throw a compiler error `expect type declaration` for `self`
    unit_test!(
        receiver,
        {
            #[smt_impl]
            fn foo(self) -> Boolean {
                false.into()
            }
        },
        "unexpected self param"
    );

    // in bool_basics.rs it is a failure test but here it is a success test
    unit_test!(r, {
        #[smt_impl]
        fn foo(x: Boolean, y: Boolean) -> Boolean {
            x.not().and(false.into()).or(true.into()).xor(y).eq(x.ne(y))
        }
    });
}
