//! Success cases for the bridge parser.
//!
//! The input is the CXX bridge language as the [cxx book](https://cxx.rs)
//! documents it, not a reduced dialect: the first test is the blobstore
//! walkthrough from the book, extended with the constructs that make the IDL
//! awkward to parse — every receiver form, `unsafe fn`, type aliases, negative
//! enum discriminants, and the explicit `impl` instantiations.

use cxx_parser::{ExternItem, FfiMod, Lang, ModItem, Param, Receiver};
use quote::ToTokens;
use syn_grammar::SynTestExt;

/// Renders a type back to source with normalised whitespace, so a test can say
/// what it means (`Pin<&mut BlobstoreClient>`) instead of matching token soup.
fn ty(t: &impl ToTokens) -> String {
    t.to_token_stream().to_string().replace(' ', "")
}

fn parse(src: &str) -> FfiMod {
    cxx_parser::CxxParser::parse_top_level_mod
        .parse_test(src)
        .assert_success()
}

const BLOBSTORE: &str = r#"
    #[cxx::bridge(namespace = "org::blobstore")]
    pub mod ffi {
        use std::pin::Pin;

        /// Metadata of a stored blob.
        struct BlobMetadata {
            size: usize,
            tags: Vec<String>,
        }

        #[derive(Debug)]
        #[repr(i32)]
        enum Compression {
            None = -1,
            Lz4,
            Zstd = 19,
        }

        unsafe extern "C++" {
            include!("demo/include/blobstore.h");

            type BlobstoreClient;
            type Payload<'a>;
            type MultiBuf = crate::multi_buf::MultiBuf;

            fn new_blobstore_client() -> UniquePtr<BlobstoreClient>;

            fn put(&self, parts: &mut MultiBuf) -> u64;

            fn tag(self: Pin<&mut BlobstoreClient>, blobid: u64, tag: &str);

            #[rust_name = "metadata_for"]
            fn metadata(self: &BlobstoreClient, blobid: u64) -> BlobMetadata;

            unsafe fn read_raw(&mut self, buf: *mut u8, len: usize) -> usize;

            fn dispatch<'a, 'b>(
                self: Pin<&mut BlobstoreClient>,
                payload: &'a mut CxxVector<Payload<'b>>,
                filter: fn(&CxxString, Option<&[u8]>) -> bool,
            ) -> Result<UniquePtr<BlobMetadata>>;
        }

        extern "Rust" {
            type MultiBuf;

            fn next_chunk(buf: &mut MultiBuf) -> &[u8];
        }

        impl UniquePtr<BlobstoreClient> {}
        impl<'a> CxxVector<Payload<'a>> {}
    }
"#;

#[test]
fn parses_the_blobstore_bridge() {
    let bridge = parse(BLOBSTORE);

    assert_eq!(bridge.name, "ffi");
    assert_eq!(bridge.attrs.len(), 1, "the #[cxx::bridge] attribute");
    assert!(matches!(bridge.vis, syn::Visibility::Public(_)));
    // use, struct, enum, two extern blocks, two impls.
    assert_eq!(bridge.items.len(), 7);
}

#[test]
fn shared_struct_keeps_fields_docs_and_syn_types() {
    let bridge = parse(BLOBSTORE);
    let s = bridge.structs().next().expect("BlobMetadata");

    assert_eq!(s.name, "BlobMetadata");
    assert_eq!(s.attrs.len(), 1, "the doc comment is an attribute");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[1].name, "tags");
    assert_eq!(ty(&s.fields[1].ty), "Vec<String>");
}

#[test]
fn enum_discriminants_are_optional_and_may_be_negative() {
    let bridge = parse(BLOBSTORE);
    let e = bridge.enums().next().expect("Compression");

    assert_eq!(e.attrs.len(), 2, "#[derive] and #[repr]");
    let values: Vec<_> = e
        .variants
        .iter()
        .map(|v| (v.name.to_string(), v.discriminant))
        .collect();
    assert_eq!(
        values,
        vec![
            ("None".to_string(), Some(-1)),
            ("Lz4".to_string(), None),
            ("Zstd".to_string(), Some(19)),
        ]
    );
}

#[test]
fn both_extern_languages_are_recognised() {
    let bridge = parse(BLOBSTORE);
    let blocks: Vec<_> = bridge.extern_blocks().collect();

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].lang, Lang::Cxx);
    assert!(blocks[0].is_unsafe);
    assert_eq!(blocks[1].lang, Lang::Rust);
    assert!(!blocks[1].is_unsafe, "`extern \"Rust\"` is written safe");
}

#[test]
fn include_opaque_type_and_alias_are_told_apart() {
    let bridge = parse(BLOBSTORE);
    let items = &bridge.extern_blocks().next().unwrap().items;

    match &items[0] {
        ExternItem::Include(header) => assert_eq!(header.value(), "demo/include/blobstore.h"),
        other => panic!("expected an include, got {other:?}"),
    }
    match &items[1] {
        ExternItem::Opaque(t) => {
            assert_eq!(t.name, "BlobstoreClient");
            assert!(t.generics.params.is_empty());
        }
        other => panic!("expected an opaque type, got {other:?}"),
    }
    match &items[2] {
        ExternItem::Opaque(t) => {
            assert_eq!(t.name, "Payload");
            assert_eq!(t.generics.lifetimes().count(), 1);
        }
        other => panic!("expected an opaque type with a lifetime, got {other:?}"),
    }
    match &items[3] {
        ExternItem::Alias(a) => {
            assert_eq!(a.name, "MultiBuf");
            assert_eq!(ty(&a.path), "crate::multi_buf::MultiBuf");
        }
        other => panic!("expected a type alias, got {other:?}"),
    }
}

/// The four receiver forms are the reason the parameter list cannot simply be
/// `ident ":" type`.
#[test]
fn every_receiver_form_is_parsed() {
    let bridge = parse(BLOBSTORE);
    let block = bridge.extern_blocks().next().unwrap();
    let fns: Vec<_> = block.fns().collect();

    let free = &fns[0];
    assert_eq!(free.name, "new_blobstore_client");
    assert!(free.receiver().is_none(), "a free function has no receiver");

    let put = &fns[1];
    assert!(matches!(
        put.receiver(),
        Some(Receiver::Ref { mutable: false })
    ));

    let tag = &fns[2];
    match tag.receiver() {
        Some(Receiver::Typed(t)) => assert_eq!(ty(t), "Pin<&mutBlobstoreClient>"),
        other => panic!("expected `self: Pin<&mut …>`, got {other:?}"),
    }

    let metadata = &fns[3];
    match metadata.receiver() {
        Some(Receiver::Typed(t)) => assert_eq!(ty(t), "&BlobstoreClient"),
        other => panic!("expected `self: &BlobstoreClient`, got {other:?}"),
    }

    let read_raw = &fns[4];
    assert!(read_raw.is_unsafe, "`unsafe fn` is part of the signature");
    assert!(matches!(
        read_raw.receiver(),
        Some(Receiver::Ref { mutable: true })
    ));
    assert_eq!(ty(&read_raw.args().next().unwrap().ty), "*mutu8");
}

/// The point of the whole crate: after `:` and `->` the input is Rust, and the
/// parser has to hand over to `syn` and take back over at the right token.
#[test]
fn hands_complex_rust_types_over_to_syn() {
    let bridge = parse(BLOBSTORE);
    let block = bridge.extern_blocks().next().unwrap();
    let dispatch = block.fns().find(|f| f.name == "dispatch").unwrap();

    assert_eq!(
        dispatch.generics.lifetimes().count(),
        2,
        "`fn dispatch<'a, 'b>`"
    );

    let args: Vec<_> = dispatch.args().collect();
    assert_eq!(args.len(), 2);
    assert_eq!(ty(&args[0].ty), "&'amutCxxVector<Payload<'b>>");
    assert_eq!(
        ty(&args[1].ty),
        "fn(&CxxString,Option<&[u8]>)->bool",
        "a function pointer type, commas and all, inside a comma-separated list"
    );

    match &dispatch.ret {
        syn::ReturnType::Type(_, t) => assert_eq!(ty(t), "Result<UniquePtr<BlobMetadata>>"),
        other => panic!("expected a return type, got {other:?}"),
    }
}

#[test]
fn attributes_on_functions_survive() {
    let bridge = parse(BLOBSTORE);
    let block = bridge.extern_blocks().next().unwrap();
    let metadata = block.fns().find(|f| f.name == "metadata").unwrap();

    assert_eq!(metadata.attrs.len(), 1);
    assert_eq!(
        metadata.attrs[0].to_token_stream().to_string(),
        r#"# [rust_name = "metadata_for"]"#
    );
}

#[test]
fn impl_instantiations_carry_their_generics() {
    let bridge = parse(BLOBSTORE);
    let impls: Vec<_> = bridge.impls().collect();

    assert_eq!(impls.len(), 2);
    assert_eq!(ty(&impls[0].ty), "UniquePtr<BlobstoreClient>");
    assert!(impls[0].generics.params.is_empty());

    assert_eq!(ty(&impls[1].ty), "CxxVector<Payload<'a>>");
    assert_eq!(impls[1].generics.lifetimes().count(), 1);
}

#[test]
fn use_statements_are_kept_in_order() {
    let bridge = parse(BLOBSTORE);
    assert!(matches!(bridge.items.first(), Some(ModItem::Use(_))));
}

#[test]
fn an_empty_bridge_is_valid() {
    let bridge = parse("mod ffi {}");
    assert!(bridge.items.is_empty());
    assert!(matches!(bridge.vis, syn::Visibility::Inherited));
}

#[test]
fn trailing_commas_are_allowed_everywhere() {
    let bridge = parse(
        r#"
        mod ffi {
            struct S { a: u8, }
            enum E { A, }
            extern "Rust" {
                fn f(a: u8,);
            }
        }
        "#,
    );
    assert_eq!(bridge.structs().next().unwrap().fields.len(), 1);
    assert_eq!(bridge.enums().next().unwrap().variants.len(), 1);
    let block = bridge.extern_blocks().next().unwrap();
    assert_eq!(block.fns().next().unwrap().params.len(), 1);
}

#[test]
fn a_function_without_parameters_or_return_type_is_valid() {
    let bridge = parse(r#"mod ffi { extern "Rust" { fn tick(); } }"#);
    let block = bridge.extern_blocks().next().unwrap();
    let f = block.fns().next().unwrap();
    assert!(f.params.is_empty(), "no parameters");
    assert!(matches!(f.ret, syn::ReturnType::Default));
}

#[test]
fn receiver_and_arguments_keep_their_order() {
    let bridge = parse(r#"mod ffi { extern "Rust" { fn f(&self, a: u8, b: u8); } }"#);
    let block = bridge.extern_blocks().next().unwrap();
    let f = block.fns().next().unwrap();

    assert!(matches!(f.params[0], Param::Receiver(_)));
    let args: Vec<_> = f.args().map(|a| a.name.to_string()).collect();
    assert_eq!(args, vec!["a", "b"]);
}
