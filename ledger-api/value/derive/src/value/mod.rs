use proc_macro2::{Span, TokenStream};
use syn::{Data, DeriveInput, Error};

mod attributes;
mod enum_;
mod struct_;

use attributes::ItemAttributes;

pub fn impl_value(input: DeriveInput) -> Result<TokenStream, Error> {
    let item_attrs = ItemAttributes::parse(&input.attrs)?;
    let generics = input.generics;

    match &input.data {
        Data::Struct(ds) => struct_::try_impl_record(&item_attrs, &input.ident, ds, &generics),
        Data::Enum(de) => enum_::try_impl_value_enum(&item_attrs, &input.ident, de, &generics),
        Data::Union(_) => Err(Error::new(
            Span::call_site(),
            "LedgerApiValue cannot be applied to union types",
        )),
    }
}
