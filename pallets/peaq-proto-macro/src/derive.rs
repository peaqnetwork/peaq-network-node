//! Implementation of `#[derive(ToProto)]`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attr::parse_proto_attrs;
use crate::codegen::{generate_enum, generate_message, generate_oneof_message};

pub fn expand(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("ToProto: failed to parse input");
    let rust_name = &ast.ident;

    let item_attrs = parse_proto_attrs(&ast.attrs);
    let proto_name = item_attrs
        .get("name")
        .cloned()
        .unwrap_or_else(|| rust_name.to_string());

    let snippet: String = match &ast.data {
        Data::Struct(data) => generate_message(&proto_name, &data.fields),
        Data::Enum(data) => {
            let has_data = data
                .variants
                .iter()
                .any(|v| !matches!(v.fields, Fields::Unit));
            if has_data {
                generate_oneof_message(&proto_name, &item_attrs, data)
            } else {
                generate_enum(&proto_name, data)
            }
        }
        Data::Union(_) => panic!("ToProto does not support union types"),
    };

    let const_ident = syn::Ident::new(
        &format!("PROTO_SNIPPET_{}", rust_name),
        rust_name.span(),
    );

    let expanded = quote! {
        #[allow(non_upper_case_globals, dead_code)]
        pub const #const_ident: &str = #snippet;
    };

    expanded.into()
}
