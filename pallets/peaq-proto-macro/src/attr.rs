//! Parser for `#[proto(...)]` helper attributes.

use std::collections::HashMap;
use syn::{Attribute, Lit, Meta, NestedMeta};

/// Parses all `#[proto(...)]` attributes on an item/field/variant into a flat key→value map.
///
/// - `#[proto(field = 1, ty = "bytes")]`  →  `{"field": "1", "ty": "bytes"}`
/// - `#[proto(repeated)]`                →  `{"repeated": ""}`
/// - `#[proto(value = 0)]`               →  `{"value": "0"}`
pub fn parse_proto_attrs(attrs: &[Attribute]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for attr in attrs {
        if !attr.path.is_ident("proto") {
            continue;
        }
        let meta = match attr.parse_meta() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let list = match meta {
            Meta::List(l) => l,
            _ => continue,
        };
        for nested in list.nested {
            match nested {
                NestedMeta::Meta(Meta::NameValue(nv)) => {
                    let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
                    let value = match nv.lit {
                        Lit::Str(s) => s.value(),
                        Lit::Int(i) => i.base10_digits().to_string(),
                        _ => continue,
                    };
                    map.insert(key, value);
                }
                NestedMeta::Meta(Meta::Path(p)) => {
                    // Flag attributes, e.g. `repeated`
                    if let Some(ident) = p.get_ident() {
                        map.insert(ident.to_string(), String::new());
                    }
                }
                _ => {}
            }
        }
    }
    map
}

/// Converts a `CamelCase` identifier to `UPPER_SNAKE_CASE`.
/// Used for proto enum variant names.
pub fn to_upper_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_uppercase());
    }
    result
}

/// Converts a `CamelCase` identifier to `snake_case`.
/// Used for oneof field names derived from enum variant names.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Strips a single trailing `_` from a field name.
/// Needed because Rust uses `type_` to avoid the `type` keyword, but proto wants `type`.
pub fn sanitize_field_name(name: &str) -> &str {
    name.strip_suffix('_').unwrap_or(name)
}
