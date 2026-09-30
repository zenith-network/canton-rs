use syn::{Generics, Path, Type, WherePredicate, punctuated::Punctuated, token::Comma};

pub fn apply_bounds<'a>(
    generics: &Generics,
    field_types: impl IntoIterator<Item = &'a Type>,
    trait_bounds: &[&Path],
    explicit_bounds: impl IntoIterator<Item = &'a WherePredicate>,
) -> Generics {
    let mut output = generics.clone();
    let predicates = &mut output.make_where_clause().predicates;

    for field_type in field_types {
        for trait_bound in trait_bounds {
            push_unique(predicates, syn::parse_quote! { #field_type: #trait_bound });
        }
    }

    for explicit_bound in explicit_bounds {
        push_unique(predicates, explicit_bound.clone());
    }

    output
}

fn push_unique(predicates: &mut Punctuated<WherePredicate, Comma>, predicate: WherePredicate) {
    if !predicates.iter().any(|existing| existing == &predicate) {
        predicates.push(predicate);
    }
}
