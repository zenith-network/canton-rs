use canton_paths::Paths;
use canton_types::Name;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Expr, ExprLit, Generics, Ident, Lit, Type, meta::ParseNestedMeta, spanned::Spanned as _,
};

use crate::collect_err_chain;

pub enum NameAttr {
    Fixed(Name),
    Expr(Expr),
}

impl NameAttr {
    pub fn parse(expr: Expr) -> Result<Self, Error> {
        if let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &expr
        {
            let name = Name::new(lit.value())
                .map_err(|error| Error::new(expr.span(), collect_err_chain(&error).join(": ")))?;

            Ok(Self::Fixed(name))
        } else {
            Ok(Self::Expr(expr))
        }
    }

    pub fn expand(&self, paths: &Paths) -> TokenStream {
        match self {
            Self::Fixed(name) => {
                let types = paths.types();
                let name = name.as_str();
                quote! { #types::Name::new_static_unchecked(#name) }
            }
            Self::Expr(expr) => quote! { #expr },
        }
    }
}

pub struct ChoiceSpec {
    argument: Type,
    result: Type,
    consuming: Expr,
    name: NameAttr,
}

impl ChoiceSpec {
    pub fn parse(meta: ParseNestedMeta<'_>) -> Result<Self, Error> {
        let mut argument = None;
        let mut result = None;
        let mut consuming = None;
        let mut name = None;

        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("argument") {
                argument = Some(nested.value()?.parse::<Type>()?);
                return Ok(());
            }

            if nested.path.is_ident("result") {
                result = Some(nested.value()?.parse::<Type>()?);
                return Ok(());
            }

            if nested.path.is_ident("consuming") {
                consuming = Some(nested.value()?.parse::<Expr>()?);
                return Ok(());
            }

            if nested.path.is_ident("name") {
                let expr = nested.value()?.parse::<Expr>()?;
                name = Some(NameAttr::parse(expr)?);
                return Ok(());
            }

            Err(nested.error("unexpected identifier"))
        })?;

        let argument = argument.ok_or_else(|| meta.error("argument type is not specified"))?;
        let result = result.ok_or_else(|| meta.error("result type is not specified"))?;
        let consuming = consuming.ok_or_else(|| meta.error("consuming is not specified"))?;
        let name = name.ok_or_else(|| meta.error("name is not specified"))?;

        Ok(Self {
            argument,
            result,
            consuming,
            name,
        })
    }

    pub fn argument(&self) -> &Type {
        &self.argument
    }

    pub fn result(&self) -> &Type {
        &self.result
    }

    pub fn consuming(&self) -> &Expr {
        &self.consuming
    }

    pub fn name(&self) -> &NameAttr {
        &self.name
    }

    /// Expand choice spec into implementation of Choice trait
    pub fn expand(
        &self,
        paths: &Paths,
        template_or_interface: &Ident,
        generics: &Generics,
    ) -> TokenStream {
        let types = paths.types();

        let argument = self.argument();
        let result = self.result();
        let consuming = self.consuming();
        let name = self.name().expand(paths);

        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_generics
                #types::Choice<#template_or_interface #type_generics>
                for #argument
                #where_clause
            {
                const CONSUMING: bool = #consuming;
                const NAME: #types::Name = #name;
                type Result = #result;
            }
        }
    }

    /// Expand choice specs into implementations of Choice trait
    pub fn expand_multiple(
        choices: &[Self],
        paths: &Paths,
        template_or_interface: &Ident,
        generics: &Generics,
    ) -> Vec<TokenStream> {
        choices
            .iter()
            .map(|choice| choice.expand(paths, template_or_interface, generics))
            .collect()
    }
}
