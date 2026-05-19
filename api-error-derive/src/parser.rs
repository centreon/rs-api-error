// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0

//! Parser for `#[api_error(...)]` attributes on enum variants and structs.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, LitStr, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

use crate::{VariantAttr, kw};

/// Content of a message variant attribute parameter
enum MessageValue {
    Literal(syn::LitStr),
    Inherit,
}

/// Custom `#[api_error(...)]` attribute parameters:
///
/// `#[api_error(message = "Custom error message", status_code = 400)]`
#[derive(Default)]
struct CustomAttrFields {
    message: Option<MessageValue>,
    status_code: Option<TokenStream>,
}

/// A variant with optional fields that will be merged into a final [`VariantAttr`].
struct PartialVariantAttr {
    span: Span,
    kind: VariantAttrKind,
}

/// Content of a message variant attribute parameter
enum VariantAttrKind {
    Transparent,
    Custom(CustomAttrFields),
}
impl Default for PartialVariantAttr {
    fn default() -> Self {
        PartialVariantAttr {
            span: Span::call_site(),
            kind: VariantAttrKind::Custom(CustomAttrFields::default()),
        }
    }
}

impl PartialVariantAttr {
    /// Convert a partial attribute into a complete one, returning an error
    /// if required fields are missing.
    fn into_complete(self) -> VariantAttr {
        match self.kind {
            VariantAttrKind::Transparent => VariantAttr::Transparent,
            VariantAttrKind::Custom(CustomAttrFields {
                message,
                status_code,
            }) => match message {
                Some(MessageValue::Literal(msg)) => VariantAttr::Custom {
                    msg: Some(msg),
                    status_code,
                },
                Some(MessageValue::Inherit) => VariantAttr::InheritMsg { status_code },
                None => VariantAttr::Custom {
                    msg: None,
                    status_code,
                },
            },
        }
    }

    /// Merge another partial attribute into this one, returning an error
    /// if there are conflicting attributes.
    fn merge(&mut self, other: PartialVariantAttr) -> syn::Result<()> {
        match (&mut self.kind, other.kind) {
            (VariantAttrKind::Transparent, _) | (_, VariantAttrKind::Transparent) => {
                Err(syn::Error::new(
                    other.span,
                    "`transparent` cannot be combined with other `api_error` attributes",
                ))
            }
            (VariantAttrKind::Custom(a), VariantAttrKind::Custom(b)) => {
                if b.message.is_some() {
                    if a.message.is_some() {
                        return Err(syn::Error::new(
                            other.span,
                            "duplicate `message` attribute in `api_error`",
                        ));
                    }
                    a.message = b.message;
                }

                if b.status_code.is_some() {
                    if a.status_code.is_some() {
                        return Err(syn::Error::new(
                            other.span,
                            "duplicate `status_code` attribute in `api_error`",
                        ));
                    }
                    a.status_code = b.status_code;
                }

                Ok(())
            }
        }
    }
}

/// Parse `#[api_error(...)]` variant attributes and merge them into a single [`VariantAttr`].
pub fn parse_variant_attrs(attrs: &[Attribute]) -> syn::Result<VariantAttr> {
    let mut attrs = attrs
        .iter()
        .filter_map(|a| match a.meta {
            syn::Meta::List(ref meta_list) => Some((a, meta_list)),
            _ => None,
        })
        .filter(|(_, a)| a.path.is_ident("api_error"))
        .map(|(a, meta_list)| {
            let mut partial = meta_list.parse_args::<PartialVariantAttr>()?;
            partial.span = a.span();
            Ok::<_, syn::Error>(partial)
        });

    let mut first = attrs.next().transpose()?.unwrap_or_default();

    for attr in attrs {
        first.merge(attr?)?;
    }

    Ok(first.into_complete())
}

impl Parse for PartialVariantAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        if input.peek(kw::transparent) {
            input.parse::<kw::transparent>()?;

            // Check if there are more tokens after transparent
            if !input.is_empty() {
                let lookahead = input.lookahead1();
                if lookahead.peek(Token![,]) {
                    // There's a comma, which means more attributes follow
                    return Err(syn::Error::new(
                        span,
                        "`transparent` cannot be combined with other `api_error` attributes",
                    ));
                }
            }

            Ok(Self {
                span,
                kind: VariantAttrKind::Transparent,
            })
        } else {
            Ok(Self {
                span,
                kind: VariantAttrKind::Custom(CustomAttrFields::parse(input)?),
            })
        }
    }
}

enum CustomFields {
    Message(#[allow(unused)] kw::message),
    StatusCode(#[allow(unused)] kw::status_code),
}

impl Parse for CustomFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::message) {
            Ok(CustomFields::Message(input.parse()?))
        } else if lookahead.peek(kw::status_code) {
            Ok(CustomFields::StatusCode(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for CustomAttrFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut message: Option<MessageValue> = None;
        let mut status_code: Option<TokenStream> = None;

        while !input.is_empty() {
            // Check if someone is trying to use transparent with other attributes
            if input.peek(kw::transparent) {
                return Err(syn::Error::new(
                    input.span(),
                    "`transparent` cannot be combined with other `api_error` attributes",
                ));
            }

            match input.parse::<CustomFields>()? {
                CustomFields::Message(_) => {
                    if input.peek(Token![=]) {
                        input.parse::<Token![=]>()?;
                        message = Some(MessageValue::Literal(input.parse()?));
                    } else {
                        let content;
                        syn::parenthesized!(content in input);

                        let lookahead = content.lookahead1();
                        if lookahead.peek(kw::inherit) {
                            content.parse::<kw::inherit>()?;
                            message = Some(MessageValue::Inherit);
                        } else {
                            return Err(syn::Error::new(
                                lookahead.error().span(),
                                r#"Only `message = "..."` or `message(inherit)` syntax is allowed in `api_error` attribute"#,
                            ));
                        }
                    }
                }
                CustomFields::StatusCode(_) => {
                    input.parse::<Token![=]>()?;
                    if input.peek(syn::LitInt) {
                        let lit_int: syn::LitInt = input.parse()?;
                        status_code = Some(quote! {
                            const {
                                match ::api_error::__http::StatusCode::from_u16(#lit_int) {
                                    Ok(code) => code,
                                    Err(_) => panic!("Invalid status code literal"),
                                }
                            }
                        });
                    } else {
                        let expr: syn::Expr = input.parse()?;
                        status_code = Some(quote! { #expr });
                    }
                }
            }

            let lookahead = input.lookahead1();
            if lookahead.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else if !input.is_empty() {
                return Err(syn::Error::new(
                    lookahead.error().span(),
                    "Expected comma separated key-value pairs in api_error attribute",
                ));
            }
        }

        Ok(CustomAttrFields {
            message,
            status_code,
        })
    }
}

/// Takes a message format literal with positional arguments and replaces
/// all positional arguments with unamed ones, verifying that the referenced
/// field indices are in-bounds.
///
/// Example:
/// `"Error occurred on field {1} with value {0}"` becomes
/// `("Error occurred on field {} with value {}", [1, 0])`
pub fn parse_unamed_msg_format(
    message: LitStr,
    field_count: usize,
) -> syn::Result<(LitStr, Vec<usize>)> {
    let value = message.value();
    if !value.contains('{') {
        // No formatting needed
        return Ok((message, Vec::new()));
    }

    let span = message.span();

    let mut idents = Vec::new();
    let mut iter = value.bytes().enumerate().peekable();
    let mut output = String::with_capacity(value.len());

    let mut escaped = false;

    while let Some((i, c)) = iter.next() {
        if c == b'{' && !escaped {
            if iter.peek().map(|(_, c)| c) == Some(&b'{') {
                escaped = true;
                output.push(c as char);
                continue;
            }

            // consumes the '{...}' expression
            let start = i + 1;
            loop {
                let (i, c) = iter
                    .next()
                    .ok_or(syn::Error::new(span, "unmatched '{' in format string"))?;

                if c == b'}' {
                    let pos = &value[start..i];
                    let pos = pos.parse::<usize>().map_err(|_| {
                        syn::Error::new(span, format!("invalid format argument '{pos}', formatting arguments must be numeric positional indices"))
                    })?;

                    if pos >= field_count {
                        return Err(syn::Error::new(
                            span,
                            format!(
                                "positional argument '{pos}' out of bounds (max {})",
                                field_count - 1
                            ),
                        ));
                    }

                    idents.push(pos);

                    output.push_str("{}");
                    break;
                }
            }
        } else {
            output.push(c as char);
        }

        escaped = false;
    }

    Ok((LitStr::new(&output, message.span()), idents))
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;

    use super::*;
    #[test]
    fn test_parse_unamed_msg_format() {
        let msg = LitStr::new(
            "Error on field {1} with value {0}",
            proc_macro2::Span::call_site(),
        );
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 2).unwrap();
        assert_eq!(parsed_msg.value(), "Error on field {} with value {}");
        assert_eq!(idents.len(), 2);
        assert_eq!(idents[0], 1);
        assert_eq!(idents[1], 0);
    }

    #[test]
    fn test_single_argument() {
        let msg = LitStr::new("Value is {0}", Span::call_site());
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 1).unwrap();

        assert_eq!(parsed_msg.value(), "Value is {}");
        assert_eq!(idents.len(), 1);
        assert_eq!(idents[0], 0);
    }

    #[test]
    fn test_multiple_same_argument() {
        let msg = LitStr::new("Repeat {0}, again {0}", Span::call_site());
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 1).unwrap();

        assert_eq!(parsed_msg.value(), "Repeat {}, again {}");
        assert_eq!(idents.len(), 2);
        assert_eq!(idents[0], 0);
        assert_eq!(idents[1], 0);
    }

    #[test]
    fn test_arguments_out_of_order() {
        let msg = LitStr::new("{2} then {0} then {1}", Span::call_site());
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 3).unwrap();

        assert_eq!(parsed_msg.value(), "{} then {} then {}");
        assert_eq!(idents.len(), 3);
        assert_eq!(idents[0], 2);
        assert_eq!(idents[1], 0);
        assert_eq!(idents[2], 1);
    }

    #[test]
    fn test_no_arguments() {
        let msg = LitStr::new("No formatting here", Span::call_site());
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 0).unwrap();

        assert_eq!(parsed_msg.value(), "No formatting here");
        assert!(idents.is_empty());
    }

    #[test]
    fn test_escaped_open_brace_is_preserved() {
        let msg = LitStr::new("Literal {{ brace", Span::call_site());
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 0).unwrap();

        // Current implementation treats '{{' as literal '{'
        assert_eq!(parsed_msg.value(), "Literal {{ brace");
        assert!(idents.is_empty());
    }

    #[test]
    fn test_unmatched_open_brace_error() {
        let msg = LitStr::new("Invalid {0", Span::call_site());
        let err = parse_unamed_msg_format(msg, 1).unwrap_err();

        assert!(
            err.to_string().contains("unmatched '{'"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_non_numeric_argument_error() {
        let msg = LitStr::new("Invalid {abc}", Span::call_site());
        let err = parse_unamed_msg_format(msg, 1).unwrap_err();

        assert!(
            err.to_string()
                .contains("invalid format argument 'abc', formatting arguments must be numeric positional indices"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_empty_argument_error() {
        let msg = LitStr::new("Invalid {}", Span::call_site());
        let err = parse_unamed_msg_format(msg, 1).unwrap_err();

        assert!(
            err.to_string()
                .contains("invalid format argument '', formatting arguments must be numeric positional indices"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_argument_out_of_bounds_error() {
        let msg = LitStr::new("Out of bounds {1}", Span::call_site());
        let err = parse_unamed_msg_format(msg, 1).unwrap_err();

        assert!(
            err.to_string().contains("out of bounds"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_large_index_error() {
        let msg = LitStr::new("Huge index {999}", Span::call_site());
        let err = parse_unamed_msg_format(msg, 2).unwrap_err();

        assert!(
            err.to_string().contains("out of bounds"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_adjacent_arguments() {
        let msg = LitStr::new("{0}{1}{0}", Span::call_site());
        let (parsed_msg, idents) = parse_unamed_msg_format(msg, 2).unwrap();

        assert_eq!(parsed_msg.value(), "{}{}{}");
        assert_eq!(idents.len(), 3);
        assert_eq!(idents[0], 0);
        assert_eq!(idents[1], 1);
        assert_eq!(idents[2], 0);
    }
}
