use canton_types::{DottedName, Name, PackageId, PackageName};
use proc_macro2::Span;
use syn::{Attribute, Error, Expr, ExprLit, Lit, Path, spanned::Spanned as _};

use crate::{collect_err_chain, paths::Paths};

pub const VALUE_ATTR_NAME: &str = "value";

#[derive(Clone)]
pub enum FieldNameAttr {
    Fixed(Name),
    Expr(Expr),
}

#[derive(Clone)]
pub enum EntityNameAttr {
    Fixed(DottedName),
    Expr(Expr),
}

#[derive(Clone)]
pub enum PackageIdAttr {
    Fixed(PackageId),
    Expr(Expr),
}

#[derive(Clone)]
pub enum PackageNameAttr {
    Fixed(PackageName),
    Expr(Expr),
}

#[derive(Clone)]
pub enum ModuleNameAttr {
    Fixed(DottedName),
    Expr(Expr),
}

/// Struct/enum level attributes
#[derive(Clone)]
pub struct ItemAttributes {
    package_id: PackageIdAttr,
    package_name: PackageNameAttr,
    module_name: ModuleNameAttr,
    paths: Paths,
    name: Option<EntityNameAttr>,
    template: bool,
}

impl ItemAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut module_name_attr = None;
        let mut name = None;
        let mut package_id_attr = None;
        let mut package_name_attr = None;
        let mut template = false;

        let attr = attributes
            .iter()
            .find(|attr| attr.path().is_ident(VALUE_ATTR_NAME));

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
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &expr
                    {
                        let entity_name = DottedName::parse(lit.value()).map_err(|err| {
                            Error::new(
                                expr.span(),
                                format!("bad entity name: {}", collect_err_chain(&err).join(": ")),
                            )
                        })?;
                        name = Some(EntityNameAttr::Fixed(entity_name));
                        return Ok(());
                    } else {
                        name = Some(EntityNameAttr::Expr(expr));
                        return Ok(());
                    }
                }

                if meta.path.is_ident("package_id") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &expr
                    {
                        let package_id = PackageId::new(lit.value()).map_err(|err| {
                            Error::new(
                                expr.span(),
                                format!("bad package ID: {}", collect_err_chain(&err).join(": ")),
                            )
                        })?;
                        package_id_attr = Some(PackageIdAttr::Fixed(package_id));
                        return Ok(());
                    } else {
                        package_id_attr = Some(PackageIdAttr::Expr(expr));
                        return Ok(());
                    }
                }

                if meta.path.is_ident("package_name") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &expr
                    {
                        let package_name = PackageName::new(lit.value()).map_err(|err| {
                            Error::new(
                                expr.span(),
                                format!("bad package name: {}", collect_err_chain(&err).join(": ")),
                            )
                        })?;
                        package_name_attr = Some(PackageNameAttr::Fixed(package_name));
                        return Ok(());
                    } else {
                        package_name_attr = Some(PackageNameAttr::Expr(expr));
                        return Ok(());
                    }
                }

                if meta.path.is_ident("module_name") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;
                    module_name_attr = if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &expr
                    {
                        let module_name = DottedName::parse(lit.value()).map_err(|err| {
                            Error::new(
                                expr.span(),
                                format!("bad module name: {}", collect_err_chain(&err).join(": ")),
                            )
                        })?;
                        Some(ModuleNameAttr::Fixed(module_name))
                    } else {
                        Some(ModuleNameAttr::Expr(expr))
                    };
                    return Ok(());
                }

                if meta.path.is_ident("template") {
                    template = true;
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
                "module_name is not specified",
            )
        })?;

        Ok(Self {
            package_id,
            package_name,
            module_name,
            paths,
            name,
            template,
        })
    }

    pub fn package_id(&self) -> &PackageIdAttr {
        &self.package_id
    }

    pub fn package_name(&self) -> &PackageNameAttr {
        &self.package_name
    }

    pub fn module_name(&self) -> &ModuleNameAttr {
        &self.module_name
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn name(&self) -> Option<&EntityNameAttr> {
        self.name.as_ref()
    }

    pub fn template(&self) -> bool {
        self.template
    }
}

/// Field/variant level attributes
///
/// # Example
///
/// ```rust,ignore
/// #[value(name = "myField")]
/// ```
pub struct MemberAttributes {
    name: Option<FieldNameAttr>,
}

impl MemberAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut name_attr = None;
        let attr = attributes
            .iter()
            .find(|attr| attr.path().is_ident(VALUE_ATTR_NAME));
        if let Some(attr) = attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let buf = meta.value()?;
                    let expr = buf.parse::<Expr>()?;

                    name_attr = if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &expr
                    {
                        let name = Name::new(lit.value()).map_err(|err| {
                            Error::new(
                                expr.span(),
                                format!("bad field name: {}", collect_err_chain(&err).join(": ")),
                            )
                        })?;
                        Some(FieldNameAttr::Fixed(name))
                    } else {
                        Some(FieldNameAttr::Expr(expr))
                    };

                    return Ok(());
                }

                Err(meta.error("unrecognized attribute"))
            })?;
        }
        Ok(Self { name: name_attr })
    }

    pub fn name(&self) -> Option<&FieldNameAttr> {
        self.name.as_ref()
    }
}
