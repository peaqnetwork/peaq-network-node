//! Implementation of `generate_proto_file!(...)`.
//!
//! The macro reads a Rust source file at **proc-macro expansion time** (i.e. during `cargo build`),
//! parses it with `syn`, generates proto snippets for the listed types in declaration order, and
//! writes the assembled `.proto` file to disk immediately.
//!
//! When `proto_mod` and `native_mod` are also provided the macro additionally emits
//! `#[cfg(feature = "std")] From / TryFrom` impls for every type in `types = [...]`,
//! derived automatically from the `#[proto(...)]` field annotations.
//!
//! `build.rs` only needs:
//! ```rust
//! println!("cargo:rerun-if-changed=src/did_spec/v0.rs");
//! ```
//! so that Cargo re-expands the macro (and rewrites the proto file) whenever the source changes.

use std::collections::HashMap;

use proc_macro::TokenStream;
use syn::{
	bracketed, parenthesized,
	parse::{Parse, ParseStream},
	Ident, Item, LitStr, Token,
};

use crate::{codegen::snippet_for_item, convert::conversions_for_types};

// -------------------------------------------------------------------------------------------------
// Input grammar
// -------------------------------------------------------------------------------------------------

struct Input {
	/// Path to the Rust source file to parse (relative to `CARGO_MANIFEST_DIR`).
	/// When `None`, no source file is read — only `imports` and `snippets` are used.
	source: Option<String>,
	/// Output `.proto` file path (relative to `CARGO_MANIFEST_DIR`).
	path: String,
	syntax: String,
	package: String,
	/// Ordered list of type names to auto-generate from the source file.
	types: Vec<Ident>,
	/// Proto import paths to emit as `import "...";` lines in the header.
	imports: Vec<String>,
	/// Pre-built proto snippets (raw strings) appended after auto-generated type snippets.
	snippets: Vec<String>,
	/// Path to the prost-generated module (e.g. `crate::proto_gen::v0`).
	/// When present together with `native_mod`, the macro emits `From`/`TryFrom` impls.
	proto_mod: Option<syn::Path>,
	/// Path to the native pallet types module (e.g. `crate::did_spec::v0`).
	native_mod: Option<syn::Path>,
}

impl Parse for Input {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut source: Option<String> = None;
		let mut path = String::from("generated.proto");
		let mut syntax = String::from("proto3");
		let mut package = String::new();
		let mut types: Vec<Ident> = vec![];
		let mut imports: Vec<String> = vec![];
		let mut snippets: Vec<String> = vec![];
		let mut proto_mod: Option<syn::Path> = None;
		let mut native_mod: Option<syn::Path> = None;

		while !input.is_empty() {
			let key: Ident = input.parse()?;
			input.parse::<Token![=]>()?;

			match key.to_string().as_str() {
				"source" => {
					let lit: LitStr = input.parse()?;
					source = Some(lit.value());
				},
				"path" => {
					let lit: LitStr = input.parse()?;
					path = lit.value();
				},
				"syntax" => {
					let lit: LitStr = input.parse()?;
					syntax = lit.value();
				},
				"package" => {
					let lit: LitStr = input.parse()?;
					package = lit.value();
				},
				"types" => {
					let content;
					bracketed!(content in input);
					while !content.is_empty() {
						types.push(content.parse()?);
						if content.peek(Token![,]) {
							content.parse::<Token![,]>()?;
						}
					}
				},
				"imports" => {
					let content;
					bracketed!(content in input);
					while !content.is_empty() {
						let lit: LitStr = content.parse()?;
						imports.push(lit.value());
						if content.peek(Token![,]) {
							content.parse::<Token![,]>()?;
						}
					}
				},
				"snippets" => {
					let content;
					bracketed!(content in input);
					while !content.is_empty() {
						snippets.push(eval_string_expr(&content)?);
						if content.peek(Token![,]) {
							content.parse::<Token![,]>()?;
						}
					}
				},
				"proto_mod" => {
					proto_mod = Some(input.parse()?);
				},
				"native_mod" => {
					native_mod = Some(input.parse()?);
				},
				other =>
					return Err(syn::Error::new(
						key.span(),
						format!("generate_proto_file!: unknown key `{}`", other),
					)),
			}

			if input.peek(Token![,]) {
				input.parse::<Token![,]>()?;
			}
		}

		Ok(Input { source, path, syntax, package, types, imports, snippets, proto_mod, native_mod })
	}
}

// -------------------------------------------------------------------------------------------------
// String expression evaluator
// -------------------------------------------------------------------------------------------------

/// Evaluate a compile-time string expression in a snippet value position.
///
/// Handles:
/// - `"literal"` → the literal string value
/// - `concat!(expr, expr, ...)` → recursively evaluated and concatenated
/// - `stringify!(tokens)` → the token stream converted to a string (single-token identifiers/ints)
fn eval_string_expr(input: ParseStream) -> syn::Result<String> {
	if input.peek(LitStr) {
		let lit: LitStr = input.parse()?;
		return Ok(lit.value());
	}

	if input.peek(Ident) {
		let ident: Ident = input.parse()?;
		input.parse::<Token![!]>()?;

		let content;
		parenthesized!(content in input);

		match ident.to_string().as_str() {
			"concat" => {
				let mut result = String::new();
				while !content.is_empty() {
					result.push_str(&eval_string_expr(&content)?);
					if content.peek(Token![,]) {
						content.parse::<Token![,]>()?;
					}
				}
				Ok(result)
			},
			"stringify" => {
				let ts: proc_macro2::TokenStream = content.parse()?;
				Ok(ts.to_string())
			},
			other => Err(syn::Error::new(
				ident.span(),
				format!("generate_proto_file!: unsupported macro in snippet value: `{}`", other),
			)),
		}
	} else {
		Err(input.error(
			"generate_proto_file!: expected string literal, concat!(...), or stringify!(...)",
		))
	}
}

// -------------------------------------------------------------------------------------------------
// Expansion
// -------------------------------------------------------------------------------------------------

pub fn expand(input: TokenStream) -> TokenStream {
	let Input { source, path, syntax, package, types, imports, snippets, proto_mod, native_mod } =
		syn::parse_macro_input!(input as Input);

	// Resolve paths relative to the crate that invokes the macro.
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
		.expect("generate_proto_file!: CARGO_MANIFEST_DIR not set");
	let manifest_dir = std::path::Path::new(&manifest_dir);

	let output_path = manifest_dir.join(path);

	// --- Auto-generate snippets from source file (only when `source` is provided) ---
	let mut type_snippets: Vec<String> = vec![];
	let item_map: HashMap<String, Item>;

	if let Some(ref source_str) = source {
		let source_path = manifest_dir.join(source_str);

		let source_code = std::fs::read_to_string(&source_path).unwrap_or_else(|e| {
			panic!("generate_proto_file!: cannot read `{}`: {}", source_path.display(), e)
		});
		let file_ast = syn::parse_file(&source_code).unwrap_or_else(|e| {
			panic!("generate_proto_file!: cannot parse `{}`: {}", source_path.display(), e)
		});

		let owned_map: HashMap<String, Item> = file_ast
			.items
			.iter()
			.filter_map(|item| item_name(item).map(|name| (name, item.clone())))
			.collect();

		for type_ident in &types {
			let name = type_ident.to_string();
			let item = owned_map.get(&name).unwrap_or_else(|| {
				panic!(
					"generate_proto_file!: type `{}` not found in `{}`",
					name,
					source_path.display()
				)
			});
			let snippet = snippet_for_item(item).unwrap_or_else(|| {
				panic!("generate_proto_file!: type `{}` is not a struct or enum", name)
			});
			type_snippets.push(snippet);
		}

		item_map = owned_map;
	} else {
		item_map = HashMap::new();
	}

	// --- Assemble proto file ---
	let import_lines: String = imports.iter().map(|imp| format!("\nimport \"{}\";", imp)).collect();
	let header = format!("syntax = \"{}\";\npackage {};{}", syntax, package, import_lines);

	let mut parts = vec![header];
	parts.extend(type_snippets);
	parts.extend(snippets);
	let content = parts.join("\n\n");

	if let Some(parent) = output_path.parent() {
		std::fs::create_dir_all(parent).unwrap_or_else(|e| {
			panic!("generate_proto_file!: cannot create directory `{}`: {}", parent.display(), e)
		});
	}
	std::fs::write(&output_path, content).unwrap_or_else(|e| {
		panic!("generate_proto_file!: cannot write `{}`: {}", output_path.display(), e)
	});

	// --- Emit conversion impls when proto_mod + native_mod are specified ---
	if let (Some(p_mod), Some(n_mod)) = (proto_mod, native_mod) {
		let ref_map: HashMap<String, &Item> =
			item_map.iter().map(|(k, v)| (k.clone(), v)).collect();
		let conv_tokens = conversions_for_types(&types, &ref_map, &p_mod, &n_mod);
		conv_tokens.into()
	} else {
		TokenStream::new()
	}
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
