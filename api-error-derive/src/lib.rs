// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0
#![warn(clippy::pedantic)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::needless_pass_by_value)]

use proc_macro2::TokenStream;
use syn::LitStr;

#[proc_macro_derive(ApiError, attributes(api_error))]
pub fn derive_api_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    crate::expand::expand(input).into()
}

mod expand;
mod parser;

mod kw {
    syn::custom_keyword!(inherit);
    syn::custom_keyword!(transparent);
    syn::custom_keyword!(status_code);
    syn::custom_keyword!(message);
}

/// All possible parameters for the `api_error` attribute.
enum VariantAttr {
    /// Do we want to delegate to the inner field?
    Transparent,

    /// Do we want to inherit the message from the Display impl?
    InheritMsg { status_code: Option<TokenStream> },

    /// A custom message and status code.
    Custom {
        /// A format message, defaulting to the status code HTTP reason.
        msg: Option<LitStr>,

        /// The HTTP status code, defaulting to an `INTERNAL_SERVER_ERROR`.
        status_code: Option<TokenStream>,
    },
}
