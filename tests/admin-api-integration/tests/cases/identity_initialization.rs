//! Node identities and current time across the sandbox's administration endpoints.

use std::time::SystemTime;

use admin_api::v30::AdminApiClient;
use canton_base::retry::RetryConfig;
use canton_proto::com::digitalasset::canton::topology::admin::v30::{
    self as p,
    identity_initialization_service_client::IdentityInitializationServiceClient as RawClient,
};
use pretty_assertions::assert_eq;
use tonic::transport::{Channel, Endpoint};

use crate::support::{Sandbox, ok, sandbox};

/// Checks all three node types return the same identity through the SDK and protobuf clients.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn get_id_matches_server(sandbox: &Sandbox) {
    let endpoints = sandbox.endpoints();
    for endpoint in [
        &endpoints.admin_api,
        &endpoints.sequencer_admin,
        &endpoints.mediator_admin,
    ] {
        let (client, mut raw) = clients(endpoint).await;
        let expected = ok(endpoint, raw.get_id(p::GetIdRequest::default()))
            .await
            .into_inner();
        assert!(
            expected.initialized,
            "{endpoint}: expected initialized identity"
        );

        let actual = ok(endpoint, client.identity_initialization().get_id())
            .await
            .expect("initialized node identity");
        assert_eq!(
            actual.to_string(),
            expected.unique_identifier,
            "node identity at {endpoint}"
        );
    }
}

/// Bounds SDK time by surrounding server responses without relying on the host's clock.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn current_time_matches_server(sandbox: &Sandbox) {
    let endpoints = sandbox.endpoints();
    for endpoint in [
        &endpoints.admin_api,
        &endpoints.sequencer_admin,
        &endpoints.mediator_admin,
    ] {
        let (client, mut raw) = clients(endpoint).await;
        let before = ok(endpoint, raw.current_time(p::CurrentTimeRequest::default()))
            .await
            .into_inner()
            .current_time;
        let actual = ok(endpoint, client.identity_initialization().current_time()).await;
        let after = ok(endpoint, raw.current_time(p::CurrentTimeRequest::default()))
            .await
            .into_inner()
            .current_time;

        let elapsed = actual
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("node time after Unix epoch");
        assert_eq!(
            elapsed.subsec_nanos() % 1_000,
            0,
            "microsecond precision at {endpoint}"
        );
        let micros = i64::try_from(elapsed.as_micros()).expect("node time in microseconds");
        assert!(
            before <= micros && micros <= after,
            "{endpoint}: SDK time {micros} must be between {before} and {after}"
        );
    }
}

/// Connect SDK and protobuf clients with the suite's deadlines and no SDK retries.
async fn clients(endpoint: &str) -> (AdminApiClient, RawClient<Channel>) {
    let client = ok(
        endpoint,
        AdminApiClient::builder(endpoint)
            .with_retry_config(RetryConfig::no_retry())
            .connect(),
    )
    .await;
    let channel = ok(
        endpoint,
        Endpoint::from_shared(endpoint.to_owned())
            .unwrap()
            .connect(),
    )
    .await;
    (client, RawClient::new(channel))
}
