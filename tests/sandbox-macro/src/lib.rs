//! Attribute glue for `sandbox`. It never starts external processes at expansion.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, FnArg, Ident, ItemFn, LitStr, Meta, Path, ReturnType, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
};

struct Options {
    config: Option<Path>,
    shared: bool,
    group: Option<LitStr>,
}

impl Parse for Options {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = None;
        let mut mode = None;
        let mut group = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "config" => {
                    if config.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate config provider"));
                    }
                    input.parse::<Token![=]>()?;
                    config = Some(input.parse()?);
                }
                "group" => {
                    if group.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate group"));
                    }
                    input.parse::<Token![=]>()?;
                    let name: LitStr = input.parse()?;
                    if name.value().is_empty() {
                        return Err(syn::Error::new(
                            name.span(),
                            "shared group must not be empty",
                        ));
                    }
                    group = Some(name);
                }
                "shared" | "fresh" => {
                    if mode.is_some() {
                        return Err(syn::Error::new(
                            key.span(),
                            "choose exactly one sandbox mode",
                        ));
                    }
                    mode = Some(key == "shared");
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "expected shared, fresh, group, or config",
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        let shared = mode.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "choose a sandbox mode: shared or fresh")
        })?;
        if !shared && let Some(group) = &group {
            return Err(syn::Error::new(group.span(), "group requires shared mode"));
        }
        Ok(Self {
            config,
            shared,
            group,
        })
    }
}

/// Add sandbox acquisition and awaited teardown to an asynchronous test.
/// Put this attribute above `#[tokio::test]`; see the `sandbox` crate README.
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
    options: Options,
    mut function: ItemFn,
    root: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            function.sig.fn_token.span,
            "sandbox fixture requires an async function; place it above #[tokio::test]",
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
            "sandbox test must receive exactly one &Sandbox argument",
        ));
    }
    let argument = function.sig.inputs.first().unwrap().clone();
    match &argument {
        FnArg::Typed(argument) if matches!(&*argument.ty, Type::Reference(reference) if reference.mutability.is_none()) =>
            {}
        _ => {
            return Err(syn::Error::new(
                argument.span(),
                "sandbox argument must be an immutable &Sandbox reference",
            ));
        }
    }
    let mut expected = None;
    let mut attributes = Vec::new();
    for attribute in std::mem::take(&mut function.attrs) {
        if attribute.path().is_ident("should_panic") {
            if expected.is_some() {
                return Err(syn::Error::new(
                    attribute.span(),
                    "duplicate should_panic attribute",
                ));
            }
            expected = Some(parse_expected(&attribute)?);
        } else {
            attributes.push(attribute);
        }
    }
    if expected.is_some() && !is_unit(&function.sig.output) {
        return Err(syn::Error::new(
            function.sig.output.span(),
            "should_panic sandbox tests must return ()",
        ));
    }
    let expected = match expected {
        None => quote!(#root::__private::ExpectedPanic::No),
        Some(None) => quote!(#root::__private::ExpectedPanic::Yes(None)),
        Some(Some(message)) => quote!(#root::__private::ExpectedPanic::Yes(Some(#message))),
    };
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
    let name = &function.sig.ident;
    let helper = Ident::new("__sandbox_body", Span::mixed_site());
    let output = &function.sig.output;
    let body = &function.block;
    let wrapper = quote!({
        async fn #helper(#argument) #output #body
        #root::__private::run(
            #config, ::core::option_env!("CARGO_TARGET_TMPDIR"), #mode,
            concat!(module_path!(), "::", stringify!(#name)),
            #expected, #helper,
        ).await
    });
    function.attrs = attributes;
    function.sig.inputs.clear();
    *function.block = syn::parse2(wrapper)?;
    Ok(quote!(#function))
}

fn is_unit(output: &ReturnType) -> bool {
    matches!(output, ReturnType::Default)
        || matches!(output, ReturnType::Type(_, ty) if matches!(&**ty, Type::Tuple(tuple) if tuple.elems.is_empty()))
}

fn parse_expected(attribute: &Attribute) -> syn::Result<Option<LitStr>> {
    if matches!(attribute.meta, Meta::Path(_)) {
        return Ok(None);
    }
    let args = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    if args.len() == 1
        && let Some(Meta::NameValue(value)) = args.first()
        && value.path.is_ident("expected")
        && let syn::Expr::Lit(literal) = &value.value
        && let syn::Lit::Str(message) = &literal.lit
    {
        return Ok(Some(message.clone()));
    }
    Err(syn::Error::new(
        attribute.span(),
        "expected #[should_panic] or #[should_panic(expected = \"message\")]",
    ))
}
