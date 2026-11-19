//! Offline party replication requests and responses against a real participant.

use std::time::Duration;

use admin_api::v30::{
    AdminApiClient,
    services::{
        PartyManagementClient,
        participant_status::ParticipantStatus,
        party_management::{ContractImportMode, ImportPartyAcsOptions, PartyOnboardingFlagStatus},
    },
};
use admin_api_proto::com::digitalasset::canton::{
    admin::participant::v30 as p, topology::admin::v30 as common,
};
use canton_base::retry::RetryConfig;
use canton_proto::com::digitalasset::canton::protocol::v30 as m;
use canton_types::{
    LedgerString,
    topology::{PartyId, SynchronizerId},
};
use ledger_api_proto::com::daml::ledger::api::v2::{
    self as ledger,
    admin::{AllocatePartyRequest, party_management_service_client::PartyManagementServiceClient},
    state_service_client::StateServiceClient,
    update_service_client::UpdateServiceClient,
};
use pretty_assertions::assert_eq;
use tokio_stream::{StreamExt as _, iter};
use tonic::transport::{Channel, Endpoint};
use tonic_types::StatusExt as _;

use crate::support::{
    Sandbox, ok, sandbox,
    topology::{Context, assert_error, proposal, timestamp},
    within,
};

type RawPartyClient = p::party_management_service_client::PartyManagementServiceClient<Channel>;

const ACTIVATION_TIMEOUT: chrono::Duration = chrono::Duration::seconds(10);

/// An observed activation provides the timestamp and starting offset for the replication RPCs.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn export_offset_and_clearance_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let (service, synchronizer) = party_service(&ctx.endpoint).await;
    let ledger_endpoint = sandbox.ledger_api_endpoint();
    let ledger_channel = ok(
        ledger_endpoint,
        Endpoint::new(ledger_endpoint.to_owned()).unwrap().connect(),
    )
    .await;

    // Allocate a regular party so the ledger has a positive offset before the activation.
    ok(
        ledger_endpoint,
        PartyManagementServiceClient::new(ledger_channel.clone()).allocate_party(
            AllocatePartyRequest {
                party_id_hint: "replication-existing".into(),
                ..Default::default()
            },
        ),
    )
    .await;
    let mut state = StateServiceClient::new(ledger_channel.clone());
    let begin = ok(
        ledger_endpoint,
        state.get_ledger_end(ledger::GetLedgerEndRequest::default()),
    )
    .await
    .into_inner()
    .offset;
    assert!(begin > 0);

    let mut mapping = ctx.party("replication-onboarding");
    let party = PartyId::parse(&mapping.party).unwrap();
    mapping.participants[0].onboarding =
        Some(m::party_to_participant::hosting_participant::Onboarding {});
    let mut request = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(mapping),
        None,
    );
    request.store = Some(common::StoreId {
        store: Some(common::store_id::Store::Synchronizer(
            common::Synchronizer {
                kind: Some(common::synchronizer::Kind::Id(synchronizer.to_string())),
            },
        )),
    });
    request.wait_to_become_effective = Some(ACTIVATION_TIMEOUT.into());
    ok(&ctx.endpoint, ctx.raw_write().authorize(request)).await;

    let activation = observe_activation(ledger_endpoint, ledger_channel, begin, &party).await;
    assert_eq!(activation.synchronizer_id, synchronizer.to_string());
    let record_time = activation.record_time.unwrap();
    let mut raw = RawPartyClient::new(ctx.channel.clone());
    for (time, force) in [
        (timestamp(&record_time), false),
        (timestamp(&record_time) + Duration::from_secs(3600), true),
    ] {
        let expected = ok(
            &ctx.endpoint,
            raw.get_highest_offset_by_timestamp(p::GetHighestOffsetByTimestampRequest {
                synchronizer_id: synchronizer.to_string(),
                timestamp: Some(time.into()),
                force,
            }),
        )
        .await
        .into_inner()
        .ledger_offset;
        let actual = ok(
            &ctx.endpoint,
            service.get_highest_offset_by_timestamp(synchronizer.clone(), time, force),
        )
        .await;
        assert_eq!(actual, expected);
        if !force {
            assert_eq!(actual, activation.offset);
        }
    }

    let mut raw_export = ok(
        &ctx.endpoint,
        raw.export_party_acs(p::ExportPartyAcsRequest {
            party_id: party.to_string(),
            synchronizer_id: synchronizer.to_string(),
            target_participant_uid: ctx.participant.uid().to_string(),
            begin_offset_exclusive: begin,
            wait_for_activation_timeout: Some(ACTIVATION_TIMEOUT.into()),
        }),
    )
    .await
    .into_inner();
    let expected = ok(&ctx.endpoint, async {
        let mut snapshot = Vec::new();
        while let Some(response) = raw_export.message().await? {
            snapshot.extend(response.chunk);
        }
        Ok::<_, tonic::Status>(snapshot)
    })
    .await;
    let exported = ok(
        &ctx.endpoint,
        service.export_party_acs(
            party.clone(),
            synchronizer.clone(),
            ctx.participant.clone(),
            begin,
            Some(ACTIVATION_TIMEOUT),
        ),
    )
    .await;
    let actual = ok(&ctx.endpoint, async {
        tokio::pin!(exported);
        let mut snapshot = Vec::new();
        while let Some(chunk) = exported.next().await {
            snapshot.extend(chunk?);
        }
        Ok::<_, admin_api::v30::error::AdminApiError>(snapshot)
    })
    .await;
    assert!(actual.starts_with(&[0x1f, 0x8b]), "gzip snapshot");
    assert_eq!(actual, expected);

    let raw_clearance = ok(
        &ctx.endpoint,
        raw.clear_party_onboarding_flag(p::ClearPartyOnboardingFlagRequest {
            party_id: party.to_string(),
            synchronizer_id: synchronizer.to_string(),
            begin_offset_exclusive: begin,
            wait_for_activation_timeout: Some(ACTIVATION_TIMEOUT.into()),
        }),
    )
    .await
    .into_inner();
    let expected = if raw_clearance.onboarded {
        assert_eq!(raw_clearance.earliest_retry_timestamp, None);
        PartyOnboardingFlagStatus::Cleared
    } else {
        PartyOnboardingFlagStatus::Pending {
            earliest_retry_timestamp: timestamp(&raw_clearance.earliest_retry_timestamp.unwrap()),
        }
    };
    let actual = ok(
        &ctx.endpoint,
        service.clear_party_onboarding_flag(party, synchronizer, begin, Some(ACTIVATION_TIMEOUT)),
    )
    .await;
    assert_eq!(actual, expected);
}

/// Canton reads the import metadata and detects that the requested party lacks an onboarding mapping.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn import_requires_the_partys_onboarding_mapping(sandbox: &Sandbox) {
    // A complete gzip member with an empty payload, split across the gzip header.
    const EMPTY_SNAPSHOT: [u8; 20] = [
        0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let ctx = Context::new(sandbox).await;
    let (service, synchronizer) = party_service(&ctx.endpoint).await;
    let party = PartyId::parse(&ctx.party("replication-missing").party).unwrap();
    let (first, second) = EMPTY_SNAPSHOT.split_at(8);
    let mut raw = RawPartyClient::new(ctx.channel);
    let expected = within(
        &ctx.endpoint,
        raw.import_party_acs(iter([
            p::ImportPartyAcsRequest {
                acs_snapshot: first.to_vec(),
                synchronizer_id: Some(synchronizer.to_string()),
                contract_import_mode: Some(p::ContractImportMode::Validation as i32),
                workflow_id_prefix: Some("replication".into()),
                party_id: Some(party.to_string()),
                ..Default::default()
            },
            p::ImportPartyAcsRequest {
                acs_snapshot: second.to_vec(),
                ..Default::default()
            },
        ])),
    )
    .await
    .unwrap_err();
    assert_eq!(
        expected.get_details_error_info().unwrap().reason,
        "PARTY_ACS_IMPORT_MISSING_ONBOARDING_MAPPING"
    );

    let mut options = ImportPartyAcsOptions::new(synchronizer, ContractImportMode::Validation);
    options.workflow_id_prefix = Some(LedgerString::new("replication".into()).unwrap());
    options.party_id = Some(party);
    let chunks = iter([first.to_vec(), second.to_vec()]);
    let actual = within(&ctx.endpoint, service.import_party_acs(options, chunks))
        .await
        .unwrap_err();
    assert_error(actual, &expected);
}

async fn party_service(endpoint: &str) -> (PartyManagementClient, SynchronizerId) {
    let client = ok(
        endpoint,
        AdminApiClient::builder(endpoint)
            .with_retry_config(RetryConfig::no_retry())
            .connect(),
    )
    .await;
    let ParticipantStatus::Initialized(status) =
        ok(endpoint, client.participant_status().participant_status()).await
    else {
        panic!("expected initialized participant");
    };
    let [connected] = status.connected_synchronizers.as_slice() else {
        panic!("expected one connected synchronizer");
    };
    (
        client.party_management(),
        connected.physical_synchronizer_id.logical().clone(),
    )
}

async fn observe_activation(
    endpoint: &str,
    channel: Channel,
    begin: i64,
    party: &PartyId,
) -> ledger::TopologyTransaction {
    let mut updates = ok(
        endpoint,
        UpdateServiceClient::new(channel).get_updates(ledger::GetUpdatesRequest {
            begin_exclusive: begin,
            update_format: Some(ledger::UpdateFormat {
                include_topology_events: Some(ledger::TopologyFormat {
                    include_participant_authorization_events: Some(
                        ledger::ParticipantAuthorizationTopologyFormat {
                            parties: vec![party.to_string()],
                        },
                    ),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
    )
    .await
    .into_inner();
    ok(endpoint, async {
        while let Some(response) = updates.message().await? {
            if let Some(ledger::get_updates_response::Update::TopologyTransaction(tx)) =
                response.update
            {
                return Ok(tx);
            }
        }
        Err(tonic::Status::not_found("party activation"))
    })
    .await
}
