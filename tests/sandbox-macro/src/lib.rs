//! Attribute glue for `sandbox`. It never starts external processes at expansion.

use std::sync::LazyLock;

use canton_version::semver::Version;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, Ident, ItemFn, LitStr, Meta, parse_macro_input, parse_quote, spanned::Spanned,
};

mod options;

use options::Options;

/// Version of Canton which was this sandbox built for.
static SANDBOX_CANTON_VERSION: LazyLock<Version> =
    LazyLock::new(|| match Version::parse(env!("SANDBOX_CANTON_VERSION")) {
        Ok(version) => version,
        Err(_) => unreachable!("SANDBOX_CANTON_VERSION must be verified by build script"),
    });

struct ShouldPanic {
    expected: Option<LitStr>,
}

impl ShouldPanic {
    /// Call only for `should_panic` attributes
    fn parse_attr(attribute: &Attribute) -> syn::Result<Self> {
        debug_assert!(attribute.path().is_ident("should_panic"));

        let expected = match &attribute.meta {
            // #[should_panic]
            Meta::Path(_) => None,
            // #[should_panic = "..."]
            Meta::NameValue(value) => match &value.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(message),
                    ..
                }) => Some(message.clone()),
                value => return Err(syn::Error::new_spanned(value, "expected a string literal")),
            },
            // #[should_panic(expected = "...")]
            Meta::List(list) => {
                if !matches!(list.delimiter, syn::MacroDelimiter::Paren(_)) {
                    return Err(syn::Error::new_spanned(list, "expected parentheses"));
                }

                let mut expected = None;
                attribute.parse_nested_meta(|meta| {
                    if !meta.path.is_ident("expected") {
                        return Err(meta.error("expected `expected = \"message\"`"));
                    }
                    if expected.is_some() {
                        return Err(meta.error("duplicate expected argument"));
                    }
                    expected = Some(meta.value()?.parse::<LitStr>()?);
                    Ok(())
                })?;

                Some(expected.ok_or_else(|| {
                    syn::Error::new_spanned(list, "expected `expected = \"message\"`")
                })?)
            }
        };

        Ok(Self { expected })
    }
}

/// Add sandbox acquisition and awaited teardown to a test.
///
/// Put this attribute above `#[tokio::test]`. See the `sandbox` crate README for more info.
#[proc_macro_attribute]
pub fn sandbox(options: TokenStream, input: TokenStream) -> TokenStream {
    let options = parse_macro_input!(options as Options);
    let function = parse_macro_input!(input as ItemFn);
    let root = match proc_macro_crate::crate_name("sandbox") {
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let name = Ident::new(&name, Span::call_site());
            quote!(::#name)
        }
        Ok(proc_macro_crate::FoundCrate::Itself) => quote!(::sandbox),
        Err(error) => {
            return syn::Error::new(Span::call_site(), error.to_string())
                .into_compile_error()
                .into();
        }
    };
    expand(options, function, root)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(
    options: options::Options,
    mut function: ItemFn,
    root: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    check_function(&function)?;

    let mut attrs = Vec::new();
    let mut should_panic = None;
    for attribute in std::mem::take(&mut function.attrs) {
        if attribute.path().is_ident("should_panic") {
            if should_panic.is_some() {
                return Err(syn::Error::new(
                    attribute.span(),
                    "duplicate should_panic attribute",
                ));
            }
            should_panic = Some(ShouldPanic::parse_attr(&attribute)?);
        } else {
            attrs.push(attribute);
        }
    }

    if let Some(canton_version_req) = &options.canton_version_req
        && !canton_version_req.matches(&SANDBOX_CANTON_VERSION)
    {
        let reason = format!("runs on Canton {canton_version_req}");
        // If the test already has #[ignore], it will be reported
        // We want to suppress the warning about the second ignore being unused though
        attrs.push(parse_quote!(#[allow(unused_attributes)]));
        attrs.push(parse_quote!(#[ignore = #reason]));
    }

    let mode = if options.shared {
        let group = options
            .group
            .map_or_else(|| quote!(#root::DEFAULT_GROUP), |group| quote!(#group));
        quote!(#root::__private::Mode::Shared(#group))
    } else {
        quote!(#root::__private::Mode::Fresh)
    };

    let config = match options.config {
        Some(config) => quote!(Some(#config)),
        None => quote!(None),
    };

    let orig_body_ident = Ident::new("__orig_body", Span::mixed_site());
    let name = &function.sig.ident;

    let output = &function.sig.output;
    let orig_body = &function.block;
    // Safe to unwrap after check_function()
    let sb_ref = function.sig.inputs.first().unwrap();

    // This is essentially the original function without attributes
    let orig_body = quote! { async fn #orig_body_ident(#sb_ref) #output #orig_body };

    let block = match &should_panic {
        Some(should_panic) => {
            let expected = match &should_panic.expected {
                Some(message) => quote!(::core::option::Option::Some(#message)),
                None => quote!(::core::option::Option::None),
            };
            parse_quote!({
                #orig_body

                #root::__private::run_should_panic(
                    #config,
                    ::core::option_env!("CARGO_TARGET_TMPDIR"),
                    #mode,
                    concat!(module_path!(), "::", stringify!(#name)),
                    #expected,
                    #orig_body_ident,
                ).await
            })
        }
        None => parse_quote!({
            #orig_body

            #root::__private::run(
                #config,
                ::core::option_env!("CARGO_TARGET_TMPDIR"),
                #mode,
                concat!(module_path!(), "::", stringify!(#name)),
                #orig_body_ident,
            ).await
        }),
    };

    // For wrapper we leave the whole signature untouched, except clearing fn arguments
    function.sig.inputs.clear();

    let wrapped_function = ItemFn {
        attrs,
        block,
        ..function
    };

    Ok(quote!(#wrapped_function))
}

fn check_function(function: &ItemFn) -> syn::Result<()> {
    // Currently only async tests are supported
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            function.sig.fn_token.span,
            "sandbox fixture requires an async function",
        ));
    }
    if !function.sig.generics.params.is_empty()
        || function.sig.generics.where_clause.is_some()
        || !matches!(function.sig.safety, syn::Safety::Default)
        || function.sig.abi.is_some()
        || function.sig.variadic.is_some()
    {
        return Err(syn::Error::new(
            function.sig.span(),
            "sandbox tests cannot be generic, unsafe, extern, or variadic",
        ));
    }
    if function.sig.inputs.len() != 1 {
        return Err(syn::Error::new(
            function.sig.inputs.span(),
            "sandbox test must receive exactly one argument",
        ));
    }
    Ok(())
}
