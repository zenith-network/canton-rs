use canton_paths::Paths;
use canton_types::{DottedName, PackageId, PackageName};
use proc_macro2::Span;
use syn::{Attribute, Error, Expr, ExprLit, Lit, Path, spanned::Spanned as _};

use crate::{Attr, collect_err_chain};

/// Struct/enum level attributes
pub struct IdentifierAttributes {
    package_id: Attr<PackageId>,
    package_name: Attr<PackageName>,
    module_name: Attr<DottedName>,
    paths: Paths,
    name: Option<Attr<DottedName>>,
}

impl IdentifierAttributes {
    pub fn package_id(&self) -> &Attr<PackageId> {
        &self.package_id
    }

    pub fn package_name(&self) -> &Attr<PackageName> {
        &self.package_name
    }

    pub fn module_name(&self) -> &Attr<DottedName> {
        &self.module_name
    }

    pub fn name(&self) -> Option<&Attr<DottedName>> {
        self.name.as_ref()
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn parse(attrs: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut module_name_attr = None;
        let mut name = None;
        let mut package_id_attr = None;
        let mut package_name_attr = None;

        let attr = attrs.iter().find(|attr| attr.path().is_ident("identifier"));

        if let Some(attr) = attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    let buf = meta.value()?;
                    let path = buf.parse::<Path>()?;
                    crate_path = Some(path);
                    return Ok(());
                }

                if meta.path.is_ident("name") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    name = Some(Self::parse_name(expr)?);
                    return Ok(());
                }

                if meta.path.is_ident("package_id") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    package_id_attr = Some(Self::parse_package_id(expr)?);
                    return Ok(());
                }

                if meta.path.is_ident("package_name") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    package_name_attr = Some(Self::parse_package_name(expr)?);
                    return Ok(());
                }

                if meta.path.is_ident("module") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    module_name_attr = Some(Self::parse_module(expr)?);
                    return Ok(());
                }

                Err(meta.error("unrecognized attribute meta name"))
            })?;
        }

        let paths = if let Some(path) = crate_path {
            Paths::from_root(path)
        } else {
            Paths::default()
        };

        let package_id = package_id_attr.ok_or_else(|| {
            Error::new(
                attr.map(|attr| attr.meta.span())
                    .unwrap_or_else(|| Span::call_site()),
                "package_id is not specified",
            )
        })?;

        let package_name = package_name_attr.ok_or_else(|| {
            Error::new(
                attr.map(|attr| attr.meta.span())
                    .unwrap_or_else(|| Span::call_site()),
                "package_name is not specified",
            )
        })?;

        let module_name = module_name_attr.ok_or_else(|| {
            Error::new(
                attr.map(|attr| attr.meta.span())
                    .unwrap_or_else(|| Span::call_site()),
                "module is not specified",
            )
        })?;

        Ok(Self {
            package_id,
            package_name,
            module_name,
            paths,
            name,
        })
    }

    fn parse_name(expr: Expr) -> Result<Attr<DottedName>, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let span = expr.span();
            let entity_name = DottedName::parse(lit.value())
                .map_err(|err| Error::new(span, collect_err_chain(&err).join(": ")))?;
            Ok(Attr::fixed(entity_name, span))
        } else {
            Ok(Attr::expr(expr))
        }
    }

    fn parse_package_id(expr: Expr) -> Result<Attr<PackageId>, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let span = expr.span();
            let package_id = PackageId::new(lit.value())
                .map_err(|err| Error::new(span, collect_err_chain(&err).join(": ")))?;
            Ok(Attr::fixed(package_id, span))
        } else {
            Ok(Attr::expr(expr))
        }
    }

    fn parse_package_name(expr: Expr) -> Result<Attr<PackageName>, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let span = expr.span();
            let package_name = PackageName::new(lit.value())
                .map_err(|err| Error::new(span, collect_err_chain(&err).join(": ")))?;
            Ok(Attr::fixed(package_name, span))
        } else {
            Ok(Attr::expr(expr))
        }
    }

    fn parse_module(expr: Expr) -> Result<Attr<DottedName>, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let span = expr.span();
            let module_name = DottedName::parse(lit.value()).map_err(|err| {
                Error::new(
                    span,
                    format!("bad module name: {}", collect_err_chain(&err).join(": ")),
                )
            })?;
            Ok(Attr::fixed(module_name, span))
        } else {
            Ok(Attr::expr(expr))
        }
    }
}
