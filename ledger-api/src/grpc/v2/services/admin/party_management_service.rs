use ledger_api_proto::com::daml::ledger::api::v2::admin::{
    self as proto, party_management_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::{LedgerString, ParticipantId, PartyId, SynchronizerId, UserId},
    v2::PartyDetails,
    value::v2::errors::{IntoValueError as _, ValueError},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::PartyManagementServiceClient`]
///
/// This service allows inspecting the party management state of the ledger known to the participant
/// and managing the participant-local party metadata.
///
/// The authorization rules for its RPCs are specified as boolean expressions over these facts:
///
/// 1. `HasRight(r)` denoting whether the authenticated user has right `r` and
/// 2. `IsAuthenticatedIdentityProviderAdmin(idp)` denoting whether `idp` is equal to the
///    `identity_provider_id` of the authenticated user and the user has an IdentityProviderAdmin
///    right.
///
/// If `identity_provider_id` is set to `None`, then it's effectively set to the value of access
/// token's 'iss' field if that is provided. If `identity_provider_id` remains empty, the default
/// identity provider will be assumed.
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

    /// Allocates a new party on a ledger and adds it to the set managed by the participant.
    ///
    /// Caller specifies a party identifier suggestion, the actual identifier allocated might be
    /// different and is implementation specific.
    ///
    /// Caller can specify party metadata that is stored locally on the participant.
    ///
    /// This call may:
    ///
    /// - Succeed, in which case the actual allocated identifier is visible in the response.
    /// - Respond with an error
    pub async fn allocate_party(
        &mut self,
        request: AllocatePartyRequest,
    ) -> Result<PartyDetails, CantonError> {
        let request = proto::AllocatePartyRequest::from(request);
        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.allocate_party(req).await
            })
            .await?
            .party_details
            .required_of::<proto::AllocatePartyResponse>("party_details")
            .no_msg()?
            .try_into()
            .validated_of::<proto::AllocatePartyResponse>("party_details")
            .no_msg()
            .map_err(CantonError::value_error)
    }

    /// Return the identifier of the participant.
    ///
    /// All horizontally scaled replicas should return the same ID.
    pub async fn get_participant_id(&mut self) -> Result<ParticipantId, CantonError> {
        self.retry_handler
            .call(&self.service, &(), |mut svc, _| async move {
                svc.get_participant_id(proto::GetParticipantIdRequest {})
                    .await
            })
            .await?
            .participant_id
            .try_into()
            .validated_of::<proto::GetParticipantIdResponse>("participant_id")
            .no_msg()
            .map_err(CantonError::value_error)
    }

    /// Get the party details of the given parties. Only known parties will be returned in the list.
    ///
    /// ## Arguments
    ///
    /// - `parties` - stable, unique identifier of the Daml parties
    /// - `identity_provider_id` - ID of the identity provider whose parties should be retrieved. If
    ///   not set, assume the party is managed by the default identity provider or party is not
    ///   hosted by the participant.
    pub async fn get_parties(
        &mut self,
        parties: Vec<PartyId>,
        identity_provider_id: Option<LedgerString>,
    ) -> Result<Vec<PartyDetails>, CantonError> {
        let request = proto::GetPartiesRequest {
            parties: parties.into_iter().map(Into::into).collect(),
            identity_provider_id: identity_provider_id.map(Into::into).unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_parties(req).await
            })
            .await?
            .party_details
            .into_iter()
            .enumerate()
            .map(|(idx, pd)| {
                PartyDetails::try_from(pd)
                    .validated_of::<proto::GetPartiesResponse>("party_details")
                    .with_msg_owned(format!("failed to convert party_details[{idx}]"))
                    .map_err(CantonError::value_error)
            })
            .collect()
    }

    /// List the parties known by the participant.
    ///
    /// The list returned contains parties whose ledger access is facilitated by the participant and
    /// the ones maintained elsewhere.
    ///
    /// ## Arguments
    ///
    /// - `page_token` - Pagination token to determine the specific page to fetch. Using the token
    ///   guarantees that parties on a subsequent page are all lexically greater than the last party
    ///   on a previous page. Server does not store intermediate results between calls chained by a
    ///   series of page tokens. As a consequence, if new parties are being added and a page is
    ///   requested twice using the same token, more parties can be returned on the second call. Set
    ///   to `None` to fetch the first page.
    /// - `page_size` - Maximum number of results to be returned by the server. The server will
    ///   return no more than that many results, but it might return fewer. Set to `None`, the
    ///   server will decide the number of results to be returned. If the page_size exceeds the
    ///   maximum supported by the server, an error will be returned. To obtain the server's maximum
    ///   consult the [`PartyManagementFeature`][ledger_api_types::v2::PartyManagementFeature]
    ///   descriptor available in the [`VersionServiceClient`][crate::grpc::v2::services::VersionServiceClient].
    /// - `identity_provider_id` - ID of the identity provider whose parties should be retrieved.
    ///   If set to `None`, assume the party is managed by the default identity provider or party is
    ///   not hosted by the participant.
    /// - `filter_party` - Filter for the party name, searching for all party names known to this
    ///   node starting with the given prefix. This can either be just a string or extend up to the
    ///   full identifier.
    pub async fn list_known_parties(
        &mut self,
        page_token: Option<String>,
        page_size: Option<i32>,
        identity_provider_id: Option<LedgerString>,
        filter_party: Option<String>,
    ) -> Result<KnownParties, CantonError> {
        let request = proto::ListKnownPartiesRequest {
            page_token: page_token.unwrap_or_default(),
            page_size: page_size.unwrap_or_default(),
            identity_provider_id: identity_provider_id.map(Into::into).unwrap_or_default(),
            filter_party: filter_party.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_known_parties(req).await
            })
            .await?
            .try_into()
            .map_err(CantonError::value_error)
    }

    // TODO: implement missing methods of this service
}

/// Party allocation request
///
/// ## Authorization
///    
/// ```plaintext
/// HasRight(ParticipantAdmin)
///     OR IsAuthenticatedIdentityProviderAdmin(identity_provider_id)
///     OR IsAuthenticatedUser(user_id)
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AllocatePartyRequest {
    /// A hint to the participant which party ID to allocate. It can be ignored.
    pub party_id_hint: Option<PartyId>,

    /// Identity provider ID
    ///
    /// If not set, assume the party is managed by the default identity provider or party is not
    /// hosted by the participant.
    pub identity_provider_id: Option<LedgerString>,

    /// The synchronizer, on which the party should be allocated.
    ///
    /// For backwards compatibility, this field may be omitted, if the participant is connected to
    /// only one synchronizer. Otherwise a synchronizer must be specified.
    pub synchronizer_id: Option<SynchronizerId>,

    /// The user who will get the `act_as` rights to the newly allocated party.
    ///
    /// If not set, no user will get rights to the party.
    pub user_id: Option<UserId>,
    // TODO: add missing local_metadata field
}

impl AllocatePartyRequest {
    /// Create new party allocation request with given party ID hint
    pub fn with_hint(party_id: PartyId) -> Self {
        Self {
            party_id_hint: Some(party_id),
            ..Default::default()
        }
    }
}

impl From<AllocatePartyRequest> for proto::AllocatePartyRequest {
    fn from(value: AllocatePartyRequest) -> Self {
        Self {
            party_id_hint: value.party_id_hint.map(Into::into).unwrap_or_default(),
            local_metadata: None, // TODO: add conversion when this field is implemented
            identity_provider_id: value
                .identity_provider_id
                .map(Into::into)
                .unwrap_or_default(),
            synchronizer_id: value.synchronizer_id.map(Into::into).unwrap_or_default(),
            user_id: value.user_id.map(Into::into).unwrap_or_default(),
        }
    }
}

/// Known parties
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownParties {
    /// The details of all Daml parties known by the participant.
    pub party_details: Vec<PartyDetails>,

    /// Pagination token to retrieve the next page. Set to `None`, if there are no further results.
    pub next_page_token: Option<String>,
}

impl TryFrom<proto::ListKnownPartiesResponse> for KnownParties {
    type Error = ValueError;

    fn try_from(value: proto::ListKnownPartiesResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            party_details: value
                .party_details
                .into_iter()
                .enumerate()
                .map(|(idx, pd)| {
                    PartyDetails::try_from(pd)
                        .validated_of::<proto::ListKnownPartiesResponse>("party_details")
                        .with_msg_owned(format!("failed to convert party_details[{idx}]"))
                })
                .collect::<Result<_, _>>()?,
            next_page_token: (!value.next_page_token.is_empty()).then_some(value.next_page_token),
        })
    }
}
