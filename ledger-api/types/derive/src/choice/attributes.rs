use canton_paths::Paths;
use canton_types::Name;
use proc_macro2::Span;
use syn::{Attribute, Error, Expr, ExprLit, Lit, Path, Type, spanned::Spanned as _};

use crate::collect_err_chain;

#[derive(Clone)]
pub enum NameAttr {
    Fixed(Name),
    Expr(Expr),
}

#[derive(Clone)]
pub struct ChoiceAttributes {
    paths: Paths,
    template: Type,
    result: Type,
    consuming: Expr,
    name: Option<NameAttr>,
}

impl ChoiceAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut template = None;
        let mut result = None;
        let mut consuming = None;
        let mut name = None;
        let mut attr_span = None;

        let attr = attributes
            .iter()
            .find(|attr| attr.path().is_ident("choice"));

        if let Some(attr) = attr {
            attr_span = Some(attr.meta.span());
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    crate_path = Some(meta.value()?.parse::<Path>()?);
                    return Ok(());
                }

                if meta.path.is_ident("template") {
                    template = Some(meta.value()?.parse::<Type>()?);
                    return Ok(());
                }

                if meta.path.is_ident("result") {
                    result = Some(meta.value()?.parse::<Type>()?);
                    return Ok(());
                }

                if meta.path.is_ident("consuming") {
                    consuming = Some(meta.value()?.parse::<Expr>()?);
                    return Ok(());
                }

                if meta.path.is_ident("name") {
                    let expr = meta.value()?.parse::<Expr>()?;
                    name = Some(Self::parse_name_attr(expr)?);
                    return Ok(());
                }

                Err(meta.error("unrecognized attribute meta name"))
            })?;
        }

        let err_span = attr_span.unwrap_or_else(Span::call_site);

        let template = template.ok_or_else(|| Error::new(err_span, "template is not specified"))?;
        let result = result.ok_or_else(|| Error::new(err_span, "result is not specified"))?;
        let consuming =
            consuming.ok_or_else(|| Error::new(err_span, "consuming is not specified"))?;
        let paths = crate_path.map(Paths::from_root).unwrap_or_default();

        Ok(Self {
            paths,
            template,
            result,
            consuming,
            name,
        })
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn template(&self) -> &Type {
        &self.template
    }

    pub fn result(&self) -> &Type {
        &self.result
    }

    pub fn consuming(&self) -> &Expr {
        &self.consuming
    }

    pub fn name(&self) -> Option<&NameAttr> {
        self.name.as_ref()
    }

    fn parse_name_attr(expr: Expr) -> Result<NameAttr, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let name = Name::new(lit.value())
                .map_err(|err| Error::new(expr.span(), collect_err_chain(&err).join(": ")))?;
            Ok(NameAttr::Fixed(name))
        } else {
            Ok(NameAttr::Expr(expr))
        }
    }
}
