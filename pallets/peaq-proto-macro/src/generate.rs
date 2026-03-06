//! Implementation of `generate_proto_file!(...)`.
//!
//! The macro reads a Rust source file at **proc-macro expansion time** (i.e. during `cargo build`),
//! parses it with `syn`, generates proto snippets for the listed types in declaration order, and
//! writes the assembled `.proto` file to disk immediately.
//!
//! `build.rs` only needs:
//! ```rust
//! println!("cargo:rerun-if-changed=src/did_spec/v0.rs");
//! ```
//! so that Cargo re-expands the macro (and rewrites the proto file) whenever the source changes.

use std::collections::HashMap;

use proc_macro::TokenStream;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    Ident, Item, LitStr, Token,
};

use crate::codegen::snippet_for_item;

// -------------------------------------------------------------------------------------------------
// Input grammar
// -------------------------------------------------------------------------------------------------

struct Input {
    /// Path to the Rust source file to parse (relative to `CARGO_MANIFEST_DIR`).
    source: String,
    /// Output `.proto` file path (relative to `CARGO_MANIFEST_DIR`).
    path: String,
    syntax: String,
    package: String,
    /// Ordered list of type names to include in the proto file.
    types: Vec<Ident>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut source = String::new();
        let mut path = String::from("generated.proto");
        let mut syntax = String::from("proto3");
        let mut package = String::new();
        let mut types: Vec<Ident> = vec![];

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "source" => {
                    let lit: LitStr = input.parse()?;
                    source = lit.value();
                }
                "path" => {
                    let lit: LitStr = input.parse()?;
                    path = lit.value();
                }
                "syntax" => {
                    let lit: LitStr = input.parse()?;
                    syntax = lit.value();
                }
                "package" => {
                    let lit: LitStr = input.parse()?;
                    package = lit.value();
                }
                "types" => {
                    let content;
                    bracketed!(content in input);
                    while !content.is_empty() {
                        types.push(content.parse()?);
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("generate_proto_file!: unknown key `{}`", other),
                    ))
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if source.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "generate_proto_file!: `source` is required",
            ));
        }

        Ok(Input { source, path, syntax, package, types })
    }
}

// -------------------------------------------------------------------------------------------------
// Expansion
// -------------------------------------------------------------------------------------------------

pub fn expand(input: TokenStream) -> TokenStream {
    let Input { source, path, syntax, package, types } =
        syn::parse_macro_input!(input as Input);

    // Resolve paths relative to the crate that invokes the macro.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("generate_proto_file!: CARGO_MANIFEST_DIR not set");
    let manifest_dir = std::path::Path::new(&manifest_dir);

    let source_path = manifest_dir.join(&source);
    let output_path = manifest_dir.join(&path);

    // Parse the source file.
    let source_code = std::fs::read_to_string(&source_path).unwrap_or_else(|e| {
        panic!(
            "generate_proto_file!: cannot read `{}`: {}",
            source_path.display(),
            e
        )
    });
    let file_ast = syn::parse_file(&source_code).unwrap_or_else(|e| {
        panic!(
            "generate_proto_file!: cannot parse `{}`: {}",
            source_path.display(),
            e
        )
    });

    // Build a name → Item map for fast lookup.
    let item_map: HashMap<String, &Item> = file_ast
        .items
        .iter()
        .filter_map(|item| item_name(item).map(|name| (name, item)))
        .collect();

    // Generate snippets in the order specified by `types`.
    let mut snippets: Vec<String> = vec![];
    for type_ident in &types {
        let name = type_ident.to_string();
        let item = item_map.get(&name).unwrap_or_else(|| {
            panic!(
                "generate_proto_file!: type `{}` not found in `{}`",
                name,
                source_path.display()
            )
        });
        let snippet = snippet_for_item(item).unwrap_or_else(|| {
            panic!(
                "generate_proto_file!: type `{}` is not a struct or enum",
                name
            )
        });
        snippets.push(snippet);
    }

    // Assemble and write the proto file.
    let header = format!("syntax = \"{}\";\npackage {};", syntax, package);
    let mut parts = vec![header];
    parts.extend(snippets);
    let content = parts.join("\n\n");

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            panic!(
                "generate_proto_file!: cannot create directory `{}`: {}",
                parent.display(),
                e
            )
        });
    }
    std::fs::write(&output_path, &content).unwrap_or_else(|e| {
        panic!(
            "generate_proto_file!: cannot write `{}`: {}",
            output_path.display(),
            e
        )
    });

    // Emit nothing — file generation is the side effect.
    // (The PROTO_SNIPPET_* constants are still emitted by #[derive(ToProto)] on each type.)
    TokenStream::new()
}

// -------------------------------------------------------------------------------------------------
// Helper
// -------------------------------------------------------------------------------------------------

fn item_name(item: &Item) -> Option<String> {
    match item {
        Item::Struct(s) => Some(s.ident.to_string()),
        Item::Enum(e) => Some(e.ident.to_string()),
        _ => None,
    }
}
