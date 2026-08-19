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

    pub fn with_filter(mut self, party_id: PartyId, filter: Filters) -> Self {
        self.filters_by_party.insert(party_id, filter);
        self
    }

    pub fn with_filter_for_any(mut self, filter: Filters) -> Self {
        self.filters_for_any_party = Some(filter);
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
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
