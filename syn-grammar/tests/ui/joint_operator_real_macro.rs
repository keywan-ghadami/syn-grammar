// `::` is a joint operator. `a : : b` with whitespace must NOT match - on
// every toolchain.
//
// Previously the codegen split multi-character operators per character and
// checked their adjacency via `Span::end() != Span::start()`. That depends on
// spans carrying positions inside a procedural macro at all - which is only
// the case from Rust 1.88 on (proc-macro2 `build.rs`, cfg
// `proc_macro_span_location`). On older toolchains `a : : b` passed as `::`.
// Since the mapping to `Token![::]`, syn itself checks `Spacing::Joint`,
// regardless of the toolchain version.
ui_macro::path!(a : : b);

fn main() {}
