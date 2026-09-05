//! A parser for the `#[cxx::bridge]` interface definition language, written
//! entirely in `syn-grammar`.
//!
//! This crate is the acceptance benchmark of the workspace (see `GOALS.md`):
//! the grammar below is not a toy, it covers the bridge language as the
//! [cxx book](https://cxx.rs) documents it — shared structs and enums,
//! `extern "C++"` and `extern "Rust"` blocks, opaque types and type aliases,
//! free functions and methods with all four receiver forms, `unsafe` functions,
//! `impl` blocks for the generic C++ containers, and the attributes
//! (`#[namespace]`, `#[rust_name]`, `#[cxx_name]`, doc comments) that carry the
//! bridge's semantics.
//!
//! What makes this a real test rather than a demo is the *boundary*: the bridge
//! IDL has no delimiters separating it from Rust. After a `:` or a `->` the
//! input simply becomes a Rust type, and the parser has to hand over to `syn`
//! mid-rule and take back over afterwards — `Pin<&'a mut CxxVector<T>>`,
//! `fn(&CxxString, Option<&[u8]>) -> bool`, `*mut u8`, `&'a [u8]`.
//!
//! It is a test crate, not an FFI tool: it parses the bridge, it does not
//! generate any C++.

#![warn(missing_docs)]

use syn::parse::{Parse, ParseStream, Result};
use syn::{Attribute, Generics, Ident, ItemUse, LitStr, Path, ReturnType, Type, Visibility};
use syn_grammar::grammar;
use syn_grammar::rt::{step, take_single, ParseError, Stream, StreamResult};

// ---------------------------------------------------------------------------
// 1. AST
// ---------------------------------------------------------------------------

/// A `mod ffi { … }` block carrying the bridge.
#[derive(Debug)]
pub struct FfiMod {
    /// Attributes on the module, including `#[cxx::bridge(namespace = "…")]`.
    pub attrs: Vec<Attribute>,
    /// The module's visibility.
    pub vis: Visibility,
    /// The module's name, conventionally `ffi`.
    pub name: Ident,
    /// Everything declared inside the module, in source order.
    pub items: Vec<ModItem>,
}

impl FfiMod {
    /// The shared structs declared in the bridge, in source order.
    pub fn structs(&self) -> impl Iterator<Item = &SharedStruct> {
        self.items.iter().filter_map(|i| match i {
            ModItem::Struct(s) => Some(s),
            _ => None,
        })
    }

    /// The shared enums declared in the bridge, in source order.
    pub fn enums(&self) -> impl Iterator<Item = &SharedEnum> {
        self.items.iter().filter_map(|i| match i {
            ModItem::Enum(e) => Some(e),
            _ => None,
        })
    }

    /// The `extern` blocks of the bridge, in source order.
    pub fn extern_blocks(&self) -> impl Iterator<Item = &ExternBlock> {
        self.items.iter().filter_map(|i| match i {
            ModItem::Extern(b) => Some(b),
            _ => None,
        })
    }

    /// The explicit instantiations (`impl UniquePtr<T> {}`), in source order.
    pub fn impls(&self) -> impl Iterator<Item = &ImplBlock> {
        self.items.iter().filter_map(|i| match i {
            ModItem::Impl(b) => Some(&**b),
            _ => None,
        })
    }
}

/// An item directly inside the bridge module.
#[derive(Debug)]
pub enum ModItem {
    /// A `use` statement, parsed by `syn`.
    Use(Box<ItemUse>),
    /// A struct shared between both languages.
    Struct(SharedStruct),
    /// An enum shared between both languages.
    Enum(SharedEnum),
    /// An `extern "C++"` or `extern "Rust"` block.
    Extern(ExternBlock),
    /// An explicit instantiation such as `impl UniquePtr<Blobstore> {}`.
    Impl(Box<ImplBlock>),
}

/// A struct whose layout both languages agree on.
#[derive(Debug)]
pub struct SharedStruct {
    /// Attributes and doc comments on the struct.
    pub attrs: Vec<Attribute>,
    /// The struct's name.
    pub name: Ident,
    /// The fields, in declaration order.
    pub fields: Vec<Field>,
}

/// A field of a [`SharedStruct`].
#[derive(Debug)]
pub struct Field {
    /// Attributes and doc comments on the field.
    pub attrs: Vec<Attribute>,
    /// The field's name.
    pub name: Ident,
    /// The field's type — an arbitrary Rust type, parsed by `syn`.
    pub ty: Type,
}

/// An enum shared between both languages.
#[derive(Debug)]
pub struct SharedEnum {
    /// Attributes on the enum, `#[repr(i32)]` among them.
    pub attrs: Vec<Attribute>,
    /// The enum's name.
    pub name: Ident,
    /// The variants, in declaration order.
    pub variants: Vec<Variant>,
}

/// A variant of a [`SharedEnum`], with an optional explicit discriminant.
#[derive(Debug)]
pub struct Variant {
    /// Attributes and doc comments on the variant.
    pub attrs: Vec<Attribute>,
    /// The variant's name.
    pub name: Ident,
    /// The discriminant, if one was written (`Red = -1`).
    pub discriminant: Option<i64>,
}

/// The language an `extern` block bridges to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// `extern "C++"` — items implemented on the C++ side.
    Cxx,
    /// `extern "Rust"` — items implemented on the Rust side.
    Rust,
}

/// An `extern "…" { … }` block.
#[derive(Debug)]
pub struct ExternBlock {
    /// Attributes on the block, `#[namespace = "…"]` among them.
    pub attrs: Vec<Attribute>,
    /// Whether the block is declared `unsafe extern`.
    pub is_unsafe: bool,
    /// Which side implements the items.
    pub lang: Lang,
    /// The items, in declaration order.
    pub items: Vec<ExternItem>,
}

impl ExternBlock {
    /// The functions declared in this block, in source order.
    pub fn fns(&self) -> impl Iterator<Item = &ForeignFn> {
        self.items.iter().filter_map(|i| match i {
            ExternItem::Fn(f) => Some(f),
            _ => None,
        })
    }
}

/// An item inside an [`ExternBlock`].
#[derive(Debug)]
pub enum ExternItem {
    /// `include!("path/to/header.h");`
    Include(LitStr),
    /// An opaque type: `type BlobstoreClient;`
    Opaque(TypeDecl),
    /// A type alias to a Rust type: `type MultiBuf = crate::MultiBuf;`
    Alias(TypeAlias),
    /// A function or method declaration.
    Fn(ForeignFn),
}

/// An opaque type declared in an extern block.
#[derive(Debug)]
pub struct TypeDecl {
    /// Attributes and doc comments on the declaration.
    pub attrs: Vec<Attribute>,
    /// The type's name.
    pub name: Ident,
    /// Lifetime parameters, if any (`type Payload<'a>;`).
    pub generics: Generics,
}

/// A type alias bridging a name to an existing Rust type.
#[derive(Debug)]
pub struct TypeAlias {
    /// Attributes and doc comments on the alias.
    pub attrs: Vec<Attribute>,
    /// The name used inside the bridge.
    pub name: Ident,
    /// Lifetime parameters, if any.
    pub generics: Generics,
    /// The path the alias points at, parsed by `syn`.
    pub path: Path,
}

/// A function declared in an extern block.
#[derive(Debug)]
pub struct ForeignFn {
    /// Attributes on the function, `#[rust_name = "…"]` among them.
    pub attrs: Vec<Attribute>,
    /// Whether the function is declared `unsafe fn`.
    pub is_unsafe: bool,
    /// The function's name.
    pub name: Ident,
    /// Lifetime parameters, if any.
    pub generics: Generics,
    /// Receiver and arguments, in declaration order.
    pub params: Vec<Param>,
    /// The return type, `ReturnType::Default` when none was written.
    pub ret: ReturnType,
}

impl ForeignFn {
    /// The receiver, if the first parameter is one — i.e. whether this is a
    /// method rather than a free function.
    pub fn receiver(&self) -> Option<&Receiver> {
        match self.params.first() {
            Some(Param::Receiver(r)) => Some(r),
            _ => None,
        }
    }

    /// The parameters that are arguments rather than the receiver.
    pub fn args(&self) -> impl Iterator<Item = &FnArg> {
        self.params.iter().filter_map(|p| match p {
            Param::Arg(a) => Some(&**a),
            Param::Receiver(_) => None,
        })
    }
}

/// One entry in a function's parameter list.
#[derive(Debug)]
pub enum Param {
    /// A receiver — only meaningful as the first parameter.
    Receiver(Receiver),
    /// A named argument.
    Arg(Box<FnArg>),
}

/// The receiver of a method, in any of the forms cxx accepts.
#[derive(Debug)]
pub enum Receiver {
    /// `&self` or `&mut self`.
    Ref {
        /// Whether the reference is mutable.
        mutable: bool,
    },
    /// `self: &Blobstore` or `self: Pin<&mut Blobstore>`.
    Typed(Box<Type>),
}

/// A named function argument.
#[derive(Debug)]
pub struct FnArg {
    /// The argument's name.
    pub name: Ident,
    /// The argument's type — an arbitrary Rust type, parsed by `syn`.
    pub ty: Type,
}

/// An explicit instantiation such as `impl UniquePtr<Blobstore> {}`.
#[derive(Debug)]
pub struct ImplBlock {
    /// Lifetime parameters, if any (`impl<'a> CxxVector<Payload<'a>> {}`).
    pub generics: Generics,
    /// The instantiated type, parsed by `syn`.
    pub ty: Type,
}

// ---------------------------------------------------------------------------
// 2. A hand-written rule
// ---------------------------------------------------------------------------

/// The language string of an `extern` block.
///
/// This is an `extern rule`: the grammar can match *a* string literal, but not
/// its content, and `extern "Java"` should not be reported as `expected string
/// literal` when a string literal is exactly what stands there. Written by hand
/// against the same runtime the generated code uses, so the error still carries
/// the position and the rule stack of the grammar that called it.
fn extern_lang<'a>(input: &Stream<'a>) -> StreamResult<'a, Lang> {
    // Before the step, so the error points at the literal and not past it.
    let at = input.cursor();
    let lit = step(input, take_single::<LitStr>)?;
    match lit.value().as_str() {
        "C++" => Ok(Lang::Cxx),
        "Rust" => Ok(Lang::Rust),
        other => Err(ParseError::at_cursor(
            at,
            format!(r#"expected "C++" or "Rust", found "{other}""#),
        )),
    }
}

// ---------------------------------------------------------------------------
// 3. Grammar
// ---------------------------------------------------------------------------

grammar! {
    grammar CxxParser {
        extern rule extern_lang -> Lang;

        /// The bridge module itself.
        pub top_level_mod -> FfiMod =
            attrs:outer_attrs vis:visibility "mod" => name:ident
            { items:mod_item* }
            -> { FfiMod { attrs, vis, name: name.into(), items } }

        mod_item -> ModItem =
            u:syn::ItemUse    # "a use statement"  -> { ModItem::Use(Box::new(u)) }
          | s:shared_struct   # "a shared struct"  -> { ModItem::Struct(s) }
          | e:shared_enum     # "a shared enum"    -> { ModItem::Enum(e) }
          | b:extern_block    # "an extern block"  -> { ModItem::Extern(b) }
          | i:impl_block      # "an impl block"    -> { ModItem::Impl(Box::new(i)) }

        // --- shared types ----------------------------------------------------

        shared_struct -> SharedStruct =
            attrs:outer_attrs "struct" => name:ident
            { fields:separated(field, ",", trailing=true, item_label="struct field") }
            -> { SharedStruct { attrs, name: name.into(), fields } }

        field -> Field =
            attrs:outer_attrs name:ident ":" ty:syn::Type
            -> { Field { attrs, name: name.into(), ty } }

        shared_enum -> SharedEnum =
            attrs:outer_attrs "enum" => name:ident
            { variants:separated(variant, ",", trailing=true, item_label="enum variant") }
            -> { SharedEnum { attrs, name: name.into(), variants } }

        variant -> Variant =
            attrs:outer_attrs name:ident discriminant:discriminant?
            -> { Variant { attrs, name: name.into(), discriminant } }

        discriminant -> i64 = "=" => v:i64 -> { v }

        // --- extern blocks ---------------------------------------------------

        extern_block -> ExternBlock =
            attrs:outer_attrs is_unsafe:"unsafe"? "extern" => lang:extern_lang
            { items:extern_item* }
            -> {
                ExternBlock {
                    attrs,
                    is_unsafe: is_unsafe.is_some(),
                    lang,
                    items,
                }
            }

        extern_item -> ExternItem =
            i:include_item  # "an include"         -> { ExternItem::Include(i) }
          | t:type_item     # "a type declaration"  -> { t }
          | f:foreign_fn    # "a function"          -> { ExternItem::Fn(f) }

        // `include!` is the only macro the bridge language allows, so it is
        // matched by name instead of by `syn::Macro`: `printn!("x.h");` then
        // reports `expected `include``, not a puzzling item error.
        include_item -> LitStr =
            "include" "!" => paren(header:lit_str) ";" -> { header }

        // Opaque type and alias share their whole prefix; splitting them into
        // two alternatives would report the second one's `=` as the expectation
        // for a plain `type Foo;`.
        type_item -> ExternItem =
            attrs:outer_attrs "type" => name:ident generics:syn::Generics?
            path:type_alias_target? ";"
            -> {
                let name = name.into();
                let generics = generics.unwrap_or_default();
                match path {
                    Some(path) => ExternItem::Alias(TypeAlias { attrs, name, generics, path }),
                    None => ExternItem::Opaque(TypeDecl { attrs, name, generics }),
                }
            }

        type_alias_target -> Path = "=" => p:syn::Path -> { p }

        foreign_fn -> ForeignFn =
            attrs:outer_attrs is_unsafe:"unsafe"? "fn" => name:ident generics:syn::Generics?
            paren(params:separated(param, ",", trailing=true, item_label="function parameter"))
            ret:syn::ReturnType ";"
            -> {
                ForeignFn {
                    attrs,
                    is_unsafe: is_unsafe.is_some(),
                    name: name.into(),
                    generics: generics.unwrap_or_default(),
                    params,
                    ret,
                }
            }

        // `ident` rejects keywords, so `self` can only be read by the receiver
        // alternatives - the three forms are unambiguous.
        param -> Param =
            "&" mutable:"mut"? "self"   -> { Param::Receiver(Receiver::Ref { mutable: mutable.is_some() }) }
          | "self" ":" => ty:syn::Type  -> { Param::Receiver(Receiver::Typed(Box::new(ty))) }
          | name:ident ":" => ty:syn::Type -> { Param::Arg(Box::new(FnArg { name: name.into(), ty })) }

        // --- explicit instantiations ----------------------------------------

        impl_block -> ImplBlock =
            "impl" => generics:syn::Generics? ty:syn::Type { empty_body }
            -> { ImplBlock { generics: generics.unwrap_or_default(), ty } }

        // cxx only takes the instantiation, never a body. `eof` alone would
        // report `expected end of input`, which does not say why.
        empty_body -> () =
            eof -> { () }
          | fail("an `impl` in a bridge declares an instantiation and stays empty")
    }
}

// ---------------------------------------------------------------------------
// 4. `Parse` for the root node
// ---------------------------------------------------------------------------

impl Parse for FfiMod {
    fn parse(input: ParseStream) -> Result<Self> {
        // Delegate to the parser function generated by `syn-grammar`.
        CxxParser::parse_top_level_mod(input)
    }
}
