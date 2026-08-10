use canton_paths::Paths;
use syn::{Attribute, Error, Path, Type};

pub struct TemplateAttributes {
    paths: Paths,
    key: Option<Type>,
}

impl TemplateAttributes {
    pub fn parse(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut crate_path = None;
        let mut key = None;

        let attr = attributes
            .iter()
            .find(|attr| attr.path().is_ident("template"));

        if let Some(attr) = attr {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_path") {
                    crate_path = Some(meta.value()?.parse::<Path>()?);
                }

                if meta.path.is_ident("key") {
                    key = Some(meta.value()?.parse::<Type>()?);
                }

                Ok(())
            })?;
        }

        let paths = crate_path.map(Paths::from_root).unwrap_or_default();

        Ok(Self { paths, key })
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn key(&self) -> Option<&Type> {
        self.key.as_ref()
    }
}
