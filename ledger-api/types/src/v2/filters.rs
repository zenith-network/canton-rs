//! Dynamic filters (defined at runtime)

use canton_types::PackageName;
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
