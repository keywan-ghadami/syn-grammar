use syn::parse::Parser;
use syn_grammar::grammar;
use syn_grammar::testing::Testable;

grammar! {
    grammar cxx_extern_block {
        pub top_level_mod -> () =
            outer_attrs
            "mod" any_ident
            { extern_block* }
            -> { () }

        extern_block -> () =
            "unsafe"?
            "extern" lit_str
            { cxx_item* }
            -> { () }

        cxx_item -> () =
            outer_attrs "fn" any_ident generics?
            paren(args:cxx_arg_list?)
            return_type ";" -> { () }
          | mac:syn::Macro ";" -> { () }

        cxx_arg_list -> Vec<()> = items:separated(cxx_arg, ",", trailing=true) -> { items }

        cxx_arg -> () =
            any_ident ":" rust_type -> { () }
    }
}

#[test]
fn parses_cxx_style_extern_block() {
    let input = r#"
        mod ffi {
            unsafe extern "C++" {
                include!("engine/core/events.h");
                
                fn dispatch_event_with_callback<'a, 'b>(
                    self: Pin<&mut EventDispatcher>,
                    payload: &'a mut CxxVector<EventPayload<'b>>,
                    filter: fn(&CxxString, Option<&[u8]>) -> bool,
                ) -> Result<UniquePtr<DispatchReceipt>>;
            }
        }
    "#;

    cxx_extern_block::parse_top_level_mod
        .parse_str(input)
        .test()
        .assert_success();
}
