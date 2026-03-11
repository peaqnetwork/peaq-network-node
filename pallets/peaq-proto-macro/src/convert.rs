//! Code generation for `From` / `TryFrom` conversions between native pallet types and
//! prost-generated proto types.
//!
//! Invoked by `generate_proto_file!` when `proto_mod` and `native_mod` are specified.
//! All emitted impls are gated with `#[cfg(feature = "std")]`.

use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Fields, GenericParam, Ident, Item, ItemEnum, ItemStruct};

use crate::attr::{parse_proto_attrs, sanitize_field_name, to_snake_case};

// -------------------------------------------------------------------------------------------------
// Entry point
// -------------------------------------------------------------------------------------------------

/// Generate `From` / `TryFrom` impls for each type listed in `types`.
pub fn conversions_for_types(
	types: &[Ident],
	item_map: &HashMap<String, &Item>,
	proto_mod: &syn::Path,
	native_mod: &syn::Path,
) -> TokenStream {
	let mut tokens = TokenStream::new();
	for type_ident in types {
		let name = type_ident.to_string();
		if let Some(item) = item_map.get(&name) {
			tokens.extend(conversion_for_item(item, proto_mod, native_mod, item_map));
		}
	}
	tokens
}

// -------------------------------------------------------------------------------------------------
// Dispatcher
// -------------------------------------------------------------------------------------------------

fn conversion_for_item(
	item: &Item,
	proto_mod: &syn::Path,
	native_mod: &syn::Path,
	item_map: &HashMap<String, &Item>,
) -> TokenStream {
	match item {
		Item::Struct(s) => conversion_for_struct(s, proto_mod, native_mod, item_map),
		Item::Enum(e) => {
			let has_data = e.variants.iter().any(|v| !matches!(v.fields, Fields::Unit));
			if has_data {
				conversion_for_data_enum(e, proto_mod, native_mod, item_map)
			} else {
				conversion_for_unit_enum(e, proto_mod, native_mod)
			}
		},
		_ => TokenStream::new(),
	}
}

// -------------------------------------------------------------------------------------------------
// struct → From + TryFrom
// -------------------------------------------------------------------------------------------------

fn conversion_for_struct(
	s: &ItemStruct,
	proto_mod: &syn::Path,
	native_mod: &syn::Path,
	item_map: &HashMap<String, &Item>,
) -> TokenStream {
	let name = &s.ident;

	// Skip proto-only helpers (no native counterpart).
	let item_attrs = parse_proto_attrs(&s.attrs);
	if item_attrs.contains_key("proto_only") {
		return TokenStream::new();
	}

	// Collect generic type-parameter names (e.g. "AccountId").
	let generic_params: HashSet<String> =
		s.generics
			.params
			.iter()
			.filter_map(|p| {
				if let GenericParam::Type(tp) = p {
					Some(tp.ident.to_string())
				} else {
					None
				}
			})
			.collect();
	let has_generics = !generic_params.is_empty();

	let named = match &s.fields {
		Fields::Named(n) => &n.named,
		_ => return TokenStream::new(),
	};

	// --- From: native → proto ---
	let from_fields: Vec<TokenStream> = named
		.iter()
		.map(|field| {
			let rust_ident = field.ident.as_ref().unwrap();
			let rust_name = rust_ident.to_string();
			let clean = sanitize_field_name(&rust_name);
			// Proto struct field ident: use raw ident if trailing _ was stripped (e.g.
			// type_→r#type).
			let proto_ident = if clean != rust_name {
				proc_macro2::Ident::new_raw(clean, rust_ident.span())
			} else {
				rust_ident.clone()
			};

			let attrs = parse_proto_attrs(&field.attrs);
			let ty = attrs.get("ty").cloned().unwrap_or_default();
			let repeated = attrs.contains_key("repeated");

			field_from_expr(
				rust_ident,
				&proto_ident,
				&ty,
				repeated,
				&field.ty,
				&generic_params,
				proto_mod,
				item_map,
			)
		})
		.collect();

	// --- TryFrom: proto → native ---
	let try_from_fields: Vec<TokenStream> = named
		.iter()
		.map(|field| {
			let rust_ident = field.ident.as_ref().unwrap();
			let rust_name = rust_ident.to_string();
			let clean = sanitize_field_name(&rust_name);
			let proto_ident = if clean != rust_name {
				proc_macro2::Ident::new_raw(clean, rust_ident.span())
			} else {
				rust_ident.clone()
			};

			let attrs = parse_proto_attrs(&field.attrs);
			let ty = attrs.get("ty").cloned().unwrap_or_default();
			let repeated = attrs.contains_key("repeated");

			field_try_from_expr(
				rust_ident,
				&proto_ident,
				&ty,
				repeated,
				&field.ty,
				&generic_params,
				native_mod,
				proto_mod,
				item_map,
			)
		})
		.collect();

	if has_generics {
		quote! {
			#[cfg(feature = "std")]
			impl From<#native_mod::#name<::std::vec::Vec<u8>>> for #proto_mod::#name {
				fn from(s: #native_mod::#name<::std::vec::Vec<u8>>) -> Self {
					Self { #(#from_fields,)* }
				}
			}

			#[cfg(feature = "std")]
			impl TryFrom<#proto_mod::#name> for #native_mod::#name<::std::vec::Vec<u8>> {
				type Error = &'static str;
				fn try_from(s: #proto_mod::#name) -> Result<Self, Self::Error> {
					Ok(Self { #(#try_from_fields,)* })
				}
			}
		}
	} else {
		quote! {
			#[cfg(feature = "std")]
			impl From<#native_mod::#name> for #proto_mod::#name {
				fn from(s: #native_mod::#name) -> Self {
					Self { #(#from_fields,)* }
				}
			}

			#[cfg(feature = "std")]
			impl TryFrom<#proto_mod::#name> for #native_mod::#name {
				type Error = &'static str;
				fn try_from(s: #proto_mod::#name) -> Result<Self, Self::Error> {
					Ok(Self { #(#try_from_fields,)* })
				}
			}
		}
	}
}

// -------------------------------------------------------------------------------------------------
// unit enum → From + TryFrom
// -------------------------------------------------------------------------------------------------

fn conversion_for_unit_enum(
	e: &ItemEnum,
	proto_mod: &syn::Path,
	native_mod: &syn::Path,
) -> TokenStream {
	let name = &e.ident;

	let from_arms: Vec<TokenStream> = e
		.variants
		.iter()
		.map(|v| {
			let variant = &v.ident;
			quote! { #native_mod::#name::#variant => Self::#variant }
		})
		.collect();

	let try_from_arms: Vec<TokenStream> = e
		.variants
		.iter()
		.map(|v| {
			let variant = &v.ident;
			quote! { #proto_mod::#name::#variant => Ok(Self::#variant) }
		})
		.collect();

	quote! {
		#[cfg(feature = "std")]
		impl From<#native_mod::#name> for #proto_mod::#name {
			fn from(v: #native_mod::#name) -> Self {
				match v { #(#from_arms,)* }
			}
		}

		#[cfg(feature = "std")]
		impl TryFrom<#proto_mod::#name> for #native_mod::#name {
			type Error = &'static str;
			fn try_from(v: #proto_mod::#name) -> Result<Self, Self::Error> {
				match v { #(#try_from_arms,)* }
			}
		}
	}
}

// -------------------------------------------------------------------------------------------------
// data enum (oneof) → From + TryFrom
// -------------------------------------------------------------------------------------------------

fn conversion_for_data_enum(
	e: &ItemEnum,
	proto_mod: &syn::Path,
	native_mod: &syn::Path,
	item_map: &HashMap<String, &Item>,
) -> TokenStream {
	let name = &e.ident;
	let item_attrs = parse_proto_attrs(&e.attrs);

	let generic_params: HashSet<String> =
		e.generics
			.params
			.iter()
			.filter_map(|p| {
				if let GenericParam::Type(tp) = p {
					Some(tp.ident.to_string())
				} else {
					None
				}
			})
			.collect();
	let has_generics = !generic_params.is_empty();

	// Derive the prost oneof module / enum names.
	// prost: message Foo { oneof my_field { ... } }
	//   → mod foo { enum MyField { ... } }
	let oneof_field_name = item_attrs
		.get("oneof")
		.cloned()
		.unwrap_or_else(|| format!("{}_type", to_snake_case(&name.to_string())));

	let oneof_mod = Ident::new(&to_snake_case(&name.to_string()), name.span());
	let oneof_enum = Ident::new(&to_pascal_case(&oneof_field_name), name.span());
	let oneof_field_ident = Ident::new(&oneof_field_name, name.span());

	// --- From: native variant → proto oneof arm ---
	let from_arms: Vec<TokenStream> = e
		.variants
		.iter()
		.map(|v| {
			let variant_ident = &v.ident;
			let attrs = parse_proto_attrs(&v.attrs);
			let ty = attrs.get("ty").cloned().unwrap_or_default();

			let inner_ty = match &v.fields {
				Fields::Unnamed(f) => f.unnamed.first().map(|f| &f.ty),
				_ => None,
			};
			let is_generic =
				inner_ty.map(|t| is_type_generic_param(t, &generic_params)).unwrap_or(false);

			let oneof_variant = Ident::new(&variant_ident.to_string(), variant_ident.span());

			if ty == "bytes" && is_generic {
				// Generic payload (AccountId = Vec<u8>): pass through directly.
				quote! {
					#native_mod::#name::#variant_ident(v) =>
						Some(#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(v))
				}
			} else if ty == "bytes" {
				quote! {
					#native_mod::#name::#variant_ident(v) =>
						Some(#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(v.into_inner()))
				}
			} else {
				let ty_ident = Ident::new(&ty, Span::call_site());
				let is_enum = is_unit_enum_in_map(&ty, item_map);
				if is_enum {
					// Unit enum payload is stored as i32 in prost.
					quote! {
						#native_mod::#name::#variant_ident(v) =>
							Some(#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(
								#proto_mod::#ty_ident::from(v) as i32
							))
					}
				} else {
					quote! {
						#native_mod::#name::#variant_ident(v) =>
							Some(#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(
								#proto_mod::#ty_ident::from(v)
							))
					}
				}
			}
		})
		.collect();

	let missing_err = format!("{}: missing {}", name, oneof_field_name);

	// --- TryFrom: proto oneof arm → native variant ---
	let try_from_arms: Vec<TokenStream> = e
		.variants
		.iter()
		.map(|v| {
			let variant_ident = &v.ident;
			let attrs = parse_proto_attrs(&v.attrs);
			let ty = attrs.get("ty").cloned().unwrap_or_default();

			let inner_ty = match &v.fields {
				Fields::Unnamed(f) => f.unnamed.first().map(|f| &f.ty),
				_ => None,
			};
			let is_generic =
				inner_ty.map(|t| is_type_generic_param(t, &generic_params)).unwrap_or(false);

			let oneof_variant = Ident::new(&variant_ident.to_string(), variant_ident.span());

			if ty == "bytes" && is_generic {
				quote! {
					#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(v) =>
						Ok(#native_mod::#name::#variant_ident(v))
				}
			} else if ty == "bytes" {
				let overflow_err = format!("{}: {} overflow", name, variant_ident);
				quote! {
					#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(v) =>
						Ok(#native_mod::#name::#variant_ident(
							::frame_support::BoundedVec::try_from(v).map_err(|_| #overflow_err)?
						))
				}
			} else {
				let ty_ident = Ident::new(&ty, Span::call_site());
				let is_enum = is_unit_enum_in_map(&ty, item_map);
				if is_enum {
					let discriminant_err =
						format!("{}: {} unknown discriminant", name, variant_ident);
					quote! {
						#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(v) =>
							Ok(#native_mod::#name::#variant_ident(
								#native_mod::#ty_ident::try_from(
									#proto_mod::#ty_ident::try_from(v).map_err(|_| #discriminant_err)?
								)?
							))
					}
				} else {
					quote! {
						#proto_mod::#oneof_mod::#oneof_enum::#oneof_variant(v) =>
							Ok(#native_mod::#name::#variant_ident(#native_mod::#ty_ident::try_from(v)?))
					}
				}
			}
		})
		.collect();

	if has_generics {
		quote! {
			#[cfg(feature = "std")]
			impl From<#native_mod::#name<::std::vec::Vec<u8>>> for #proto_mod::#name {
				fn from(c: #native_mod::#name<::std::vec::Vec<u8>>) -> Self {
					let #oneof_field_ident = match c { #(#from_arms,)* };
					Self { #oneof_field_ident }
				}
			}

			#[cfg(feature = "std")]
			impl TryFrom<#proto_mod::#name> for #native_mod::#name<::std::vec::Vec<u8>> {
				type Error = &'static str;
				fn try_from(c: #proto_mod::#name) -> Result<Self, Self::Error> {
					match c.#oneof_field_ident.ok_or(#missing_err)? {
						#(#try_from_arms,)*
					}
				}
			}
		}
	} else {
		quote! {
			#[cfg(feature = "std")]
			impl From<#native_mod::#name> for #proto_mod::#name {
				fn from(c: #native_mod::#name) -> Self {
					let #oneof_field_ident = match c { #(#from_arms,)* };
					Self { #oneof_field_ident }
				}
			}

			#[cfg(feature = "std")]
			impl TryFrom<#proto_mod::#name> for #native_mod::#name {
				type Error = &'static str;
				fn try_from(c: #proto_mod::#name) -> Result<Self, Self::Error> {
					match c.#oneof_field_ident.ok_or(#missing_err)? {
						#(#try_from_arms,)*
					}
				}
			}
		}
	}
}

// -------------------------------------------------------------------------------------------------
// Per-field expression builders
// -------------------------------------------------------------------------------------------------

/// Build the `proto_field: <expr>` token stream for the `From` (native → proto) direction.
fn field_from_expr(
	rust_ident: &Ident,
	proto_ident: &Ident,
	ty: &str,
	repeated: bool,
	rust_ty: &syn::Type,
	generic_params: &HashSet<String>,
	proto_mod: &syn::Path,
	item_map: &HashMap<String, &Item>,
) -> TokenStream {
	if ty == "bytes" {
		if repeated {
			// BoundedVec<T, N> → Vec<T>; for repeated bytes T is Vec<u8> or AccountId (=Vec<u8>).
			quote! { #proto_ident: s.#rust_ident.into_inner() }
		} else if is_type_generic_param(rust_ty, generic_params) {
			// Generic param (AccountId = Vec<u8>): no conversion needed.
			quote! { #proto_ident: s.#rust_ident }
		} else {
			// BoundedVec<u8, N> or type-alias thereof → Vec<u8>.
			quote! { #proto_ident: s.#rust_ident.into_inner() }
		}
	} else if ty == "string" {
		// BoundedVec<u8, N> → String (lossy; data must be valid UTF-8 when stored on-chain).
		quote! { #proto_ident: ::std::string::String::from_utf8_lossy(&s.#rust_ident).into_owned() }
	} else if ty.starts_with("map<") {
		// map<string, string>: BoundedBTreeMap<BoundedVec<u8,N>,BoundedVec<u8,N>,M> →
		// HashMap<String,String>.
		quote! {
			#proto_ident: s.#rust_ident.into_inner().into_iter()
				.map(|(k, v)| (
					::std::string::String::from_utf8_lossy(&k).into_owned(),
					::std::string::String::from_utf8_lossy(&v).into_owned(),
				))
				.collect()
		}
	} else {
		let ty_ident = Ident::new(ty, Span::call_site());
		let is_enum = is_unit_enum_in_map(ty, item_map);

		if is_enum {
			if repeated {
				quote! { #proto_ident: s.#rust_ident.into_inner().into_iter().map(|x| #proto_mod::#ty_ident::from(x) as i32).collect() }
			} else {
				// Enum field is stored as i32 in prost.
				quote! { #proto_ident: #proto_mod::#ty_ident::from(s.#rust_ident) as i32 }
			}
		} else if repeated {
			quote! { #proto_ident: s.#rust_ident.into_inner().into_iter().map(#proto_mod::#ty_ident::from).collect() }
		} else {
			// Singular message field is Option<T> in prost.
			quote! { #proto_ident: Some(#proto_mod::#ty_ident::from(s.#rust_ident)) }
		}
	}
}

/// Build the `native_field: <expr>` token stream for the `TryFrom` (proto → native) direction.
#[allow(clippy::too_many_arguments)]
fn field_try_from_expr(
	rust_ident: &Ident,
	proto_ident: &Ident,
	ty: &str,
	repeated: bool,
	rust_ty: &syn::Type,
	generic_params: &HashSet<String>,
	native_mod: &syn::Path,
	proto_mod: &syn::Path,
	item_map: &HashMap<String, &Item>,
) -> TokenStream {
	let overflow_err = format!("{} overflow", rust_ident);
	let missing_err = format!("missing {}", rust_ident);
	let discriminant_err = format!("{}: unknown discriminant", rust_ident);

	if ty == "bytes" {
		if repeated {
			quote! { #rust_ident: ::frame_support::BoundedVec::try_from(s.#proto_ident).map_err(|_| #overflow_err)? }
		} else if is_type_generic_param(rust_ty, generic_params) {
			// AccountId = Vec<u8>: direct.
			quote! { #rust_ident: s.#proto_ident }
		} else {
			quote! { #rust_ident: ::frame_support::BoundedVec::try_from(s.#proto_ident).map_err(|_| #overflow_err)? }
		}
	} else if ty == "string" {
		quote! { #rust_ident: ::frame_support::BoundedVec::try_from(s.#proto_ident.into_bytes()).map_err(|_| #overflow_err)? }
	} else if ty.starts_with("map<") {
		// map<string, string>: HashMap<String,String> →
		// BoundedBTreeMap<BoundedVec<u8,N>,BoundedVec<u8,N>,M>.
		let key_overflow = format!("{}.key overflow", rust_ident);
		let val_overflow = format!("{}.val overflow", rust_ident);
		let map_overflow = format!("{} map overflow", rust_ident);
		quote! {
			#rust_ident: {
				let btree: ::std::collections::BTreeMap<_, _> = s.#proto_ident
					.into_iter()
					.map(|(k, v)| -> Result<_, &'static str> {
						let k = ::frame_support::BoundedVec::try_from(k.into_bytes())
							.map_err(|_| #key_overflow)?;
						let v = ::frame_support::BoundedVec::try_from(v.into_bytes())
							.map_err(|_| #val_overflow)?;
						Ok((k, v))
					})
					.collect::<Result<_, _>>()?;
				::frame_support::BoundedBTreeMap::try_from(btree)
					.map_err(|_| #map_overflow)?
			}
		}
	} else {
		let ty_ident = Ident::new(ty, Span::call_site());
		let is_enum = is_unit_enum_in_map(ty, item_map);

		if is_enum {
			if repeated {
				quote! {
					#rust_ident: ::frame_support::BoundedVec::try_from(
						s.#proto_ident.into_iter()
							.map(|x| #native_mod::#ty_ident::try_from(
								#proto_mod::#ty_ident::try_from(x).map_err(|_| #discriminant_err)?
							))
							.collect::<Result<::std::vec::Vec<_>, _>>()?
					).map_err(|_| #overflow_err)?
				}
			} else {
				// Enum stored as i32 in prost.
				quote! {
					#rust_ident: #native_mod::#ty_ident::try_from(
						#proto_mod::#ty_ident::try_from(s.#proto_ident).map_err(|_| #discriminant_err)?
					)?
				}
			}
		} else if repeated {
			quote! {
				#rust_ident: ::frame_support::BoundedVec::try_from(
					s.#proto_ident.into_iter()
						.map(#native_mod::#ty_ident::try_from)
						.collect::<Result<::std::vec::Vec<_>, _>>()?
				).map_err(|_| #overflow_err)?
			}
		} else {
			// Singular message: Option<T> in prost.
			quote! {
				#rust_ident: #native_mod::#ty_ident::try_from(
					s.#proto_ident.ok_or(#missing_err)?
				)?
			}
		}
	}
}

// -------------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------------

/// Returns `true` if `ty_name` refers to a unit enum in the source file.
fn is_unit_enum_in_map(ty_name: &str, item_map: &HashMap<String, &Item>) -> bool {
	if let Some(Item::Enum(e)) = item_map.get(ty_name) {
		return e.variants.iter().all(|v| matches!(v.fields, Fields::Unit));
	}
	false
}

/// Returns `true` if `ty` is a single-segment path that matches one of the generic param names
/// (e.g. `AccountId` in `struct Foo<AccountId>`).
fn is_type_generic_param(ty: &syn::Type, generic_params: &HashSet<String>) -> bool {
	if let syn::Type::Path(tp) = ty {
		if tp.qself.is_none() && tp.path.segments.len() == 1 {
			return generic_params.contains(&tp.path.segments[0].ident.to_string());
		}
	}
	false
}

/// `snake_case` → `PascalCase`  (e.g. `controller_type` → `ControllerType`).
fn to_pascal_case(s: &str) -> String {
	s.split('_')
		.map(|part| {
			let mut chars = part.chars();
			match chars.next() {
				None => String::new(),
				Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
			}
		})
		.collect()
}
