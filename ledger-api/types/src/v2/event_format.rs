use std::collections::HashMap;

use canton_types::PartyId;
use ledger_api_proto::com::daml::ledger::api::v2 as proto;

use crate::v2::Filters;

/// Runtime-defined event format
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventFormat {
    pub filters_by_party: HashMap<PartyId, Filters>,
    pub filters_for_any_party: Option<Filters>,
    pub verbose: bool,
}

impl EventFormat {
    pub fn new() -> Self {
        Self {
            filters_by_party: HashMap::new(),
            filters_for_any_party: None,
            verbose: false,
        }
    }
}

impl From<EventFormat> for proto::EventFormat {
    fn from(value: EventFormat) -> Self {
        Self {
            filters_by_party: value
                .filters_by_party
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
            filters_for_any_party: value.filters_for_any_party.map(Into::into),
            verbose: value.verbose,
        }
    }
}
