use canton_paths::Paths;
use canton_types::Name;
use syn::{
    Attribute, Error, Expr, ExprLit, Lit, LitStr, Path, Token, WherePredicate,
    punctuated::Punctuated, spanned::Spanned as _,
};

use crate::{Attr, collect_err_chain};

/// Struct/enum level attributes
#[derive(Clone)]
pub struct ItemAttributes {
    paths: Paths,
    bounds: Vec<WherePredicate>,
    into_bounds: Vec<WherePredicate>,
    from_bounds: Vec<WherePredicate>,
}

impl ItemAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut bounds = Vec::new();
        let mut into_bounds = Vec::new();
        let mut from_bounds = Vec::new();

        for attr in attributes
            .iter()
            .filter(|attr| attr.path().is_ident("value"))
        {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    let buf = meta.value()?;
                    let path = buf.parse::<Path>()?;
                    crate_path = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("bound") {
                    bounds.extend(Self::parse_bounds(meta.value()?.parse()?)?);
                    return Ok(());
                }
                if meta.path.is_ident("into_bound") {
                    into_bounds.extend(Self::parse_bounds(meta.value()?.parse()?)?);
                    return Ok(());
                }
                if meta.path.is_ident("from_bound") {
                    from_bounds.extend(Self::parse_bounds(meta.value()?.parse()?)?);
                    return Ok(());
                }

                Err(meta.error("unrecognized attribute meta name"))
            })?;
        }

        let paths = crate_path.map(Paths::from_root).unwrap_or_default();

        Ok(Self {
            paths,
            bounds,
            into_bounds,
            from_bounds,
        })
    }

    fn parse_bounds(lit: LitStr) -> Result<Punctuated<WherePredicate, Token![,]>, Error> {
        lit.parse_with(Punctuated::<WherePredicate, Token![,]>::parse_terminated)
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn bounds(&self) -> &[WherePredicate] {
        &self.bounds
    }

    #[allow(
        clippy::wrong_self_convention,
        reason = "Meaning of 'into_' here is different"
    )]
    pub fn into_bounds(&self) -> &[WherePredicate] {
        &self.into_bounds
    }

    #[allow(
        clippy::wrong_self_convention,
        reason = "Meaning of 'from_' here is different"
    )]
    pub fn from_bounds(&self) -> &[WherePredicate] {
        &self.from_bounds
    }
}

/// Field/variant level attributes
///
/// # Example
///
/// ```rust,ignore
/// #[name = "myField"]
/// ```
pub struct MemberAttributes {
    name: Option<Attr<Name>>,
    omit_bound: bool,
}

impl MemberAttributes {
    fn parse_name(expr: Expr) -> Result<Attr<Name>, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let span = expr.span();
            let name = Name::new(lit.value())
                .map_err(|err| Error::new(span, collect_err_chain(&err).join(": ")))?;
            Ok(Attr::fixed(name, span))
        } else {
            Ok(Attr::expr(expr))
        }
    }

    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut name = None;
        let mut omit_bound = false;
        let name_attr = attributes.iter().find(|attr| attr.path().is_ident("name"));

        if let Some(attr) = name_attr {
            let meta = attr.meta.require_name_value()?;
            name = Some(Self::parse_name(meta.value.clone())?);
        }

        let value_attr = attributes.iter().find(|attr| attr.path().is_ident("value"));
        if let Some(attr) = value_attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("omit_bound") {
                    if omit_bound {
                        return Err(meta.error("duplicate `omit_bound` attribute"));
                    }
                    omit_bound = true;
                    return Ok(());
                }

                Err(meta.error("unrecognized attribute meta name"))
            })?;
        }

        Ok(Self { name, omit_bound })
    }

    pub fn name(&self) -> Option<&Attr<Name>> {
        self.name.as_ref()
    }

    pub fn omit_bound(&self) -> bool {
        self.omit_bound
    }
}
