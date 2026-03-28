    Blocking waiting for file lock on artifact directory
   Compiling winnow-grammar v0.1.0 (/home/user/syn-grammar/winnow-grammar)
error[E0782]: expected a type, found a trait
  --> winnow-grammar/tests/explicit_span_test.rs:17:1
   |
17 | / grammar! {
18 | |     grammar ExplicitSp...
19 | |         pub custom_nod...
20 | |     }
21 | | }
   | |_^
   |
   = note: this error originates in the macro `grammar` (in Nightly builds, run with -Z macro-backtrace for more info)

For more information about this error, try `rustc --explain E0782`.
error: could not compile `winnow-grammar` (test "explicit_span_test") due to 1 previous error
