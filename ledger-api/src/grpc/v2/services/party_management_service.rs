use ledger_api_proto::com::daml::ledger::api::v2::admin::{
    ListKnownPartiesRequest, ListKnownPartiesResponse, party_management_service_client as svc_proto,
};

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::PartyManagementServiceClient`]
#[derive(Clone, Debug)]
pub struct PartyManagementServiceClient {
    service: svc_proto::PartyManagementServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl PartyManagementServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::PartyManagementServiceClient<InterceptedService>,
        retry_handler: RetryHandler,
    ) -> Self {
        Self {
            service,
            retry_handler,
        }
    }

    /// Set retry config for the client
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// Lists every party the participant knows, local or hosted elsewhere on its synchronizers,
    /// following the page tokens to the last page. Pages use the participant's default size.
    pub async fn list_known_parties(&mut self) -> Result<Vec<KnownParty>, CantonError> {
        let mut parties = Vec::new();
        let mut page_token = String::new();
        loop {
            let request = ListKnownPartiesRequest {
                page_token,
                ..Default::default()
            };
            let page = self
                .retry_handler
                .call(&self.service, &request, |mut svc, req| async move {
                    svc.list_known_parties(req).await
                })
                .await?;
            match absorb_page(page, &mut parties) {
                Some(next) => page_token = next,
                None => return Ok(parties),
            }
        }
    }
}

/// A party listed by `ListKnownParties`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownParty {
    /// The party id.
    pub party: String,
    /// Whether the party is hosted on the participant serving the request.
    pub is_local: bool,
}

/// Appends a page's parties to `parties` and returns the next page token, `None` on the last page.
fn absorb_page(page: ListKnownPartiesResponse, parties: &mut Vec<KnownParty>) -> Option<String> {
    parties.extend(page.party_details.into_iter().map(|d| KnownParty {
        party: d.party,
        is_local: d.is_local,
    }));
    (!page.next_page_token.is_empty()).then_some(page.next_page_token)
}

#[cfg(test)]
mod tests {
    use ledger_api_proto::com::daml::ledger::api::v2::admin::PartyDetails;

    use super::*;

    fn page(parties: &[(&str, bool)], next: &str) -> ListKnownPartiesResponse {
        ListKnownPartiesResponse {
            party_details: parties
                .iter()
                .map(|(party, is_local)| PartyDetails {
                    party: party.to_string(),
                    is_local: *is_local,
                    ..Default::default()
                })
                .collect(),
            next_page_token: next.to_string(),
        }
    }

    #[test]
    fn pages_accumulate_until_the_token_is_empty() {
        let mut parties = Vec::new();
        assert_eq!(
            absorb_page(page(&[("alice::1", true)], "p2"), &mut parties),
            Some("p2".to_string())
        );
        assert_eq!(
            absorb_page(page(&[("bob::2", false)], ""), &mut parties),
            None
        );
        assert_eq!(
            parties,
            vec![
                KnownParty {
                    party: "alice::1".into(),
                    is_local: true
                },
                KnownParty {
                    party: "bob::2".into(),
                    is_local: false
                },
            ]
        );
    }

    #[test]
    fn an_empty_last_page_ends_the_listing() {
        let mut parties = Vec::new();
        assert_eq!(absorb_page(page(&[], ""), &mut parties), None);
        assert!(parties.is_empty());
    }
}
