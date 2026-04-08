use syn_grammar::grammar;
use syn_grammar::testing::Testable;
use syn::parse::Parser;

grammar! {
    grammar repro_parser {
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

        cxx_arg_list -> Vec<()> = items:separated(cxx_arg, ",") -> { items }

        cxx_arg -> () =
            any_ident ":" rust_type -> { () }
    }
}

#[test]
fn test_repro_issue() {
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

    repro_parser::parse_top_level_mod
        .parse_str(input)
        .test()
        .assert_success();
}
