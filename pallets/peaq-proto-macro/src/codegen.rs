//! Shared proto snippet generation logic.
//!
//! Used by both `#[derive(ToProto)]` (operates on a live token stream) and
//! `generate_proto_file!()` (operates on a parsed source file).

use std::collections::HashMap;
use syn::{DataEnum, Fields, Item, ItemEnum, ItemStruct};

use crate::attr::{
    parse_proto_attrs, sanitize_field_name, to_snake_case, to_upper_snake_case,
};

// -------------------------------------------------------------------------------------------------
// Top-level dispatcher
// -------------------------------------------------------------------------------------------------

/// Generate a proto snippet for a single `syn::Item`.
/// Returns `None` if the item is not a struct or enum, or if it lacks `#[derive(ToProto)]`.
pub fn snippet_for_item(item: &Item) -> Option<String> {
    match item {
        Item::Struct(s) => Some(snippet_for_struct(s)),
        Item::Enum(e) => Some(snippet_for_enum(e)),
        _ => None,
    }
}

fn snippet_for_struct(s: &ItemStruct) -> String {
    let item_attrs = parse_proto_attrs(&s.attrs);
    let proto_name = item_attrs
        .get("name")
        .cloned()
        .unwrap_or_else(|| s.ident.to_string());
    generate_message(&proto_name, &s.fields)
}

fn snippet_for_enum(e: &ItemEnum) -> String {
    let item_attrs = parse_proto_attrs(&e.attrs);
    let proto_name = item_attrs
        .get("name")
        .cloned()
        .unwrap_or_else(|| e.ident.to_string());
    let has_data = e.variants.iter().any(|v| !matches!(v.fields, Fields::Unit));
    if has_data {
        generate_oneof_message(&proto_name, &item_attrs, &DataEnum {
            enum_token: e.enum_token,
            brace_token: e.brace_token,
            variants: e.variants.clone(),
        })
    } else {
        generate_enum(&proto_name, &DataEnum {
            enum_token: e.enum_token,
            brace_token: e.brace_token,
            variants: e.variants.clone(),
        })
    }
}

// -------------------------------------------------------------------------------------------------
// struct → proto message
// -------------------------------------------------------------------------------------------------

pub fn generate_message(proto_name: &str, fields: &Fields) -> String {
    let named = match fields {
        Fields::Named(n) => n,
        _ => panic!(
            "ToProto: only named-field structs are supported as proto messages (in `{}`)",
            proto_name
        ),
    };

    let mut lines = vec![format!("message {} {{", proto_name)];

    for field in &named.named {
        let rust_name = field.ident.as_ref().unwrap().to_string();
        let attrs = parse_proto_attrs(&field.attrs);

        let field_number = attrs
            .get("field")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                panic!(
                    "ToProto: field `{}` in `{}` is missing #[proto(field = N)]",
                    rust_name, proto_name
                )
            });

        let proto_type = attrs.get("ty").cloned().unwrap_or_else(|| {
            panic!(
                "ToProto: field `{}` in `{}` is missing #[proto(ty = \"T\")]",
                rust_name, proto_name
            )
        });

        let repeated = attrs.contains_key("repeated");
        let prefix = if repeated { "repeated " } else { "" };
        let clean_name = sanitize_field_name(&rust_name);

        lines.push(format!(
            "  {}{} {} = {};",
            prefix, proto_type, clean_name, field_number
        ));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

// -------------------------------------------------------------------------------------------------
// unit enum → proto enum
// -------------------------------------------------------------------------------------------------

pub fn generate_enum(proto_name: &str, data: &DataEnum) -> String {
    let mut lines = vec![format!("enum {} {{", proto_name)];

    for variant in &data.variants {
        let variant_name = variant.ident.to_string();
        let attrs = parse_proto_attrs(&variant.attrs);

        let value = attrs
            .get("value")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                panic!(
                    "ToProto: variant `{}` in enum `{}` is missing #[proto(value = N)]",
                    variant_name, proto_name
                )
            });

        lines.push(format!("  {} = {};", to_upper_snake_case(&variant_name), value));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

// -------------------------------------------------------------------------------------------------
// data enum → proto message { oneof { ... } }
// -------------------------------------------------------------------------------------------------

pub fn generate_oneof_message(
    proto_name: &str,
    item_attrs: &HashMap<String, String>,
    data: &DataEnum,
) -> String {
    let oneof_field = item_attrs
        .get("oneof")
        .cloned()
        .unwrap_or_else(|| format!("{}_type", to_snake_case(proto_name)));

    let mut lines = vec![
        format!("message {} {{", proto_name),
        format!("  oneof {} {{", oneof_field),
    ];

    for variant in &data.variants {
        let variant_name = variant.ident.to_string();
        let attrs = parse_proto_attrs(&variant.attrs);

        let field_number = attrs
            .get("field")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                panic!(
                    "ToProto: variant `{}` in oneof `{}` is missing #[proto(field = N)]",
                    variant_name, proto_name
                )
            });

        let proto_type = attrs.get("ty").cloned().unwrap_or_else(|| {
            panic!(
                "ToProto: variant `{}` in oneof `{}` is missing #[proto(ty = \"T\")]",
                variant_name, proto_name
            )
        });

        lines.push(format!(
            "    {} {} = {};",
            proto_type,
            to_snake_case(&variant_name),
            field_number
        ));
    }

    lines.push("  }".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}
