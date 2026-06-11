use std::collections::HashMap;

use proc_macro::TokenStream;

use heck::{ToLowerCamelCase, ToSnakeCase};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

use crate::{ensure_named_fields, parse_action_attrs, ActionAttrs};

/// Field metadata for code generation.
struct FieldInfo {
    ident: syn::Ident,
    is_option: bool,
}

/// Look up a field by name, trying both the original camelCase name and its snake_case equivalent.
fn lookup_field<'a>(
    field_map: &'a HashMap<String, FieldInfo>,
    name: &str,
) -> Option<&'a FieldInfo> {
    // Try exact match first
    if let Some(field) = field_map.get(name) {
        return Some(field);
    }
    // Try snake_case version of the name
    let snake_name = name.to_snake_case();
    field_map.get(&snake_name)
}

/// Build a map of field names to their metadata.
fn build_field_map(
    fields: &syn::FieldsNamed,
) -> Result<(HashMap<String, FieldInfo>, bool), syn::Error> {
    let mut field_map = HashMap::new();
    let mut has_nonce = false;

    for field in fields.named.iter() {
        let Some(name) = field.ident.as_ref() else {
            continue;
        };
        let name_str = name.to_string();

        if name_str == "hyperliquid_chain" || name_str == "hyperliquidChain" {
            continue;
        }

        let ty_str = quote! { #field.ty }.to_string();
        let is_option = ty_str.contains("Option");

        if name_str == "nonce" {
            has_nonce = true;
        }

        field_map.insert(
            name_str,
            FieldInfo {
                ident: name.clone(),
                is_option,
            },
        );
    }

    Ok((field_map, has_nonce))
}

/// Convert an EIP-712 type string (e.g., "string", "uint64", "address") to a DynSolType token.
fn eip712_type_to_dyn_sol_type(ty: &str) -> TokenStream2 {
    let ty_lower = ty.to_lowercase();

    if ty_lower == "string" {
        quote! { alloy::dyn_abi::DynSolType::String }
    } else if ty_lower == "address" {
        quote! { alloy::dyn_abi::DynSolType::Address }
    } else if ty_lower == "bool" {
        quote! { alloy::dyn_abi::DynSolType::Bool }
    } else if ty_lower == "bytes" {
        quote! { alloy::dyn_abi::DynSolType::Bytes }
    } else if ty_lower.starts_with("uint") {
        let size: usize = ty_lower
            .strip_prefix("uint")
            .and_then(|s| {
                if s.is_empty() {
                    Some(256)
                } else {
                    s.parse().ok()
                }
            })
            .unwrap_or(256);
        quote! { alloy::dyn_abi::DynSolType::Uint(#size) }
    } else if ty_lower.starts_with("int") {
        let size: usize = ty_lower
            .strip_prefix("int")
            .and_then(|s| {
                if s.is_empty() {
                    Some(256)
                } else {
                    s.parse().ok()
                }
            })
            .unwrap_or(256);
        quote! { alloy::dyn_abi::DynSolType::Int(#size) }
    } else if ty_lower.starts_with("bytes") {
        let size: usize = ty_lower
            .strip_prefix("bytes")
            .and_then(|s| s.parse().ok())
            .unwrap_or(32);
        quote! { alloy::dyn_abi::DynSolType::FixedBytes(#size) }
    } else {
        // Default to string for unknown types
        quote! { alloy::dyn_abi::DynSolType::String }
    }
}

fn enrich_types_preimage(full_types_preimage: &str) -> String {
    let Some(idx) = full_types_preimage.find("hyperliquidChain,") else {
        return full_types_preimage.to_string();
    };
    let insert_at = idx + "hyperliquidChain,".len();
    format!(
        "{}address payloadMultiSigUser,address outerSigner,{}",
        &full_types_preimage[..insert_at],
        &full_types_preimage[insert_at..]
    )
}

fn build_struct_hash_tokens(
    fields: &syn::FieldsNamed,
    full_types_preimage: &str,
    multisig_params: &[(&str, TokenStream2)],
) -> Result<(Vec<TokenStream2>, bool), syn::Error> {
    let (field_map, has_nonce) = build_field_map(fields)?;

    let params = parse_types_params(full_types_preimage)?;
    let mut tokens = Vec::with_capacity(params.len() + 1);
    tokens.push(quote! {
        alloy::dyn_abi::DynSolValue::FixedBytes(type_hash, 32)
    });

    for (ty, name) in params {
        let dyn_sol_type = eip712_type_to_dyn_sol_type(&ty);

        // Special case: hyperliquidChain comes from the signing chain, not a field
        if name == "hyperliquidChain" {
            tokens.push(quote! {
                chain.get_hyperliquid_chain().to_abi_value(&#dyn_sol_type)?
            });
            if !multisig_params.is_empty() {
                for (_, expr) in multisig_params {
                    tokens.push(expr.clone());
                }
            }
            continue;
        }

        if name == "payloadMultiSigUser" || name == "outerSigner" {
            continue;
        }

        // Special case: nonce/time field - must unwrap Option<u64>
        if name == "nonce" || name == "time" {
            let field = field_map.get("nonce").ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "nonce field missing")
            })?;
            let ident = &field.ident;
            let expr = if field.is_option {
                quote! {
                    {
                        let nonce = self.#ident.ok_or(crate::Error::GenericParse("nonce must be set before signing".to_string()))?;
                        nonce.to_abi_value(&#dyn_sol_type)?
                    }
                }
            } else {
                quote! {
                    self.#ident.to_abi_value(&#dyn_sol_type)?
                }
            };
            tokens.push(expr);
            continue;
        }

        // Regular field - use ToAbiValue trait
        let field = lookup_field(&field_map, &name).ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "field not found: {name} (tried snake_case: {})",
                    name.to_snake_case()
                ),
            )
        })?;

        let ident = &field.ident;
        tokens.push(quote! {
            self.#ident.to_abi_value(&#dyn_sol_type)?
        });
    }

    Ok((tokens, has_nonce))
}

fn build_user_signed_action_impl(
    ident: &syn::Ident,
    action_type_lit: &syn::LitStr,
    types_lit: &syn::LitStr,
    multisig_types_lit: &syn::LitStr,
    struct_hash_tokens: &[TokenStream2],
    multisig_struct_hash_tokens: &[TokenStream2],
    uses_time: bool,
) -> TokenStream2 {
    quote! {
        impl crate::actions::UserSignedAction for #ident {
            const ACTION_TYPE: &'static str = #action_type_lit;

            fn struct_hash(&self, chain: &crate::SigningChain) -> Result<alloy::primitives::B256, crate::Error> {
                use crate::ToAbiValue;
                let type_hash = alloy::primitives::keccak256(#types_lit);
                let values = vec![
                    #(#struct_hash_tokens,)*
                ];
                let tuple = alloy::dyn_abi::DynSolValue::Tuple(values);
                Ok(alloy::primitives::keccak256(tuple.abi_encode()))
            }

            fn multisig_struct_hash(
                &self,
                chain: &crate::SigningChain,
                payload_multi_sig_user: alloy::primitives::Address,
                outer_signer: alloy::primitives::Address,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                use crate::ToAbiValue;
                let type_hash = alloy::primitives::keccak256(#multisig_types_lit);
                let values = vec![
                    #(#multisig_struct_hash_tokens,)*
                ];
                let tuple = alloy::dyn_abi::DynSolValue::Tuple(values);
                Ok(alloy::primitives::keccak256(tuple.abi_encode()))
            }
        }

        impl crate::actions::Action for #ident {
            const ACTION_TYPE: &'static str = <Self as crate::actions::UserSignedAction>::ACTION_TYPE;
            const PAYLOAD_KEY: &'static str = <Self as crate::actions::UserSignedAction>::ACTION_TYPE;

            fn is_user_signed() -> bool {
                true
            }

            fn uses_time() -> bool {
                #uses_time
            }

            fn signing_hash(
                &self,
                meta: &crate::actions::SigningMeta,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                <Self as crate::actions::UserSignedAction>::eip712_signing_hash(
                    self,
                    meta.signing_chain,
                )
            }

            fn nonce(&self) -> Option<u64> {
                self.nonce
            }

            fn extract_action_kind(&self) -> crate::actions::ActionKind {
                crate::actions::ActionKind::#ident(self.clone())
            }

            fn with_nonce(mut self, nonce: u64) -> Self {
                self.nonce = Some(nonce);
                self
            }
        }

    }
}

pub(crate) fn derive_user_signed_action(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ActionAttrs {
        action_type_override,
        types_preimage,
        ..
    } = match parse_action_attrs(&input.attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.to_compile_error().into(),
    };

    let Some(types_preimage) = types_preimage else {
        return syn::Error::new(
            input.ident.span(),
            "UserSignedAction requires #[action(types = \"...\")]",
        )
        .to_compile_error()
        .into();
    };
    let full_types_preimage = format!("HyperliquidTransaction:{types_preimage}");

    let fields = match ensure_named_fields(&input) {
        Ok(fields) => fields,
        Err(err) => return err.to_compile_error().into(),
    };

    let ident = &input.ident;
    let action_type_value =
        action_type_override.unwrap_or_else(|| ident.to_string().to_lower_camel_case());
    let action_type_lit = LitStr::new(&action_type_value, ident.span());
    let types_lit = LitStr::new(&full_types_preimage, ident.span());

    let parsed_params = match parse_types_params(&full_types_preimage) {
        Ok(result) => result,
        Err(err) => return err.to_compile_error().into(),
    };
    let uses_time = parsed_params.iter().any(|(_, name)| name == "time");

    let multisig_types_preimage = enrich_types_preimage(&full_types_preimage);
    let multisig_types_lit = LitStr::new(&multisig_types_preimage, ident.span());
    let multisig_param_exprs = [
        (
            "payloadMultiSigUser",
            quote! {
                payload_multi_sig_user.to_abi_value(&alloy::dyn_abi::DynSolType::Address)?
            },
        ),
        (
            "outerSigner",
            quote! {
                outer_signer.to_abi_value(&alloy::dyn_abi::DynSolType::Address)?
            },
        ),
    ];

    let (struct_hash_tokens, has_nonce) =
        match build_struct_hash_tokens(fields, &full_types_preimage, &[]) {
            Ok(result) => result,
            Err(err) => return err.to_compile_error().into(),
        };

    let (multisig_struct_hash_tokens, _) =
        match build_struct_hash_tokens(fields, &multisig_types_preimage, &multisig_param_exprs) {
            Ok(result) => result,
            Err(err) => return err.to_compile_error().into(),
        };

    if !has_nonce {
        return syn::Error::new(
            input.ident.span(),
            "UserSignedAction derive requires a `nonce` field",
        )
        .to_compile_error()
        .into();
    }

    build_user_signed_action_impl(
        ident,
        &action_type_lit,
        &types_lit,
        &multisig_types_lit,
        &struct_hash_tokens,
        &multisig_struct_hash_tokens,
        uses_time,
    )
    .into()
}

fn parse_types_params(full_types_preimage: &str) -> Result<Vec<(String, String)>, syn::Error> {
    let component = alloy_dyn_abi::eip712::parser::ComponentType::parse(full_types_preimage)
        .map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("failed to parse types: {e}"),
            )
        })?;

    let mut parsed_params: Vec<(String, String)> = Vec::new();
    for prop in component.props {
        parsed_params.push((prop.ty.stem().span().to_string(), prop.name.to_string()));
    }

    Ok(parsed_params)
}
