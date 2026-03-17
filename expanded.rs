    Checking winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.58s

#[rustc_test_marker = "test_generics"]
#[doc(hidden)]
pub const test_generics: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_generics"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "winnow-grammar/tests/generics_test.rs",
        start_line: 14usize,
        start_col: 4usize,
        end_line: 14usize,
        end_col: 17usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_generics()),
    ),
};
fn test_generics() {
    Generics::parse_main()
        .parse_test("1 2 3")
        .assert_success_is(<[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3])));
}
