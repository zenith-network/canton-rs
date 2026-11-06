//! Participant status and synchronizer filtering against real server responses.

use std::collections::{BTreeMap, BTreeSet};

use admin_api::v30::{
    AdminApiClient,
    services::participant_status::{InitializedParticipantStatus, ParticipantStatus},
};
use admin_api_proto::com::digitalasset::canton::{
    admin::{health::v30 as health, participant::v30 as p},
    topology::admin::v30::{Synchronizer as RawSynchronizer, synchronizer::Kind},
};
use admin_api_types::topology::v30::{
    Synchronizer, connected_synchronizer::Health, health::ComponentStatusVariants,
};
use canton_base::retry::RetryConfig;
use canton_types::topology::{PhysicalSynchronizerId, SynchronizerId};
use p::{
    participant_status_response::ParticipantStatusResponseStatus as RawStatus,
    participant_status_service_client::ParticipantStatusServiceClient as RawClient,
};
use pretty_assertions::assert_eq;
use tonic::transport::{Channel, Endpoint};

use crate::support::{Sandbox, ok, sandbox};

/// Checks the client preserves the participant's status fields.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn participant_status_matches_real_participant(sandbox: &Sandbox) {
    let endpoint = &sandbox.endpoints().admin_api;
    let (client, mut raw) = clients(sandbox).await;
    let before = raw_status(endpoint, &mut raw, None).await;
    let status = initialized(ok(endpoint, client.participant_status().participant_status()).await);
    let after = raw_status(endpoint, &mut raw, None).await;
    assert_status(&status, &before, &after);
    assert!(status.active);
    assert!(status.common_status.active);
    assert!(!status.connected_synchronizers.is_empty());
    assert!(!status.supported_protocol_versions.is_empty());
    assert!(!status.common_status.components.is_empty());
}

/// Checks requests for connected logical and physical IDs match the server's responses.
#[sandbox(canton_version = "^3.6.1", shared)]
#[tokio::test]
async fn participant_status_for_connected_synchronizer_matches_server(sandbox: &Sandbox) {
    let endpoint = &sandbox.endpoints().admin_api;
    let (client, mut raw) = clients(sandbox).await;
    let full = raw_status(endpoint, &mut raw, None).await;
    let physical = connected_synchronizer(&full);
    let mut service = client.participant_status();

    for (synchronizer, kind) in [
        (
            Synchronizer::Id(physical.logical().clone()),
            Kind::Id(physical.logical().to_string()),
        ),
        (
            Synchronizer::PhysicalId(physical.clone()),
            Kind::PhysicalId(physical.to_string()),
        ),
    ] {
        let before = raw_status(endpoint, &mut raw, Some(kind.clone())).await;
        let status = initialized(ok(endpoint, service.participant_status_for(synchronizer)).await);
        let after = raw_status(endpoint, &mut raw, Some(kind)).await;
        assert_status(&status, &before, &after);
    }
}

/// Checks requests for absent logical IDs and physical instances match the server's responses.
#[sandbox(canton_version = "^3.6.1", shared)]
#[tokio::test]
async fn participant_status_for_unconnected_synchronizer_matches_server(sandbox: &Sandbox) {
    let endpoint = &sandbox.endpoints().admin_api;
    let (client, mut raw) = clients(sandbox).await;
    let full = raw_status(endpoint, &mut raw, None).await;
    let mut service = client.participant_status();

    // Canton 3.6.2 ignores the filter; compare with the real response in either case.
    for (synchronizer, kind) in unconnected_synchronizers(&full) {
        let before = raw_status(endpoint, &mut raw, Some(kind.clone())).await;
        let status = initialized(ok(endpoint, service.participant_status_for(synchronizer)).await);
        let after = raw_status(endpoint, &mut raw, Some(kind)).await;
        assert_status(&status, &before, &after);
    }
}

/// Checks filtering keeps matching synchronizers and their components alongside node components.
#[sandbox(canton_version = "^3.6.1", shared)]
#[tokio::test]
#[ignore = "Canton 3.6.2 doesn't support participant status filtering yet"]
// In Canton 3.6.2 the filtering is stated in the protobuf, but actually not yet implemented.
// This test needs to be re-enabled in the future with an updated Canton version, when the actual
// implementation will be in place.
async fn participant_status_for_filters_synchronizer_data(sandbox: &Sandbox) {
    let endpoint = &sandbox.endpoints().admin_api;
    let (client, mut raw) = clients(sandbox).await;
    let full = raw_status(endpoint, &mut raw, None).await;
    let physical = connected_synchronizer(&full);
    let mut service = client.participant_status();
    let connected = [
        (
            Synchronizer::Id(physical.logical().clone()),
            Some(&physical),
        ),
        (Synchronizer::PhysicalId(physical.clone()), Some(&physical)),
    ];
    let unconnected =
        unconnected_synchronizers(&full).map(|(synchronizer, _)| (synchronizer, None));

    for (synchronizer, expected) in connected.into_iter().chain(unconnected) {
        let status = initialized(ok(endpoint, service.participant_status_for(synchronizer)).await);
        let connected: BTreeSet<_> = status
            .connected_synchronizers
            .iter()
            .map(|synchronizer| synchronizer.physical_synchronizer_id.to_string())
            .collect();
        assert_eq!(
            connected,
            expected.into_iter().map(ToString::to_string).collect()
        );
        assert_filtered_components(&status, &full, expected);
    }
}

/// Connect SDK and protobuf clients with the suite's deadlines and no SDK retries.
async fn clients(sandbox: &Sandbox) -> (AdminApiClient, RawClient<Channel>) {
    let endpoint = &sandbox.endpoints().admin_api;
    let client = ok(
        endpoint,
        AdminApiClient::builder(endpoint)
            .with_retry_config(RetryConfig::no_retry())
            .connect(),
    )
    .await;
    let channel = ok(
        endpoint,
        Endpoint::from_shared(endpoint.clone()).unwrap().connect(),
    )
    .await;
    (client, RawClient::new(channel))
}

/// Require the initialized variant provided by the sandbox harness.
fn initialized(status: ParticipantStatus) -> InitializedParticipantStatus {
    match status {
        ParticipantStatus::Initialized(status) => status,
        _ => panic!("expected initialized participant: {status:#?}"),
    }
}

/// Make a protobuf request independently of the SDK's request conversions.
async fn raw_status(endpoint: &str, raw: &mut RawClient<Channel>, kind: Option<Kind>) -> RawStatus {
    let response = ok(
        endpoint,
        raw.participant_status(p::ParticipantStatusRequest {
            synchronizer_id: kind.map(|kind| RawSynchronizer { kind: Some(kind) }),
        }),
    )
    .await
    .into_inner();
    match response.kind {
        Some(p::participant_status_response::Kind::Status(status)) => status,
        _ => panic!("expected initialized protobuf status: {response:#?}"),
    }
}

/// Read a physical ID from the real participant rather than hard-coding its namespace.
fn connected_synchronizer(status: &RawStatus) -> PhysicalSynchronizerId {
    let connected = status
        .connected_synchronizers
        .first()
        .expect("connected synchronizer");
    PhysicalSynchronizerId::parse(&connected.physical_synchronizer_id).unwrap()
}

/// Construct valid but unconnected logical and physical IDs using the sandbox's namespace.
fn unconnected_synchronizers(status: &RawStatus) -> [(Synchronizer, Kind); 2] {
    let physical = connected_synchronizer(status);
    let unknown =
        SynchronizerId::new("integration-unknown".into(), physical.namespace().clone()).unwrap();
    assert_ne!(unknown, *physical.logical());
    let absent_instance = physical.increment_serial().unwrap();
    assert!(
        status
            .connected_synchronizers
            .iter()
            .all(|synchronizer| absent_instance != synchronizer.physical_synchronizer_id)
    );
    [
        (
            Synchronizer::Id(unknown.clone()),
            Kind::Id(unknown.to_string()),
        ),
        (
            Synchronizer::PhysicalId(absent_instance.clone()),
            Kind::PhysicalId(absent_instance.to_string()),
        ),
    ]
}

/// Compare all exposed fields, bounding uptime by the surrounding server responses.
fn assert_status(status: &InitializedParticipantStatus, before: &RawStatus, after: &RawStatus) {
    let common = &status.common_status;
    let raw = before.common_status.as_ref().expect("common status");
    assert_eq!(common.uid.to_string(), raw.uid);
    assert_eq!(common.version.to_string(), raw.version);
    assert_eq!(common.ports, raw.ports);
    assert_eq!(common.active, raw.active);
    assert_eq!(status.active, before.active);

    let uptime = common.uptime.expect("participant uptime");
    let uptime = (uptime.num_seconds(), uptime.subsec_nanos());
    let lower = raw.uptime.as_ref().expect("protobuf uptime");
    let upper = after
        .common_status
        .as_ref()
        .unwrap()
        .uptime
        .as_ref()
        .unwrap();
    assert!(
        (lower.seconds, lower.nanos) <= uptime && uptime <= (upper.seconds, upper.nanos),
        "SDK uptime {uptime:?} must be between {lower:?} and {upper:?}"
    );

    let mut versions: Vec<_> = status
        .supported_protocol_versions
        .iter()
        .map(|v| i32::from(*v))
        .collect();
    let mut raw_versions = before.supported_protocol_versions.clone();
    versions.sort_unstable();
    raw_versions.sort_unstable();
    assert_eq!(versions, raw_versions);

    let connected: BTreeMap<_, _> = status
        .connected_synchronizers
        .iter()
        .map(|synchronizer| {
            let health = match synchronizer.health {
                Health::Healthy => p::connected_synchronizer::Health::Healthy,
                Health::Unhealthy => p::connected_synchronizer::Health::Unhealthy,
                _ => panic!("unexpected synchronizer health: {:?}", synchronizer.health),
            };
            (
                synchronizer.physical_synchronizer_id.to_string(),
                health as i32,
            )
        })
        .collect();
    let raw_connected: BTreeMap<_, _> = before
        .connected_synchronizers
        .iter()
        .map(|synchronizer| {
            (
                synchronizer.physical_synchronizer_id.clone(),
                synchronizer.health,
            )
        })
        .collect();
    assert_eq!(connected, raw_connected);

    let mut components: Vec<_> = common.components.iter().collect();
    let mut raw_components: Vec<_> = raw.components.iter().collect();
    components.sort_unstable_by(|a, b| (&a.name, &a.labels).cmp(&(&b.name, &b.labels)));
    raw_components.sort_unstable_by(|a, b| (&a.name, &a.labels).cmp(&(&b.name, &b.labels)));
    assert_eq!(components.len(), raw_components.len());
    for (component, raw) in components.into_iter().zip(raw_components) {
        assert_eq!(component.name, raw.name);
        assert_eq!(component.labels, raw.labels);
        match (
            &component.status,
            raw.status.as_ref().expect("component state"),
        ) {
            (ComponentStatusVariants::Ok(data), health::component_status::Status::Ok(raw))
            | (
                ComponentStatusVariants::Degraded(data),
                health::component_status::Status::Degraded(raw),
            )
            | (
                ComponentStatusVariants::Failed(data),
                health::component_status::Status::Failed(raw),
            )
            | (
                ComponentStatusVariants::Fatal(data),
                health::component_status::Status::Fatal(raw),
            ) => {
                assert_eq!(data.description, raw.description);
            }
            _ => panic!("component state differs: {component:#?}, {raw:#?}"),
        }
    }
}

/// Filtering must preserve node components and only the matching synchronizer's components.
fn assert_filtered_components(
    status: &InitializedParticipantStatus,
    full: &RawStatus,
    physical: Option<&PhysicalSynchronizerId>,
) {
    let components: BTreeSet<_> = status
        .common_status
        .components
        .iter()
        .map(|component| (&component.name, &component.labels))
        .collect();
    let physical = physical.map(ToString::to_string);
    let expected: BTreeSet<_> = full
        .common_status
        .as_ref()
        .unwrap()
        .components
        .iter()
        .filter(|component| {
            component
                .labels
                .get("synchronizer")
                .is_none_or(|label| Some(label) == physical.as_ref())
        })
        .map(|component| (&component.name, &component.labels))
        .collect();
    assert!(!expected.is_empty(), "node-level components");
    assert_eq!(components, expected);
}
