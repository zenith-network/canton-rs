use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::errors::{IndexOutOfBounds, InternedDottedNameError, InternedStringError};

pub fn seal_interned_str(idx: i32, package: &proto::Package) -> Result<&str, InternedStringError> {
    let idx: usize = idx.try_into()?;
    package
        .interned_strings
        .get(idx)
        .ok_or(IndexOutOfBounds::new(idx, package.interned_strings.len()).into())
        .map(String::as_str)
}

pub fn seal_interned_dotted_name(
    idx: i32,
    package: &proto::Package,
) -> Result<Vec<&str>, InternedDottedNameError> {
    let idx: usize = idx.try_into()?;
    package
        .interned_dotted_names
        .get(idx)
        .ok_or(IndexOutOfBounds::new(
            idx,
            package.interned_dotted_names.len(),
        ))?
        .segments_interned_str
        .iter()
        .map(|idx| {
            let idx: usize = (*idx).try_into()?;
            package
                .interned_strings
                .get(idx)
                .ok_or(IndexOutOfBounds::new(idx, package.interned_dotted_names.len()).into())
                .map(String::as_str)
        })
        .collect::<Result<_, _>>()
}
