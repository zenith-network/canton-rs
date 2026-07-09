use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::Package;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind<'a> {
    Star,
    Arrow(Arrow<'a>),
    Nat,
}

impl<'a> Kind<'a> {
    pub(crate) fn from_unsealed(kind: &'a proto::Kind, package: Package<'a>) -> Self {
        match kind.sum.as_ref().unwrap() {
            proto::kind::Sum::Star(_) => Kind::Star,
            proto::kind::Sum::Arrow(arrow) => Kind::Arrow(Arrow::from_unsealed(arrow, package)),
            proto::kind::Sum::Nat(_) => Kind::Nat,
            proto::kind::Sum::InternedKind(index) => package.get_interned_kind(*index),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Arrow<'a> {
    package: Package<'a>,
    unsealed: &'a proto::kind::Arrow,
}

impl<'a> Arrow<'a> {
    pub(crate) fn from_unsealed(unsealed: &'a proto::kind::Arrow, package: Package<'a>) -> Self {
        Self { package, unsealed }
    }

    pub fn as_unsealed(&self) -> &'a proto::kind::Arrow {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn params(&self) -> Vec<Kind<'a>> {
        self.unsealed
            .params
            .iter()
            .map(|kind| Kind::from_unsealed(kind, self.package))
            .collect()
    }

    pub fn result(&self) -> Kind<'a> {
        Kind::from_unsealed(self.unsealed.result.as_ref().unwrap(), self.package)
    }
}

impl fmt::Debug for Arrow<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arrow")
            .field("package", &format_args!("{:#?}", self.package))
            .field("params", &self.params())
            .field("result", &self.result())
            .finish()
    }
}

impl PartialEq for Arrow<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Arrow<'_> {}

impl Hash for Arrow<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
