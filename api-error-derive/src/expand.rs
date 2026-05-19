use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Ident, Variant, spanned::Spanned,
};

use crate::{VariantAttr, parser};

/// The generated code for trait methods.
struct Expansion {
    status_code: TokenStream,
    message: TokenStream,
}

pub fn expand(input: DeriveInput) -> TokenStream {
    let tokens = match input.data {
        Data::Enum(data) => expand_enum(data),
        Data::Struct(data) => expand_struct(&input.ident, data, &input.attrs),
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "ApiError can only be derived for structs and enums",
        )),
    };

    let Expansion {
        status_code,
        message,
    } = match tokens {
        Ok(ts) => ts,
        Err(err) => return err.to_compile_error(),
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ident = &input.ident;

    let api_err_impl = quote! {
        #[automatically_derived]
        impl #impl_generics ApiError for #ident #ty_generics #where_clause {
            fn status_code(&self) -> ::api_error::__http::StatusCode {
                #status_code
            }

            fn message<'a>(&'a self) -> ::std::borrow::Cow<'a, str> {
                #message
            }
        }
    };

    #[cfg(feature = "axum")]
    let axum_impl = Some(quote! {
        #[automatically_derived]
        impl #impl_generics ::api_error::axum::__axum_core::response::IntoResponse for #ident #ty_generics #where_clause {
            fn into_response(self) -> ::api_error::axum::__axum_core::response::Response {
                ::api_error::axum::__ERROR_RESPONDER.get().copied()
                    .unwrap_or(::api_error::axum::default_error_responder)(&self)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::api_error::axum::__axum_core::response::IntoResponse for &#ident #ty_generics #where_clause {
            fn into_response(self) -> ::api_error::axum::__axum_core::response::Response {
                ::api_error::axum::__ERROR_RESPONDER.get().copied()
                    .unwrap_or(::api_error::axum::default_error_responder)(self)
            }
        }
    });

    #[cfg(not(feature = "axum"))]
    let axum_impl: Option<TokenStream> = None;

    quote! {
        #api_err_impl

        #axum_impl
    }
}

fn expand_struct(ident: &Ident, data: DataStruct, attrs: &[Attribute]) -> syn::Result<Expansion> {
    let attr = parser::parse_variant_attrs(attrs)?;

    let status_code = match &attr {
        VariantAttr::Transparent if data.fields.len() == 1 => {
            quote! { ApiError::status_code(&self.0) }
        }
        VariantAttr::Transparent => Err(syn::Error::new_spanned(
            ident,
            "the `#[api_error(transparent)]` attribute is only allowed on structs with only one field",
        ))?,
        VariantAttr::InheritMsg { status_code } | VariantAttr::Custom { status_code, .. } => {
            status_code
                .clone()
                .unwrap_or(quote! { ::api_error::__http::StatusCode::INTERNAL_SERVER_ERROR })
        }
    };

    let message = match (attr, data.fields) {
        (VariantAttr::Transparent, fields) if fields.len() == 1 => {
            quote! { ApiError::message(&self.0) }
        }
        (VariantAttr::Transparent, _) => {
            return Err(syn::Error::new_spanned(
                ident,
                "the `#[api_error(transparent)]` attribute is only allowed on structs with only one field",
            ))?;
        }
        (VariantAttr::InheritMsg { .. }, _) => {
            quote! { ::std::borrow::Cow::Owned(::std::string::ToString::to_string(self)) }
        }
        (VariantAttr::Custom { msg: Some(msg), .. }, Fields::Unit) => {
            quote! { ::std::borrow::Cow::Borrowed(#msg) }
        }
        (VariantAttr::Custom { msg: Some(msg), .. }, Fields::Unnamed(fields)) => {
            let fields: Vec<_> = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| Ident::new(&format!("__field{i}"), f.span()))
                .collect();

            // for unamed fields we want to re-parse the format string to
            // extract positional arguments
            let (fmt_str, pos) = parser::parse_unamed_msg_format(msg, fields.len())?;
            if pos.is_empty() {
                quote! { ::std::borrow::Cow::Borrowed(#fmt_str) }
            } else {
                let fmt_args = pos.into_iter().map(|pos| &fields[pos]);
                quote! {
                    let Self( #(#fields),* ) = self;
                    ::std::borrow::Cow::Owned(::std::format!(#fmt_str, #(#fmt_args),*))
                }
            }
        }
        (VariantAttr::Custom { msg: Some(msg), .. }, Fields::Named(fields)) => {
            let fields = fields.named.iter().map(|f| &f.ident);
            quote! {
                let Self { #(#fields),* } = self;
                ::std::borrow::Cow::Owned(::std::format!(#msg))
            }
        }
        (VariantAttr::Custom { msg: None, .. }, _) => {
            quote! { ::std::borrow::Cow::Borrowed(ApiError::status_code(self).canonical_reason().unwrap_or("Unknown error")) }
        }
    };

    Ok(Expansion {
        status_code,
        message,
    })
}

fn expand_enum(data: DataEnum) -> syn::Result<Expansion> {
    let variant_expansions = data
        .variants
        .into_iter()
        .map(expand_enum_variant)
        .collect::<syn::Result<Vec<_>>>()?;

    let (status_arms, message_arms): (Vec<_>, Vec<_>) = variant_expansions
        .into_iter()
        .map(|v| (v.status_code, v.message))
        .unzip();

    Ok(Expansion {
        status_code: quote! { match self { #(#status_arms),* } },
        message: quote! { match self { #(#message_arms),* } },
    })
}

/// Parse a single enum variant to extract `ApiError` attributes and generate match arms.
fn expand_enum_variant(v: Variant) -> syn::Result<Expansion> {
    let attr_args = parser::parse_variant_attrs(&v.attrs)?;

    let status_arm = expand_status_arm(&v.ident, &v.fields, &attr_args)?;
    let message_arm = expand_message_arm(&v.ident, &v.fields, attr_args)?;

    Ok(Expansion {
        status_code: status_arm,
        message: message_arm,
    })
}

/// Generate a pattern to match a given enum variant.
fn expand_variant_pattern(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote! { Self::#ident },
        Fields::Unnamed(fields) => {
            let idents = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, field)| syn::Ident::new(&format!("__field{i}"), field.span()));

            quote! { Self::#ident(#(#idents),*) }
        }
        Fields::Named(fields) => {
            let idents = fields.named.iter().map(|f| &f.ident);
            quote! { Self::#ident { #(#idents),* } }
        }
    }
}

/// Generate a match statement for the `status_code` method.
fn expand_status_arm(
    variant_ident: &Ident,
    fields: &Fields,
    attrs: &VariantAttr,
) -> syn::Result<TokenStream> {
    let status_pat = expand_variant_pattern(variant_ident, fields);
    let status_code = match (fields, attrs) {
        // forward to inner field with transparent
        (Fields::Unnamed(fields), VariantAttr::Transparent) if fields.unnamed.len() == 1 => {
            quote! { ApiError::status_code(__field0) }
        }
        (_, VariantAttr::Transparent) => Err(syn::Error::new_spanned(
            variant_ident,
            "the `#[api_error(transparent)]` attribute is only allowed on unamed variants with only one field",
        ))?,

        // custom or default status code
        (_, VariantAttr::Custom { status_code, .. } | VariantAttr::InheritMsg { status_code }) => {
            status_code.clone().unwrap_or_else(
                || quote! { ::api_error::__http::StatusCode::INTERNAL_SERVER_ERROR },
            )
        }
    };

    Ok(quote! { #status_pat => #status_code })
}

/// Generate a match statement for the `message` method.
fn expand_message_arm(
    variant_ident: &Ident,
    fields: &Fields,
    attr: VariantAttr,
) -> syn::Result<TokenStream> {
    let message_pat = expand_variant_pattern(variant_ident, fields);

    let message_arm = match (fields, attr) {
        (Fields::Unnamed(fields), VariantAttr::Custom { msg: Some(msg), .. }) => {
            // for unamed fields we want to re-parse the format string to
            // extract positional arguments
            let (fmt_str, pos) = parser::parse_unamed_msg_format(msg, fields.unnamed.len())?;
            let fmt_args = pos
                .into_iter()
                .map(|i| syn::Ident::new(&format!("__field{i}"), fields.unnamed[i].span()));

            if fmt_args.len() > 0 {
                quote! { ::std::borrow::Cow::Owned(::std::format!(#fmt_str, #(#fmt_args),*)) }
            } else {
                quote! { ::std::borrow::Cow::Borrowed(#fmt_str) }
            }
        }
        (Fields::Unit | Fields::Named(_), VariantAttr::Custom { msg: Some(msg), .. }) => {
            quote! { ::std::borrow::Cow::Owned(::std::format!(#msg)) }
        }
        (_, VariantAttr::Custom { msg: None, .. }) => {
            // default message to canonical reason
            quote! { ::std::borrow::Cow::Borrowed(ApiError::status_code(self).canonical_reason().unwrap_or("Unknown error")) }
        }

        // transparent expansion, call the inner field
        (Fields::Unnamed(fields), VariantAttr::Transparent) if fields.unnamed.len() == 1 => {
            quote! { ApiError::message(__field0) }
        }
        (_, VariantAttr::Transparent) => Err(syn::Error::new_spanned(
            variant_ident,
            "the `#[api_error(transparent)]` attribute is only allowed on unamed variants with only one field",
        ))?,

        // inherit expansion, call the display impl
        (_, VariantAttr::InheritMsg { .. }) => {
            quote! { ::std::borrow::Cow::Owned(::std::string::ToString::to_string(self)) }
        }
    };

    Ok(quote! { #message_pat => #message_arm })
}
