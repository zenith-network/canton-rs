use canton_paths::Paths;
use syn::{Attribute, Error, Path, Type};

use crate::choice_attributes::ChoiceSpec;

pub struct TemplateAttributes {
    paths: Paths,
    key: Option<Type>,
    choices: Vec<ChoiceSpec>,
    implements: Vec<Type>,
}

impl TemplateAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut key = None;
        let mut choices = Vec::new();
        let mut implements = Vec::new();

        let attr = attributes
            .iter()
            .find(|attr| attr.path().is_ident("template"));

        if let Some(attr) = attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    crate_path = Some(meta.value()?.parse::<Path>()?);
                    return Ok(());
                }

                if meta.path.is_ident("key") {
                    key = Some(meta.value()?.parse::<Type>()?);
                    return Ok(());
                }

                if meta.path.is_ident("choice") {
                    choices.push(ChoiceSpec::parse(meta)?);
                    return Ok(());
                }

                if meta.path.is_ident("implements") {
                    implements.push(meta.value()?.parse::<Type>()?);
                    return Ok(());
                }

                Err(meta.error("unexpected identifier"))
            })?;
        }

        let paths = crate_path.map(Paths::from_root).unwrap_or_default();

        Ok(Self {
            paths,
            key,
            choices,
            implements,
        })
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn key(&self) -> Option<&Type> {
        self.key.as_ref()
    }

    pub fn choices(&self) -> &[ChoiceSpec] {
        &self.choices
    }

    pub fn implements(&self) -> &[Type] {
        &self.implements
    }
}
