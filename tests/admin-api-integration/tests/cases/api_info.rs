//! API identification across the sandbox's gRPC endpoints.

use admin_api::v30::AdminApiClient;
use admin_api_types::ApiName;
use canton_base::retry::RetryConfig;
use pretty_assertions::assert_eq;

use crate::support::{Sandbox, ok, sandbox};

/// Checks all three node types identify their administration endpoints consistently.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn admin_endpoints_match_server(sandbox: &Sandbox) {
    let endpoints = sandbox.endpoints();
    for endpoint in [
        &endpoints.admin_api,
        &endpoints.sequencer_admin,
        &endpoints.mediator_admin,
    ] {
        assert_api_info(endpoint, ApiName::AdminApi).await;
    }
}

/// Checks a Ledger API endpoint is identified when supplied to the Admin API client.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn ledger_endpoint_matches_server(sandbox: &Sandbox) {
    assert_api_info(&sandbox.endpoints().ledger_api, ApiName::LedgerApi).await;
}

/// Checks a public sequencer endpoint is identified when supplied to the Admin API client.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn sequencer_public_endpoint_matches_server(sandbox: &Sandbox) {
    assert_api_info(
        &sandbox.endpoints().sequencer_public,
        ApiName::SequencerPublicApi,
    )
    .await;
}

/// Check the SDK reports the expected API for this endpoint.
async fn assert_api_info(endpoint: &str, expected: ApiName) {
    let client = ok(
        endpoint,
        AdminApiClient::builder(endpoint)
            .with_retry_config(RetryConfig::no_retry())
            .connect(),
    )
    .await;
    let actual = ok(endpoint, client.get_api_info()).await;
    assert_eq!(actual, expected, "SDK API name at {endpoint}");
}
