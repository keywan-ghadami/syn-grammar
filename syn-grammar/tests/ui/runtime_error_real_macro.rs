// Error message on the REAL procedural-macro path.
//
// The rest of the test suite runs through `Parser::parse_str` and thus through
// the proc-macro2 fallback. Here a real macro runs - the only path on which the
// behaviour in production use can be checked. ADR 13, point 14.
//
// The snapshot shows that the message carries positions there. That is only
// the case from Rust 1.88 on (proc-macro2 then sets `proc_macro_span_location`
// on stable too); below that all spans would be (0,0). The project requires
// 1.88; this test pins that promise.
ui_macro::assignment!(let x = ;);

fn main() {}
