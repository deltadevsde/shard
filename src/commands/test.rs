use anyhow::{Context, Result};
use quote::{format_ident, quote};
use std::fs;
use std::path::Path;
// We need ExprMatch for matching on match expressions
use syn::{
    parse_file, parse_quote, Expr, ExprMatch, Fields, File, Item, ItemEnum, ItemImpl, Pat, PatIdent,
};

/// Creates a new transaction type in both tx.rs and state.rs files
pub fn create_transaction(
    project_path: &str,
    tx_name: &str,
    fields: Vec<(String, String)>,
) -> Result<()> {
    let path = Path::new(project_path);
    if !path.exists() {
        anyhow::bail!("Project directory not found");
    }

    // Read and parse the source files into AST (Abstract Syntax Tree)
    // This converts the Rust source code into a data structure we can manipulate
    let tx_path = path.join("src").join("tx.rs");
    let tx_content = fs::read_to_string(&tx_path)?;
    let mut tx_ast = parse_file(&tx_content)?;

    let state_path = path.join("src").join("state.rs");
    let state_content = fs::read_to_string(&state_path)?;
    let mut state_ast = parse_file(&state_content)?;

    // Modify the ASTs by adding the new transaction
    add_transaction_variant(&mut tx_ast, tx_name, &fields)?;
    add_transaction_handling(&mut state_ast, tx_name, &fields)?;

    // Convert the modified ASTs back to formatted Rust code and write to files
    fs::write(&tx_path, prettyplease::unparse(&tx_ast))?;
    fs::write(&state_path, prettyplease::unparse(&state_ast))?;

    println!("✨ Added new transaction type: {}", tx_name);
    Ok(())
}

/// Adds a new variant to the Transaction enum and its verify implementation
fn add_transaction_variant(
    ast: &mut File,
    tx_name: &str,
    fields: &[(String, String)],
) -> Result<()> {
    // Search through the AST to find the Transaction enum definition
    let transaction_enum = ast
        .items
        .iter_mut()
        .find_map(|item| {
            if let Item::Enum(item_enum) = item {
                if item_enum.ident == "Transaction" {
                    Some(item_enum)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .context("Could not find Transaction enum")?;

    // Create a new identifier for our transaction variant
    let variant_ident = format_ident!("{}", tx_name);

    // Create the enum variant - either a simple variant or one with fields
    let variant = if fields.is_empty() {
        // Simple variant like `MyTx`
        parse_quote! {
            #variant_ident
        }
    } else {
        // Variant with fields like `MyTx { field1: Type1, field2: Type2 }`
        let fields = fields.iter().map(|(name, ty)| {
            let field_ident = format_ident!("{}", name);
            let ty_ident = format_ident!("{}", ty);
            parse_quote! {
                #field_ident: #ty_ident
            }
        });
        parse_quote! {
            #variant_ident { #(#fields),* }
        }
    };

    // Add the variant if it doesn't already exist
    if !transaction_enum
        .variants
        .iter()
        .any(|v| v.ident == variant_ident)
    {
        transaction_enum.variants.push(variant);
    }

    // Find the Transaction implementation block
    let verify_impl = ast
        .items
        .iter_mut()
        .find_map(|item| {
            if let Item::Impl(impl_item) = item {
                if impl_item.trait_.is_none() {
                    Some(impl_item)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .context("Could not find Transaction impl")?;

    // Update the verify method with a new match arm
    for method in &mut verify_impl.items {
        if let syn::ImplItem::Fn(method) = method {
            if method.sig.ident == "verify" {
                // Fixed: Use if let Expr::Match(match_expr) instead of Stmt::Match
                if let Some(Expr::Match(match_expr)) = method
                    .block
                    .stmts
                    .first_mut()
                    .and_then(|stmt| {
                        if let syn::Stmt::Expr(expr, _) = stmt {
                            Some(expr)
                        } else {
                            None
                        }
                    })
                    .and_then(|expr| {
                        if let Expr::Match(match_expr) = expr {
                            Some(match_expr)
                        } else {
                            None
                        }
                    })
                {
                    let new_arm = if fields.is_empty() {
                        parse_quote! {
                            Self::#variant_ident => Ok(())
                        }
                    } else {
                        let field_pats = fields.iter().map(|(name, _)| {
                            let field_ident = format_ident!("{}", name);
                            parse_quote!(#field_ident)
                        });
                        parse_quote! {
                            Self::#variant_ident { #(#field_pats),* } => {
                                // TODO: Add verification logic
                                Ok(())
                            }
                        }
                    };

                    // Add new match arm if not already present
                    if !match_expr.arms.iter().any(|arm| {
                        if let Pat::Ident(PatIdent { ident, .. }) = arm.pat {
                            ident == variant_ident
                        } else {
                            false
                        }
                    }) {
                        match_expr.arms.insert(0, new_arm);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Adds handling for the new transaction type in the State implementation
fn add_transaction_handling(
    ast: &mut File,
    tx_name: &str,
    fields: &[(String, String)],
) -> Result<()> {
    // Find the State implementation block
    let state_impl = ast
        .items
        .iter_mut()
        .find_map(|item| {
            if let Item::Impl(impl_item) = item {
                if impl_item.trait_.is_none() {
                    Some(impl_item)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .context("Could not find State impl")?;

    let variant_ident = format_ident!("{}", tx_name);

    // Update both validate_tx and process_tx methods
    for method in &mut state_impl.items {
        if let syn::ImplItem::Fn(method) = method {
            if method.sig.ident == "validate_tx" || method.sig.ident == "process_tx" {
                // Fixed: Similar fix for match expression access
                if let Some(Expr::Match(match_expr)) = method
                    .block
                    .stmts
                    .get_mut(1)
                    .and_then(|stmt| {
                        if let syn::Stmt::Expr(expr, _) = stmt {
                            Some(expr)
                        } else {
                            None
                        }
                    })
                    .and_then(|expr| {
                        if let Expr::Match(match_expr) = expr {
                            Some(match_expr)
                        } else {
                            None
                        }
                    })
                {
                    let new_arm = if fields.is_empty() {
                        parse_quote! {
                            Transaction::#variant_ident => Ok(())
                        }
                    } else {
                        let field_pats = fields.iter().map(|(name, _)| {
                            let field_ident = format_ident!("{}", name);
                            parse_quote!(#field_ident)
                        });
                        parse_quote! {
                            Transaction::#variant_ident { #(#field_pats),* } => {
                                // TODO: Add handling logic
                                Ok(())
                            }
                        }
                    };

                    // Add new match arm if not already present
                    if !match_expr.arms.iter().any(|arm| {
                        if let Pat::Ident(PatIdent { ident, .. }) = arm.pat {
                            ident == variant_ident
                        } else {
                            false
                        }
                    }) {
                        match_expr.arms.insert(0, new_arm);
                    }
                }
            }
        }
    }

    Ok(())
}
