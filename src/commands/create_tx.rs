use anyhow::{bail, Result};
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::fs;
use std::path::Path;
use syn::{
    parse2, parse_file, parse_quote, parse_str, Arm, Expr, Field, Fields, FieldsNamed, Ident, Item,
    Type, Variant,
};

use crate::types::TransactionField;

// parses command line arguments into Vec<TransactionField>
pub fn parse_fields(args: &[String]) -> Vec<TransactionField> {
    args.chunks(2)
        .map(|chunk| {
            if chunk.len() == 2 {
                TransactionField::new(chunk[0].clone(), chunk[1].clone())
            } else {
                // string as default type for now
                TransactionField::new(chunk[0].clone(), "String".to_string())
            }
        })
        .collect()
}

pub fn create_transaction(
    project_path: &str,
    tx_name: &str,
    fields: Vec<TransactionField>,
) -> Result<()> {
    let path = Path::new(project_path);
    if !path.exists() {
        bail!("Project directory not found. Make sure you're in the correct directory.");
    }

    let tx_path = path.join("src").join("tx.rs");
    let state_path = path.join("src").join("state.rs");

    let tx_content = modify_tx_file(tx_name, &fields)?;
    let state_content = modify_state_file(tx_name, &fields)?;

    fs::write(tx_path, tx_content)?;
    fs::write(state_path, state_content)?;

    print_transaction_info(tx_name, &fields);
    Ok(())
}

pub fn modify_tx_file(tx_name: &str, fields: &[TransactionField]) -> Result<String> {
    let mut ast = parse_file(&fs::read_to_string("src/tx.rs")?)?;

    // first, find the TransactionType enum in the whole file (ast)
    let transaction_enum = ast
        .items
        .iter_mut()
        .find_map(|item| match item {
            Item::Enum(item_enum) if item_enum.ident == "TransactionType" => Some(item_enum),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Couldn't find TransactionType enum"))?;

    // create the new variant for the enum
    let new_variant = if fields.is_empty() {
        // if there are no fields, we can just use the tx_name as the variant
        parse_quote! {
            #tx_name
        }
    } else {
        // otherwise, we need to create a new variant with named fields
        let syn_fields: Vec<syn::Field> = fields.iter().map(|field| field.to_syn_field()).collect();

        Variant {
            attrs: vec![],
            ident: syn::Ident::new(tx_name, Span::call_site()),
            fields: Fields::Named(FieldsNamed {
                // named fields in {} brackets
                brace_token: syn::token::Brace::default(),
                named: syn_fields.into_iter().collect(),
            }),
            discriminant: None,
        }
    };

    transaction_enum.variants.push(new_variant);

    // Remove the Noop variant from the enum if any other variants exist
    let filtered_variants: Vec<Variant> = transaction_enum
        .variants
        .iter()
        .filter(|variant| variant.ident != "Noop")
        .cloned()
        .collect();
    transaction_enum.variants.clear();
    transaction_enum.variants.extend(filtered_variants);

    // Find and modify the verify method in the impl block
    let impl_block = ast
        .items
        .iter_mut()
        .find_map(|item| match item {
            Item::Impl(impl_block) => Some(impl_block),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Could not find impl block"))?;

    let verify_method = impl_block
        .items
        .iter_mut()
        .find_map(|item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == "verify" => Some(method),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Could not find verify method"))?;

    for stmt in &mut verify_method.block.stmts {
        if let syn::Stmt::Expr(Expr::Match(match_expr), _) = stmt {
            if let Expr::Field(field_expr) = &match_expr.expr.as_ref() {
                if field_expr.member.to_token_stream().to_string() == "tx_type" {
                    let tx_name_ident = Ident::new(tx_name, Span::call_site());
                    let verify_arm: Arm = if fields.is_empty() {
                        parse2(quote! {
                            TransactionType::#tx_name_ident => Ok(())
                        })?
                    } else {
                        let field_idents = fields
                            .iter()
                            .map(|field| Ident::new(&field.name, Span::call_site()));
                        parse2(quote! {
                            TransactionType::#tx_name_ident { #(#field_idents),* } => Ok(())
                        })?
                    };

                    // Remove the existing Noop arm here as well
                    match_expr.arms.retain(|arm| {
                        if let syn::Pat::Path(path) = &arm.pat {
                            path.path.segments.last().unwrap().ident != "Noop"
                        } else {
                            true
                        }
                    });

                    match_expr.arms.push(verify_arm);
                    break;
                }
            }
        }
    }

    Ok(prettyplease::unparse(&ast))
}

pub fn modify_state_file(tx_name: &str, fields: &[TransactionField]) -> Result<String> {
    let mut ast = parse_file(&fs::read_to_string("src/state.rs")?)?;

    let impl_block = ast
        .items
        .iter_mut()
        .find_map(|item| match item {
            Item::Impl(impl_block) => Some(impl_block),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Could not find impl block"))?;

    let tx_file_ast = parse_file(&fs::read_to_string("src/tx.rs")?)?;
    let transaction_type_count = tx_file_ast
        .items
        .iter()
        .find_map(|item| match item {
            Item::Enum(item_enum) if item_enum.ident == "TransactionType" => {
                Some(item_enum.variants.len())
            }
            _ => None,
        })
        .unwrap_or(0);

    for method in &mut impl_block.items {
        if let syn::ImplItem::Fn(method_fn) = method {
            let method_name = &method_fn.sig.ident;
            if method_name == "validate_tx" || method_name == "process_tx" {
                if let syn::Stmt::Expr(Expr::Match(match_expr), _) = &mut method_fn.block.stmts[1] {
                    if transaction_type_count >= 1 {
                        match_expr.arms.retain(|arm| {
                            if let syn::Pat::Path(path) = &arm.pat {
                                path.path.segments.last().unwrap().ident != "Noop"
                            } else {
                                true
                            }
                        });
                    }

                    let tx_name_ident = Ident::new(tx_name, Span::call_site());
                    let new_arm: Arm = if fields.is_empty() {
                        parse2(quote! {
                            TransactionType::#tx_name_ident => Ok(())
                        })?
                    } else {
                        let field_idents = fields
                            .iter()
                            .map(|field| Ident::new(&field.name, Span::call_site()));

                        parse2(quote! {
                            TransactionType::#tx_name_ident { #(#field_idents),* } => Ok(())
                        })?
                    };

                    match_expr.expr = parse2(quote!(tx.tx_type))?;
                    match_expr.arms.insert(0, new_arm);
                }
            }
        }
    }

    Ok(prettyplease::unparse(&ast))
}

fn print_transaction_info(tx_name: &str, fields: &[TransactionField]) {
    println!("✨ Created new transaction type: {}", tx_name);
    println!("Transaction fields:");
    for field in fields {
        println!("  {}: {}", field.name, field.field_type);
    }
    println!("\nUpdate the verify and process methods in src/tx.rs and src/state.rs to add your custom logic!");
}
