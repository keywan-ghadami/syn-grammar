// core/grammar-kit/src/common_tests/simple.rs

// This file contains simple test cases that verify basic functionality
// like constant return values, simple arithmetic, and basic error handling.
// It is designed to be included by backend test suites.

test_case!(
    simple_return,
    { pub rule main -> i32 = "a" -> { 3 } },
    [
        ("a", val 3),
        ("b", err "expected `a`")
    ]
);

test_case!(
    simple_addition,
    { pub rule main -> i32 = "a" -> { 1 + 2 } },
    [
        ("a", val 3)
    ]
);

test_case!(
    complex_types,
    {
        use std::rc::Rc;
        pub rule main -> std::rc::Rc<i32> = "a" -> { std::rc::Rc::new(1) }
    },
    [
        // Use full path or closure to verify
        // v is &Rc<i32>, so we need double deref to get i32
        ("a", check |v| assert_eq!(**v, 1))
    ]
);
