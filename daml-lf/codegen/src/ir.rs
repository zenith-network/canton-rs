//! Intermediate representation on generation

#![allow(dead_code, reason = "Keeping some methods for future use if needed")]

use canton_types::PackageId;
use daml_lf::package::VersionedSealedPackage;

use crate::graph::DiGraph;

/// A single definition of an item
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Definition<'a> {
    #[cfg(feature = "v2")]
    /// Definition of a Daml LF v2 item
    V2(crate::v2::ir::Definition<'a>),
}

impl<'a> Definition<'a> {
    pub fn package(&self) -> VersionedSealedPackage<'a> {
        match self {
            #[cfg(feature = "v2")]
            Definition::V2(node) => node.package().into(),
        }
    }

    pub fn package_id(&self) -> &'a PackageId {
        match self {
            #[cfg(feature = "v2")]
            Definition::V2(node) => node.package_id(),
        }
    }

    pub fn package_name(&self) -> &'a str {
        match self {
            #[cfg(feature = "v2")]
            Definition::V2(node) => node.package().metadata().name(),
        }
    }

    pub fn package_version(&self) -> &'a str {
        match self {
            #[cfg(feature = "v2")]
            Definition::V2(node) => node.package().metadata().version(),
        }
    }
}

/// Generation graph (graph of definitions)
pub type GenGraph<'a> = DiGraph<Definition<'a>>;

// Re-export
pub use crate::graph::Edge;

#[derive(Clone, Debug)]
pub struct GeneratedPackage {
    ident: syn::Ident,
    header: Vec<syn::Item>,
    modules: Vec<GeneratedModule>,
}

impl GeneratedPackage {
    pub fn new(ident: syn::Ident, header: Vec<syn::Item>) -> Self {
        Self::new_with_modules(ident, header, Vec::new())
    }

    pub fn new_with_modules(
        ident: syn::Ident,
        header: Vec<syn::Item>,
        modules: Vec<GeneratedModule>,
    ) -> Self {
        Self {
            ident,
            header,
            modules,
        }
    }

    pub fn push_header(&mut self, item: syn::Item) {
        self.header.push(item);
    }

    pub fn extend_header(&mut self, items: impl IntoIterator<Item = syn::Item>) {
        self.header.extend(items);
    }

    pub fn modules(&self) -> &[GeneratedModule] {
        &self.modules
    }

    pub fn modules_mut(&mut self) -> &mut Vec<GeneratedModule> {
        &mut self.modules
    }

    pub fn push_module(&mut self, module: GeneratedModule) {
        self.modules.push(module);
    }

    pub fn extend_modules(&mut self, modules: impl IntoIterator<Item = GeneratedModule>) {
        self.modules.extend(modules);
    }

    pub fn render_into_module(self) -> syn::ItemMod {
        let Self {
            ident,
            header: mut content,
            modules,
        } = self;

        content.extend(
            modules
                .into_iter()
                .map(GeneratedModule::render)
                .map(syn::Item::Mod),
        );

        render_module(ident, Vec::new(), content)
    }

    pub fn render(self) -> syn::File {
        syn::File {
            shebang: None,
            frontmatter: None,
            attrs: Vec::new(),
            items: vec![syn::Item::Mod(self.render_into_module())],
        }
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedModule {
    ident: syn::Ident,
    header: Vec<syn::Item>,
    items: Vec<syn::Item>,
    submodules: Vec<GeneratedModule>,
}

impl GeneratedModule {
    pub fn new(ident: syn::Ident) -> Self {
        Self {
            ident,
            header: Vec::new(),
            items: Vec::new(),
            submodules: Vec::new(),
        }
    }

    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn header(&self) -> &[syn::Item] {
        &self.header
    }

    pub fn set_header(&mut self, header: Vec<syn::Item>) {
        self.header = header;
    }

    pub fn push_item(&mut self, item: syn::Item) {
        self.items.push(item);
    }

    pub fn extend_items(&mut self, items: impl IntoIterator<Item = syn::Item>) {
        self.items.extend(items);
    }

    pub fn submodules(&self) -> &[GeneratedModule] {
        &self.submodules
    }

    pub fn submodules_mut(&mut self) -> &mut Vec<GeneratedModule> {
        &mut self.submodules
    }

    pub fn push_submodule(&mut self, submodule: GeneratedModule) {
        self.submodules.push(submodule);
    }

    pub fn extend_submodules(&mut self, submodules: impl IntoIterator<Item = GeneratedModule>) {
        self.submodules.extend(submodules);
    }

    pub fn render(self) -> syn::ItemMod {
        let Self {
            ident,
            header: mut content,
            items,
            submodules,
        } = self;

        content.extend(items);
        content.extend(submodules.into_iter().map(Self::render).map(syn::Item::Mod));

        render_module(ident, Vec::new(), content)
    }
}

/// Render a public module with given content
fn render_module(
    ident: syn::Ident,
    attrs: Vec<syn::Attribute>,
    items: Vec<syn::Item>,
) -> syn::ItemMod {
    syn::ItemMod {
        attrs,
        vis: syn::Visibility::Public(syn::token::Pub::default()),
        unsafety: None,
        mod_token: syn::token::Mod::default(),
        ident,
        content: Some((syn::token::Brace::default(), items)),
        semi: None,
    }
}
