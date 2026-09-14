//! Dynamic filters (defined at runtime)

use canton_types::{PackageId, PackageName, ParticipantId, SynchronizerId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::Identifier;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Filters {
    pub cumulative: Vec<CumulativeFilter>,
}

impl Filters {
    pub const fn new() -> Self {
        Self {
            cumulative: Vec::new(),
        }
    }

    pub fn from_filter(filter: CumulativeFilter) -> Self {
        Self {
            cumulative: vec![filter],
        }
    }

    pub fn with_filter(&mut self, filter: CumulativeFilter) -> &mut Self {
        self.cumulative.push(filter);
        self
    }

    pub fn template(template_id: Identifier<PackageName>) -> Self {
        Self {
            cumulative: vec![TemplateFilter::from(template_id).into()],
        }
    }

    pub fn templates(template_ids: Vec<Identifier<PackageName>>) -> Self {
        Self {
            cumulative: template_ids
                .into_iter()
                .map(|template_id| TemplateFilter::from(template_id).into())
                .collect(),
        }
    }

    pub fn interface(interface_id: Identifier<PackageName>) -> Self {
        Self {
            cumulative: vec![TemplateFilter::from(interface_id).into()],
        }
    }

    pub fn interfaces(interface_ids: Vec<Identifier<PackageName>>) -> Self {
        Self {
            cumulative: interface_ids
                .into_iter()
                .map(|template_id| InterfaceFilter::from(template_id).into())
                .collect(),
        }
    }

    pub fn wildcard() -> Self {
        Self {
            cumulative: vec![WildcardFilter::default().into()],
        }
    }
}

impl From<Filters> for proto::Filters {
    fn from(value: Filters) -> Self {
        Self {
            cumulative: value.cumulative.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CumulativeFilter {
    Wildcard(WildcardFilter),
    Interface(InterfaceFilter),
    Template(TemplateFilter),
}

impl From<WildcardFilter> for CumulativeFilter {
    fn from(value: WildcardFilter) -> Self {
        Self::Wildcard(value)
    }
}

impl From<InterfaceFilter> for CumulativeFilter {
    fn from(value: InterfaceFilter) -> Self {
        Self::Interface(value)
    }
}

impl From<TemplateFilter> for CumulativeFilter {
    fn from(value: TemplateFilter) -> Self {
        Self::Template(value)
    }
}

impl From<CumulativeFilter> for proto::CumulativeFilter {
    fn from(value: CumulativeFilter) -> Self {
        use proto::cumulative_filter::IdentifierFilter;
        Self {
            identifier_filter: Some(match value {
                CumulativeFilter::Wildcard(wildcard_filter) => {
                    IdentifierFilter::WildcardFilter(wildcard_filter.into())
                }
                CumulativeFilter::Interface(interface_filter) => {
                    IdentifierFilter::InterfaceFilter(interface_filter.into())
                }
                CumulativeFilter::Template(template_filter) => {
                    IdentifierFilter::TemplateFilter(template_filter.into())
                }
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct WildcardFilter {
    pub include_created_event_blob: bool,
}

impl WildcardFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include_created_event_blob(&mut self) -> &mut Self {
        self.include_created_event_blob = true;
        self
    }
}

impl From<WildcardFilter> for proto::WildcardFilter {
    fn from(value: WildcardFilter) -> Self {
        Self {
            include_created_event_blob: value.include_created_event_blob,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TemplateFilter {
    /// We use package-name here because package-id is marked as deprecated in Protobuf
    pub template_id: Identifier<PackageName>,
    pub include_created_event_blob: bool,
}

impl TemplateFilter {
    pub fn new(template_id: Identifier<PackageName>) -> Self {
        Self {
            template_id,
            include_created_event_blob: false,
        }
    }

    pub fn include_created_event_blob(&mut self) -> &mut Self {
        self.include_created_event_blob = true;
        self
    }
}

impl From<Identifier<PackageName>> for TemplateFilter {
    fn from(value: Identifier<PackageName>) -> Self {
        Self::new(value)
    }
}

impl From<TemplateFilter> for proto::TemplateFilter {
    fn from(value: TemplateFilter) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            include_created_event_blob: value.include_created_event_blob,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InterfaceFilter {
    /// We use package-name here because package-id is marked as deprecated in Protobuf
    pub interface_id: Identifier<PackageName>,
    pub include_interface_view: bool,
    pub include_created_event_blob: bool,
}

impl InterfaceFilter {
    pub fn new(interface_id: Identifier<PackageName>) -> Self {
        Self {
            interface_id,
            include_interface_view: false,
            include_created_event_blob: false,
        }
    }

    pub fn include_interface_view(&mut self) -> &mut Self {
        self.include_interface_view = true;
        self
    }

    pub fn include_created_event_blob(&mut self) -> &mut Self {
        self.include_created_event_blob = true;
        self
    }
}

impl From<Identifier<PackageName>> for InterfaceFilter {
    fn from(value: Identifier<PackageName>) -> Self {
        Self::new(value)
    }
}

impl From<InterfaceFilter> for proto::InterfaceFilter {
    fn from(value: InterfaceFilter) -> Self {
        Self {
            interface_id: Some(value.interface_id.into()),
            include_interface_view: value.include_interface_view,
            include_created_event_blob: value.include_created_event_blob,
        }
    }
}

/// Filter the [`VettedPackages`][ledger_api_types::v2::VettedPackages] by package metadata.
///
/// A [`PackageMetadataFilter`] without `package_ids` and without `package_name_prefixes` matches
/// any vetted package.
///
/// Non-empty fields specify candidate values of which at least one must match. If both fields are
/// set, then a candidate is returned if it matches one of the fields.
#[derive(Clone, Debug, Default)]
pub struct PackageMetadataFilter {
    /// If this list is non-empty, any vetted package with a package ID in this list will match the
    /// filter.
    pub package_ids: Vec<PackageId>,

    /// If this list is non-empty, any vetted package with a name matching at least one prefix in
    /// this list will match the filter.
    pub package_name_prefixes: Vec<String>,
}

impl PackageMetadataFilter {
    /// Crate a new empty filter, which matches any vetted package.
    pub const fn any() -> Self {
        Self {
            package_ids: Vec::new(),
            package_name_prefixes: Vec::new(),
        }
    }

    /// Add a package ID to the filter.
    pub fn with_package_id(mut self, package_id: PackageId) -> Self {
        self.package_ids.push(package_id);
        self
    }

    /// Add a package name prefix to the filter.
    pub fn with_package_name_prefix(mut self, prefix: String) -> Self {
        self.package_name_prefixes.push(prefix);
        self
    }

    /// Returns `true` if the filter is "any" (matches everything).
    pub const fn is_any(&self) -> bool {
        self.package_ids.is_empty() && self.package_name_prefixes.is_empty()
    }
}

impl From<PackageMetadataFilter> for proto::PackageMetadataFilter {
    fn from(value: PackageMetadataFilter) -> Self {
        Self {
            package_ids: value.package_ids.into_iter().map(Into::into).collect(),
            package_name_prefixes: value.package_name_prefixes,
        }
    }
}

/// Filter the [`VettedPackages`][ledger_api_types::v2::VettedPackages] by the participant and
/// synchronizer that they are hosted on.
///
/// Empty fields are ignored, such that a [`TopologyStateFilter`] without `participant_ids` and
/// without `synchronizer_ids` matches a vetted package hosted on any participant and synchronizer.
///
/// Non-empty fields specify candidate values of which at least one must match. If both fields are
/// set then at least one candidate value must match from each field.
#[derive(Clone, Debug, Default)]
pub struct TopologyStateFilter {
    /// If this list is non-empty, only vetted packages hosted on participants listed in this field
    /// match the filter.
    ///
    /// Query the current Ledger API's participant's ID via the public `get_participant_id` method
    /// in `PartyManagementService`.
    pub participant_ids: Vec<ParticipantId>,

    /// If this list is non-empty, only vetted packages from the topology state of the synchronizers
    /// in this list match the filter.
    pub synchronizer_ids: Vec<SynchronizerId>,
}

impl TopologyStateFilter {
    /// Create a new empty filter, which matches vetted packages on any participant and any
    /// synchronizer.
    pub const fn any() -> Self {
        Self {
            participant_ids: Vec::new(),
            synchronizer_ids: Vec::new(),
        }
    }

    pub fn on_participant(mut self, participant_id: ParticipantId) -> Self {
        self.participant_ids.push(participant_id);
        self
    }

    pub fn on_synchronizer(mut self, synchronizer_id: SynchronizerId) -> Self {
        self.synchronizer_ids.push(synchronizer_id);
        self
    }

    /// Returns `true` if the filter is "any" (matches everything).
    pub const fn is_any(&self) -> bool {
        self.participant_ids.is_empty() && self.synchronizer_ids.is_empty()
    }
}

impl From<TopologyStateFilter> for proto::TopologyStateFilter {
    fn from(value: TopologyStateFilter) -> Self {
        Self {
            participant_ids: value.participant_ids.into_iter().map(Into::into).collect(),
            synchronizer_ids: value.synchronizer_ids.into_iter().map(Into::into).collect(),
        }
    }
}
