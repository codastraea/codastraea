#[test]
fn syntax_errors() {
    let t = trybuild::TestCases::new();

    t.compile_fail("tests/macro_error/collect_errors.rs");
    t.compile_fail("tests/macro_error/const_parameters.rs");
    t.compile_fail("tests/macro_error/free_function.rs");
    t.compile_fail("tests/macro_error/generic_parameters.rs");
    t.compile_fail("tests/macro_error/lifetime_parameters.rs");
    t.compile_fail("tests/macro_error/runtime_parameters.rs");
    t.compile_fail("tests/macro_error/synchronous_function.rs");
}
