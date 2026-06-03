use proc_macro::TokenStream;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields, LitStr};

use crate::{ensure_struct_fields, parse_action_attrs, ActionAttrs};

fn has_nonce_field(fields: &syn::FieldsNamed) -> bool {
    fields
        .named
        .iter()
        .any(|field| field.ident.as_ref().is_some_and(|ident| ident == "nonce"))
}

fn build_l1_action_impl(
    ident: &syn::Ident,
    action_type_lit: &syn::LitStr,
    payload_key_lit: &syn::LitStr,
) -> TokenStream2 {
    quote! {
        impl crate::actions::L1Action for #ident {
            const ACTION_TYPE: &'static str = #action_type_lit;
            const PAYLOAD_KEY: &'static str = #payload_key_lit;
        }

        impl crate::actions::Action for #ident {
            const ACTION_TYPE: &'static str = <Self as crate::actions::L1Action>::ACTION_TYPE;
            const PAYLOAD_KEY: &'static str = <Self as crate::actions::L1Action>::PAYLOAD_KEY;

            fn signing_hash(
                &self,
                meta: &crate::actions::SigningMeta,
            ) -> Result<alloy::primitives::B256, crate::Error> {
                let vault_for_hash =
                    if <Self as crate::actions::L1Action>::EXCLUDE_VAULT_FROM_HASH {
                        None
                    } else {
                        meta.vault_address
                    };

                let wrapper = crate::actions::L1ActionWrapper { action: self };
                let connection_id = crate::actions::compute_l1_hash(
                    &wrapper,
                    meta.nonce,
                    vault_for_hash,
                    meta.expires_after,
                )?;

                Ok(crate::actions::agent_signing_hash(
                    connection_id,
                    &meta.signing_chain.get_source(),
                ))
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

pub(crate) fn derive_l1_action(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ActionAttrs {
        action_type_override,
        payload_key_override,
        ..
    } = match parse_action_attrs(&input.attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.to_compile_error().into(),
    };

    let data_fields = match ensure_struct_fields(&input) {
        Ok(fields) => fields,
        Err(err) => return err.to_compile_error().into(),
    };

    let has_nonce = match data_fields {
        Fields::Named(fields) => has_nonce_field(fields),
        _ => false,
    };

    if !has_nonce {
        return syn::Error::new(
            input.ident.span(),
            "L1Action derive requires a `nonce: Option<u64>` field",
        )
        .to_compile_error()
        .into();
    }

    let ident = &input.ident;
    let action_type_value =
        action_type_override.unwrap_or_else(|| ident.to_string().to_lower_camel_case());
    let action_type_lit = LitStr::new(&action_type_value, ident.span());

    let payload_key_value = payload_key_override.unwrap_or_else(|| action_type_value.clone());
    let payload_key_lit = LitStr::new(&payload_key_value, ident.span());

    build_l1_action_impl(ident, &action_type_lit, &payload_key_lit).into()
}
