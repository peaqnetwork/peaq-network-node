//! Procedural macros for generating protobuf definitions from Rust data structures.
//!
//! # Overview
//!
//! Two macros are provided:
//!
//! - `#[derive(ToProto)]` — annotate a struct or enum with `#[proto(...)]` helper attributes to
//!   declare protobuf field numbers and types.  The derive emits a `pub const
//!   PROTO_SNIPPET_<TypeName>: &str` containing the corresponding `.proto` snippet.
//!
//! - `generate_proto_file!(...)` — collects previously emitted `PROTO_SNIPPET_*` constants,
//!   prefixes them with a `syntax` / `package` header, and generates a `write_proto_file()`
//!   function (std-only) plus a companion `#[test]` that writes the assembled `.proto` file to disk
//!   when you run `cargo test`.
//!
//! # Attribute Schema
//!
//! ## Item level (on the struct / enum itself)
//! ```text
//! #[proto(name = "OverrideName")]   // optional: override the proto message / enum name
//! #[proto(oneof = "field_name")]    // data enum only: name of the oneof field in the wrapper msg
//! ```
//!
//! ## Field level (named struct fields)
//! ```text
//! #[proto(field = 1, ty = "bytes")]            // required: field number + proto type
//! #[proto(field = 3, ty = "MyMsg", repeated)]  // repeated (BoundedVec<T,_> → repeated T)
//! ```
//!
//! ## Variant level — unit enum
//! ```text
//! #[proto(value = 0)]   // required: enum value number
//! ```
//!
//! ## Variant level — data enum (oneof)
//! ```text
//! #[proto(field = 1, ty = "bytes")]   // required: oneof field number + type
//! ```

use proc_macro::TokenStream;

mod attr;
mod codegen;
mod convert;
mod derive;
mod generate;

/// Derive macro that emits a `pub const PROTO_SNIPPET_<TypeName>: &str` for each annotated type.
///
/// Supports structs (→ `message`), unit enums (→ `enum`), and data enums (→ `message` + `oneof`).
/// Helper attribute: `#[proto(...)]` — see crate-level docs for the full schema.
#[proc_macro_derive(ToProto, attributes(proto))]
pub fn derive_to_proto(input: TokenStream) -> TokenStream {
	derive::expand(input)
}

/// Function-like macro that assembles `PROTO_SNIPPET_*` constants into a complete `.proto` file.
///
/// Generates:
/// - `#[cfg(feature = "std")] pub fn write_proto_file()` — writes the file to the given path.
/// - `#[cfg(all(test, feature = "std"))] mod __proto_gen { #[test] fn generate_proto() { ... } }`
///
/// # Example
/// ```rust,ignore
/// generate_proto_file! {
///     path    = "src/did_spec/did.proto",
///     syntax  = "proto3",
///     package = "peaq.did.v0",
///     snippets = [
///         PROTO_SNIPPET_VerificationType,
///         PROTO_SNIPPET_ServiceEndpoint,
///         PROTO_SNIPPET_VerificationMethod,
///         PROTO_SNIPPET_Permissions,
///         PROTO_SNIPPET_Controller,
///         PROTO_SNIPPET_DidDocument,
///     ]
/// }
/// ```
#[proc_macro]
pub fn generate_proto_file(input: TokenStream) -> TokenStream {
	generate::expand(input)
}
