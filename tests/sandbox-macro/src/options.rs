use canton_version::semver::VersionReq;
use proc_macro2::Span;
use syn::{
    Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
};

pub struct Options {
    pub config: Option<Path>,
    pub shared: bool,
    pub group: Option<LitStr>,
    pub canton_version_req: Option<VersionReq>,
}

impl Parse for Options {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut config = None;
        let mut mode = None;
        let mut group = None;
        let mut canton_version_req = None;
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
                "canton_version" => {
                    if canton_version_req.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate canton_version"));
                    }
                    input.parse::<Token![=]>()?;
                    let version_str: LitStr = input.parse()?;
                    let version = VersionReq::parse(&version_str.value())
                        .map_err(|err| syn::Error::new(version_str.span(), err))?;
                    canton_version_req = Some(version);
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
            canton_version_req,
        })
    }
}
