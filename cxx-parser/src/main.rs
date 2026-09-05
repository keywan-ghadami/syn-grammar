//! Reads a `#[cxx::bridge]` module and prints either a summary of it or the
//! parse error.
//!
//! ```text
//! cargo run -p cxx-parser -- path/to/bridge.rs
//! cargo run -p cxx-parser            # reads stdin
//! ```
//!
//! The point of the binary is the error path: it is the shortest way to see
//! what a `syn-grammar` diagnostic looks like on real input.

use std::io::Read;

use cxx_parser::{CxxParser, ExternItem, ModItem};
use syn::parse::Parser;

fn main() -> std::process::ExitCode {
    let source = match std::env::args().nth(1) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read {path}: {e}");
                return std::process::ExitCode::FAILURE;
            }
        },
        None => {
            let mut buf = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                eprintln!("cannot read stdin: {e}");
                return std::process::ExitCode::FAILURE;
            }
            buf
        }
    };

    match CxxParser::parse_top_level_mod.parse_str(&source) {
        Ok(bridge) => {
            print_summary(&bridge);
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            // The message is multi-line: expectation, position, and the chain
            // of rules the failure happened in.
            eprintln!("error: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn print_summary(bridge: &cxx_parser::FfiMod) {
    println!("mod {}", bridge.name);
    for item in &bridge.items {
        match item {
            ModItem::Use(_) => println!("  use …"),
            ModItem::Struct(s) => println!("  struct {} ({} fields)", s.name, s.fields.len()),
            ModItem::Enum(e) => println!("  enum {} ({} variants)", e.name, e.variants.len()),
            ModItem::Impl(i) => println!("  impl {}", type_name(&i.ty)),
            ModItem::Extern(b) => {
                println!(
                    "  {}extern {:?}",
                    if b.is_unsafe { "unsafe " } else { "" },
                    b.lang
                );
                for item in &b.items {
                    match item {
                        ExternItem::Include(h) => println!("    include {:?}", h.value()),
                        ExternItem::Opaque(t) => println!("    type {}", t.name),
                        ExternItem::Alias(a) => {
                            println!("    type {} = {}", a.name, type_name(&a.path))
                        }
                        ExternItem::Fn(f) => println!(
                            "    {}fn {}({} args){}",
                            if f.is_unsafe { "unsafe " } else { "" },
                            f.name,
                            f.args().count(),
                            if f.receiver().is_some() {
                                " [method]"
                            } else {
                                ""
                            }
                        ),
                    }
                }
            }
        }
    }
}

fn type_name(t: &impl quote::ToTokens) -> String {
    quote::ToTokens::to_token_stream(t).to_string()
}
