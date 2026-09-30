use canton_paths::Paths;
use proc_macro2::Span;
use syn::{Attribute, Error, Path, Type};

use crate::choice_attributes::ChoiceSpec;

pub struct InterfaceAttributes {
    paths: Paths,
    view: Type,
    choices: Vec<ChoiceSpec>,
    requires: Vec<Type>,
}

impl InterfaceAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut view = None;
        let mut choices = Vec::new();
        let mut requires = Vec::new();

        let attr = attributes
            .iter()
            .find(|attr| attr.path().is_ident("interface"));

        if let Some(attr) = attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    crate_path = Some(meta.value()?.parse::<Path>()?);
                }

                if meta.path.is_ident("view") {
                    view = Some(meta.value()?.parse::<Type>()?);
                }

                if meta.path.is_ident("choice") {
                    choices.push(ChoiceSpec::parse(meta)?);
                    return Ok(());
                }

                if meta.path.is_ident("requires") {
                    requires.push(meta.value()?.parse::<Type>()?);
                }

                Ok(())
            })?;
        }

        let paths = crate_path.map(Paths::from_root).unwrap_or_default();
        let view =
            view.ok_or_else(|| Error::new(Span::call_site(), "view type is not specified"))?;

        Ok(Self {
            paths,
            view,
            choices,
            requires,
        })
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn view(&self) -> &Type {
        &self.view
    }

    pub fn choices(&self) -> &[ChoiceSpec] {
        &self.choices
    }

    pub fn requires(&self) -> &[Type] {
        &self.requires
    }
}
