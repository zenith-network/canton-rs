use std::{collections::BTreeMap, io, pin::pin, time::SystemTime};

use admin_api_types::error::ValueError;
use canton_base::retry::{RetryConfig, RetryHandler};
use canton_proto::com::digitalasset::canton::admin::participant::v30::{
    self as proto, party_management_service_client as svc_proto,
};
use canton_protocol::error::ProtoDecodeError;
use canton_types::{
    ContractId, LedgerString, PackageId, PackageName,
    topology::{ParticipantId, PartyId, SynchronizerId, UniqueIdentifier},
};
use chrono::Duration;
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue, RequiredProtoField as _};
use tokio_stream::{Stream, StreamExt as _, once};

use crate::v30::{client::InterceptedService, error::AdminApiError};

/// Wrapper for [`PartyManagementServiceClient`][svc_proto::PartyManagementServiceClient].
///
/// Provides the participant admin RPCs for offline party replication.
#[derive(Clone, Debug)]
pub struct PartyManagementClient {
    service: svc_proto::PartyManagementServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl PartyManagementClient {
    /// Create a wrapper from the underlying tonic service client.
    pub fn new(
        service: svc_proto::PartyManagementServiceClient<InterceptedService>,
        retry_handler: RetryHandler,
    ) -> Self {
        Self {
            service,
            retry_handler,
        }
    }

    /// Set the retry configuration for the client.
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// Offline party replication focused ACS (Active Contract Set) export.
    ///
    /// Canton finds the party's activation on the target after `begin_offset_exclusive` and
    /// exports contracts for `synchronizer_id` that the target is missing.
    ///
    /// The returned chunks form one gzip snapshot and can be passed to [`Self::import_party_acs`].
    /// Chunk boundaries can split contracts. Full snapshot can be constructed by concatenating
    /// chunks in order of arrival.
    ///
    /// The retry policy applies only when opening the stream. The caller should retry the whole
    /// method if reading one stream item fails.
    ///
    /// # Arguments
    ///
    /// - `party_id` - The identifier of the party being replicated. This party must already be
    ///   active on the target participant for the ACS export to succeed.
    /// - `synchronizer_id` - The identifier of the synchronizer managing the contracts to be
    ///   exported. Scopes the ACS export to contracts assigned to this synchronizer.
    /// - `target_participant_id` - The unique identifier of the target participant where the party
    ///   will be replicated. Scopes the ACS export to contracts missing from this participant.
    /// - `begin_offset_exclusive` - The exclusive ledger offset used as a starting point to find
    ///   the party's most recent activation on the target participant.
    /// - `wait_for_activation_timeout` - The maximum duration the service will wait to find the
    ///   topology transaction that activates the party on the target participant. If not set,
    ///   Canton will use a default timeout.
    pub async fn export_party_acs(
        &self,
        party_id: PartyId,
        synchronizer_id: SynchronizerId,
        target_participant_id: ParticipantId,
        begin_offset_exclusive: i64,
        wait_for_activation_timeout: Option<Duration>,
    ) -> Result<impl Stream<Item = Result<Vec<u8>, AdminApiError>>, AdminApiError> {
        let request = proto::ExportPartyAcsRequest {
            party_id: party_id.into(),
            synchronizer_id: synchronizer_id.into(),
            target_participant_uid: UniqueIdentifier::from(target_participant_id).into(),
            begin_offset_exclusive,
            wait_for_activation_timeout: wait_for_activation_timeout.map(Into::into),
        };

        let stream = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.export_party_acs(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?;

        Ok(stream.map(|item| {
            item.map(|response| response.chunk)
                .map_err(AdminApiError::from_status)
        }))
    }

    /// Similar to [`Self::export_party_acs`], but this function reads all the chunks into a single
    /// buffer.
    ///
    /// Note that the size of the buffer grows dynamically to fill all the received chunks. To avoid
    /// memory overflows, the user should consider using [`Self::export_party_acs`] when he's not
    /// sure about the size of the requested snapshot.
    pub async fn export_party_acs_full(
        &self,
        party_id: PartyId,
        synchronizer_id: SynchronizerId,
        target_participant_id: ParticipantId,
        begin_offset_exclusive: i64,
        wait_for_activation_timeout: Option<Duration>,
    ) -> Result<Vec<u8>, AdminApiError> {
        let mut chunks = pin!(
            self.export_party_acs(
                party_id,
                synchronizer_id,
                target_participant_id,
                begin_offset_exclusive,
                wait_for_activation_timeout,
            )
            .await?
        );

        let mut snapshot = chunks.try_next().await?.unwrap_or_default();

        while let Some(chunk) = chunks.try_next().await? {
            snapshot.extend(chunk);
        }

        Ok(snapshot)
    }

    /// Offline party replication focused ACS (Active Contract Set) import.
    ///
    /// Imports the party's ACS into the target participant.
    ///
    /// This operation assumes the provided snapshot contains the complete and untampered ACS
    /// originating from a trusted source participant. Because the target participant cannot
    /// independently verify the historical provenance of the imported contracts, validation is
    /// performed on a best-effort basis, even if
    /// [`contract_import_mode`][ImportPartyAcsOptions::contract_import_mode] is
    /// [`Validation`][ContractImportMode::Validation].
    /// Use only when the provided snapshot comes from a known, trusted authority.
    ///
    /// Note that this method __does not__ use the retry policy. The caller is expected to re-create
    /// the stream and retry manually.
    pub async fn import_party_acs(
        &self,
        options: ImportPartyAcsOptions,
        chunks: impl Stream<Item = Vec<u8>> + Send + 'static,
    ) -> Result<(), AdminApiError> {
        let mut chunks = Box::pin(chunks);

        // Apply import options to the first request, as Canton expects it
        let chunk = chunks.next().await.ok_or_else(|| {
            AdminApiError::input_error(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "empty ACS snapshot stream",
            ))
        })?;
        let first = options.first_request(chunk);

        // Cast the rest of the stream into the service requests
        let remaining = chunks.map(|chunk| proto::ImportPartyAcsRequest {
            acs_snapshot: chunk,
            ..Default::default()
        });
        let requests = once(first).chain(remaining);

        // Clone so that the method receiver will be &self, to keep the API consistent
        let mut service = self.service.clone();

        service
            .import_party_acs(requests)
            .await
            .map(|_| ())
            .map_err(AdminApiError::from_status)
    }

    /// Similar to [`Self::import_party_acs`], but imports the whole snapshot in one request.
    ///
    /// The snapshot can be constructed by concatenating all chunks in their order.
    ///
    /// Note that unlike [`Self::import_party_acs`], this method __does__ use the retry policy.
    pub async fn import_party_acs_full(
        &self,
        options: ImportPartyAcsOptions,
        acs_snapshot: Vec<u8>,
    ) -> Result<(), AdminApiError> {
        let request = options.first_request(acs_snapshot);
        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                // Since `Once` is not Clone, we wrap request into it here inside the closure
                svc.import_party_acs(once(req))
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?;
        Ok(())
    }

    /// Find the highest clean ledger offset for a record time on the synchronizer.
    ///
    /// By default, Canton requires an event with exactly the requested record time. Set `force`
    /// for disaster recovery to allow a prior event, or the current ledger end for a future time.
    pub async fn get_highest_offset_by_timestamp(
        &self,
        synchronizer_id: SynchronizerId,
        timestamp: SystemTime,
        force: bool,
    ) -> Result<i64, AdminApiError> {
        let request = proto::GetHighestOffsetByTimestampRequest {
            synchronizer_id: synchronizer_id.into(),
            timestamp: Some(timestamp.into()),
            force,
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_highest_offset_by_timestamp(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await
            .map(|response| response.ledger_offset)
    }

    /// Clear the onboarding flag for a party on this participant.
    ///
    /// Canton finds the onboarding activation after `begin_offset_exclusive. If clearance is
    /// pending, Canton schedules it in the background. This call is idempotent and can be polled at
    /// or after the returned retry timestamp.
    ///
    /// # Arguments
    ///
    /// - `party_id` - The identifier of the party whose onboarding flag is being cleared. This
    ///   party must already be active on the target participant.
    /// - `synchronizer_id` - The identifier of the synchronizer.
    /// - `begin_offset_exclusive` - The exclusive ledger offset used as a starting point to find
    ///   the party's most recent activation on the target participant.
    /// - `wait_for_activation_timeout` - The maximum duration the service will wait to find the
    ///   topology transaction that activates the party. If not set, the service will use a default
    ///   timeout.
    pub async fn clear_party_onboarding_flag(
        &self,
        party_id: PartyId,
        synchronizer_id: SynchronizerId,
        begin_offset_exclusive: i64,
        wait_for_activation_timeout: Option<Duration>,
    ) -> Result<PartyOnboardingFlagStatus, AdminApiError> {
        let request = proto::ClearPartyOnboardingFlagRequest {
            party_id: party_id.into(),
            synchronizer_id: synchronizer_id.into(),
            begin_offset_exclusive,
            wait_for_activation_timeout: wait_for_activation_timeout.map(Into::into),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.clear_party_onboarding_flag(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .try_into()
            .map_err(AdminApiError::value_error)
    }
}

/// Options sent once at the beginning of an ACS import.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ImportPartyAcsOptions {
    /// Only contracts assigned to this synchronizer are imported.
    pub synchronizer_id: SynchronizerId,

    /// How Canton validates the imported contracts.
    pub contract_import_mode: ContractImportMode,

    /// Prefix for the workflow IDs of transactions generated by the import.
    pub workflow_id_prefix: Option<LedgerString>,

    /// Party being replicated. On protocol version 35 or later, this enables crash-resilient
    /// scheduling of onboarding flag clearance.
    pub party_id: Option<PartyId>,

    /// Override rules for representative package IDs on the target participant.
    pub representative_package_id_override: Option<RepresentativePackageIdOverride>,
}

impl ImportPartyAcsOptions {
    /// Create import options with the required synchronizer and validation mode.
    pub fn new(synchronizer_id: SynchronizerId, contract_import_mode: ContractImportMode) -> Self {
        Self {
            synchronizer_id,
            contract_import_mode,
            workflow_id_prefix: None,
            party_id: None,
            representative_package_id_override: None,
        }
    }

    /// Set party ID.
    pub fn with_party_id(mut self, party_id: PartyId) -> Self {
        self.party_id = Some(party_id);
        self
    }

    fn first_request(self, chunk: Vec<u8>) -> proto::ImportPartyAcsRequest {
        proto::ImportPartyAcsRequest {
            acs_snapshot: chunk,
            synchronizer_id: Some(self.synchronizer_id.into()),
            workflow_id_prefix: self.workflow_id_prefix.map(Into::into),
            contract_import_mode: Some(
                proto::ContractImportMode::from(self.contract_import_mode) as i32
            ),
            representative_package_id_override: self
                .representative_package_id_override
                .map(Into::into),
            party_id: self.party_id.map(Into::into),
        }
    }
}

/// Validation mode for ACS imports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContractImportMode {
    /// Validate each contract, including authentication of its contract ID.
    Validation,

    /// Accept contracts without validation. Use when the contracts are known to be valid for
    /// their representative package IDs.
    Accept,
}

impl From<ContractImportMode> for proto::ContractImportMode {
    fn from(value: ContractImportMode) -> Self {
        match value {
            ContractImportMode::Validation => Self::Validation,
            ContractImportMode::Accept => Self::Accept,
        }
    }
}

/// Override rules Canton applies when assigning representative package IDs during import.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RepresentativePackageIdOverride {
    /// Overrides by contract ID.
    pub contract_override: BTreeMap<ContractId, PackageId>,
    /// Overrides by creation or representative package ID.
    pub package_id_override: BTreeMap<PackageId, PackageId>,
    /// Overrides by package name.
    pub package_name_override: BTreeMap<PackageName, PackageId>,
}

impl From<RepresentativePackageIdOverride> for proto::RepresentativePackageIdOverride {
    fn from(value: RepresentativePackageIdOverride) -> Self {
        Self {
            contract_override: value
                .contract_override
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            package_id_override: value
                .package_id_override
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            package_name_override: value
                .package_name_override
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.into()))
                .collect(),
        }
    }
}

/// Result of a call to clear a party's onboarding flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PartyOnboardingFlagStatus {
    /// The flag is cleared.
    Cleared,

    /// The flag is still set and Canton has scheduled its clearance.
    Pending {
        /// The earliest time to poll Canton for clearance.
        earliest_retry_timestamp: SystemTime,
    },
}

impl TryFrom<proto::ClearPartyOnboardingFlagResponse> for PartyOnboardingFlagStatus {
    type Error = ValueError;

    fn try_from(value: proto::ClearPartyOnboardingFlagResponse) -> Result<Self, Self::Error> {
        type Proto = proto::ClearPartyOnboardingFlagResponse;

        match (value.onboarded, value.earliest_retry_timestamp) {
            (true, None) => Ok(Self::Cleared),
            (true, Some(_)) => Err(InvalidProtoFieldValue::new_of::<Proto>(
                "earliest_retry_timestamp",
                ProtoDecodeError::other("unexpected retry timestamp for a cleared onboarding flag"),
            )
            .into()),
            (false, timestamp) => {
                let earliest_retry_timestamp = timestamp
                    .required_of::<Proto>("earliest_retry_timestamp")?
                    .try_into()
                    .validated_of::<Proto>("earliest_retry_timestamp")?;
                Ok(Self::Pending {
                    earliest_retry_timestamp,
                })
            }
        }
    }
}
