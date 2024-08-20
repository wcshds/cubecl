use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{punctuated::Punctuated, FieldValue, Lit, Member, PathArguments, Token};

use crate::{analyzer::KEYWORDS, codegen_function::expr::codegen_expr, tracker::VariableTracker};

use super::base::{Codegen, CodegenKind};

/// Codegen for literals
pub(crate) fn codegen_lit(lit: &syn::ExprLit) -> TokenStream {
    match lit.lit {
        // We treat floats differently to avoid getting 4..into() for instance
        Lit::Float(_) => {
            let lit_str = lit.lit.to_token_stream().to_string();
            let float_lit = lit_str.parse::<f32>().unwrap();
            quote::quote! { #float_lit }
        }
        _ => {
            quote::quote! { #lit }
        }
    }
}

/// Codegen for arrays of literals
pub(crate) fn codegen_array_lit(array: &syn::ExprArray) -> TokenStream {
    let mut tokens = quote::quote! {};
    for element in array.elems.iter() {
        let token = match element {
            syn::Expr::Lit(lit) => Codegen::new(codegen_lit(lit), CodegenKind::Literal),
            _ => {
                return syn::Error::new_spanned(array, "Only arrays of literals are supported")
                    .into_compile_error()
            }
        };
        tokens.extend(quote::quote! { #token, });
    }
    quote::quote! { [ #tokens ] }
}

/// Codegen for a local declaration (let ...)
/// Supports:
/// let x = ...
/// let x: T = ...
/// let _ = ...
/// let (a, b) = ...
/// let mut _ = ...
pub(crate) fn codegen_local(
    local: &syn::Local,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> TokenStream {
    let let_tok = local.let_token;

    let ident = match &local.pat {
        syn::Pat::Ident(ident) => ident.to_token_stream(),
        syn::Pat::Type(pat_type) => match &*pat_type.pat {
            syn::Pat::Ident(pat_ident) => pat_ident.to_token_stream(),
            _ => todo!("Codegen: Unsupported typed path {:?}", pat_type.pat),
        },
        syn::Pat::Wild(wild) => wild.underscore_token.to_token_stream(),
        syn::Pat::Tuple(_) => {
            // destructuring pattern; we can just return it as is
            return quote::quote! {
                #local
            };
        }
        _ => todo!("Codegen: Declaration {:?} is unsupported.", local.pat),
    };

    variable_tracker.codegen_declare(ident.to_string(), loop_level as u8);

    match local.init.as_ref() {
        Some(init) => {
            let (init, kind, _) = codegen_expr(&init.expr, loop_level, variable_tracker).process();

            if matches!(kind, CodegenKind::Comptime) {
                variable_tracker
                    .set_as_comptime(ident.to_string(), loop_level as u8, None)
                    .unwrap();
            }

            if matches!(kind, CodegenKind::Comptime) {
                quote::quote! {
                    #let_tok #ident = #init;
                }
            } else {
                quote::quote! {
                    #let_tok #ident = {
                        let _inner = #init;
                        cubecl::frontend::Init::init(_inner, context)
                    };
                }
            }
        }
        None => {
            quote::quote! {
                #let_tok #ident;
            }
        }
    }
}

/// Codegen for indexed access
pub(crate) fn codegen_index(
    index: &syn::ExprIndex,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> Codegen {
    let array = codegen_expr(&index.expr, loop_level, variable_tracker);
    let index = codegen_expr(&index.index, loop_level, variable_tracker);

    let tokens = quote::quote! {
        {
            let _array = #array;
            let _index = #index;
            cubecl::frontend::index::expand(context, _array, _index)
        }
    };

    let mut codegen = Codegen::new(tokens, CodegenKind::Expand);
    codegen.set_array_indexing(Some(super::base::ArrayIndexing {
        array: array.tokens(),
        index: index.tokens(),
    }));

    codegen
}

/// Codegen for assignation
/// Supports:
/// - scalar
/// - indexed array
pub(crate) fn codegen_assign(
    assign: &syn::ExprAssign,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> TokenStream {
    match assign.left.as_ref() {
        syn::Expr::Index(index) => {
            let array = codegen_expr(&index.expr, loop_level, variable_tracker);
            let index = codegen_expr(&index.index, loop_level, variable_tracker);
            let value = codegen_expr(&assign.right, loop_level, variable_tracker);

            quote::quote! {
                {
                    let _array = #array;
                    let _index = #index;
                    let _value = #value;
                    cubecl::frontend::index_assign::expand(context, _array, _index, _value)
                }
            }
        }
        syn::Expr::Unary(_) | syn::Expr::Field(_) | syn::Expr::Path(_) => {
            let lhs = codegen_expr(&assign.left, loop_level, variable_tracker);
            let rhs = codegen_expr(&assign.right, loop_level, variable_tracker);

            quote::quote! {
                {
                    let _assign_lhs = #lhs;
                    let _assign_rhs = #rhs;
                    cubecl::frontend::assign::expand(context, _assign_rhs, _assign_lhs)
                }
            }
        }
        _ => todo!("Assign of expr {:?} unsupported", assign.left),
    }
}

pub(crate) fn codegen_path_var(
    path: &syn::ExprPath,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> Codegen {
    let ident = match path.path.get_ident() {
        Some(ident) => ident,
        None => {
            return Codegen::new(
                quote::quote! {
                    #path
                },
                CodegenKind::Expand,
            );
        }
    };

    let name = ident.to_string();

    if name == "None" {
        return Codegen::new(quote::quote! { None }, CodegenKind::Comptime);
    }

    if KEYWORDS.contains(&name.as_str()) {
        Codegen::new(
            quote::quote! {
                #ident :: expand(context)
            },
            CodegenKind::Expand,
        )
    } else {
        let (will_be_used_again, is_comptime) = variable_tracker
            .codegen_reuse(name, loop_level as u8, None)
            .unwrap_or((true, false));

        let kind = if is_comptime {
            CodegenKind::Comptime
        } else {
            CodegenKind::Expand
        };

        let output = if will_be_used_again {
            quote::quote! {
                #ident.clone()
            }
        } else {
            quote::quote! {
                #ident
            }
        };

        Codegen::new(output, kind)
    }
}

/// Codegen for a field used in rhs of a statement
/// This function adds cloning when necessary
pub(crate) fn codegen_field(
    field: &syn::ExprField,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> TokenStream {
    let (struct_, field) = if let Member::Named(attribute_ident) = &field.member {
        if let syn::Expr::Path(struct_expr) = &*field.base {
            let struct_ident = struct_expr
                .path
                .get_ident()
                .expect("Codegen: field access only supported on ident struct.");

            (struct_ident, attribute_ident)
        } else {
            todo!("Codegen: field access only supported on ident struct.");
        }
    } else {
        todo!("Codegen: unnamed attribute not supported.");
    };

    let (will_be_used_again, _) = variable_tracker
        .codegen_reuse(
            struct_.to_string(),
            loop_level as u8,
            Some(field.to_string()),
        )
        .unwrap();

    if will_be_used_again {
        quote::quote! {
            #struct_ . #field .clone()
        }
    } else {
        quote::quote! {
            #struct_ . #field
        }
    }
}

// Codegen for a struct declaration
pub(crate) fn codegen_struct(
    struct_: &syn::ExprStruct,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> TokenStream {
    let mut deconstructed_path = Vec::new();
    for segment in struct_.path.segments.iter() {
        let generics = if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
            Some(arguments)
        } else {
            None
        };
        deconstructed_path.push((&segment.ident, generics));
    }

    let (struct_name, generics) = deconstructed_path
        .pop()
        .expect("At least one ident in the path");

    // This is hacky but using <struct_ as CubeType>::ExpandType {...} is experimental in Rust
    let expanded_struct_name = syn::Ident::new(
        format!("{}Expand", struct_name).as_str(),
        proc_macro2::Span::call_site(),
    );

    deconstructed_path.push((&expanded_struct_name, generics));

    // Reconstruct the path
    let mut path_tokens = quote::quote! {};
    for (ident, angle_bracketed_generics) in deconstructed_path {
        let ident_tokens = ident.to_token_stream();
        let generics_tokens = angle_bracketed_generics.to_token_stream();

        path_tokens.extend(quote::quote! {
            #ident_tokens #generics_tokens
        });
    }

    let fields = codegen_field_creation(&struct_.fields, loop_level, variable_tracker);
    quote::quote! {
        #path_tokens { #fields }
    }
}

fn codegen_field_creation(
    fields: &Punctuated<FieldValue, Token![,]>,
    loop_level: usize,
    variable_tracker: &mut VariableTracker,
) -> TokenStream {
    let mut field_tokens = quote::quote! {};
    for field in fields.iter() {
        let field_name_token = &field.member;
        let field_value_token = codegen_expr(&field.expr, loop_level, variable_tracker);
        field_tokens.extend(quote::quote! { #field_name_token : #field_value_token,  });
    }
    field_tokens
}
