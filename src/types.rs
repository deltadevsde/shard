use proc_macro2::Span;
use syn::{parse_str, Ident, Type};

#[derive(Debug, Clone)]
pub struct TransactionField {
    pub name: String,
    pub field_type: String,
}

impl TransactionField {
    pub fn new(name: String, field_type: String) -> Self {
        Self { name, field_type }
    }

    pub fn to_syn_field(&self) -> syn::Field {
        let field_ident = Ident::new(&self.name, Span::call_site());
        let field_type: Type =
            parse_str(&self.field_type).unwrap_or_else(|_| parse_str("String").unwrap());

        syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Inherited,
            ident: Some(field_ident),
            colon_token: Some(syn::token::Colon::default()),
            ty: field_type,
            mutability: syn::FieldMutability::None,
        }
    }
}
