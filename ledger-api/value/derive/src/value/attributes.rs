use canton_paths::Paths;
use canton_types::Name;
use syn::{Attribute, Error, Expr, ExprLit, Lit, Path, spanned::Spanned as _};

use crate::{Attr, collect_err_chain};

/// Struct/enum level attributes
#[derive(Clone)]
pub struct ItemAttributes {
    paths: Paths,
}

impl ItemAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;

        let attr = attributes.iter().find(|attr| attr.path().is_ident("value"));

        if let Some(attr) = attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    let buf = meta.value()?;
                    let path = buf.parse::<Path>()?;
                    crate_path = Some(path);
                    return Ok(());
                }

                Err(meta.error("unrecognized attribute meta name"))
            })?;
        }

        let paths = crate_path.map(Paths::from_root).unwrap_or_default();

        Ok(Self { paths })
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
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
        let attr = attributes.iter().find(|attr| attr.path().is_ident("name"));

        if let Some(attr) = attr {
            let meta = attr.meta.require_name_value()?;
            name = Some(Self::parse_name(meta.value.clone())?);
        }
        Ok(Self { name })
    }

    pub fn name(&self) -> Option<&Attr<Name>> {
        self.name.as_ref()
    }
}
