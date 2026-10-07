//! Connections and version metadata.

use ledger_api::grpc::v2::{client::CantonClient, retry::RetryConfig};
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use pretty_assertions::assert_eq;

use crate::support::*;

/// Checks eager and lazy connections expose the same API version and feature fields as the server.
#[tokio::test]
async fn connection_and_version_match_real_participant() {
    let ctx = Context::new().await;
    let raw = ok(
        p::version_service_client::VersionServiceClient::new(ctx.channel.clone())
            .get_ledger_api_version(p::GetLedgerApiVersionRequest {}),
    )
    .await
    .into_inner();
    let lazy = CantonClient::builder(endpoint())
        .with_retry_config(RetryConfig::no_retry())
        .connect_lazy()
        .unwrap();
    for client in [&ctx.client, &lazy] {
        let version = ok(client.version().get_ledger_api_version()).await;
        assert_eq!(version.version, raw.version);
        let features = raw.features.as_ref().expect("server features");
        let users = features
            .user_management
            .as_ref()
            .expect("user-management features");
        assert_eq!(version.features.user_management.supported, users.supported);
        assert_eq!(
            version.features.user_management.max_rights_per_user,
            users.max_rights_per_user
        );
        assert_eq!(
            version.features.user_management.max_users_page_size,
            users.max_users_page_size
        );
        assert_eq!(
            version.features.party_management.max_parties_page_size,
            features
                .party_management
                .as_ref()
                .unwrap()
                .max_parties_page_size
        );
        assert_eq!(
            version
                .features
                .package_feature
                .max_vetted_packages_page_size,
            features
                .package_feature
                .as_ref()
                .unwrap()
                .max_vetted_packages_page_size
        );
        let raw_delay = features
            .offset_checkpoint
            .as_ref()
            .unwrap()
            .max_offset_checkpoint_emission_delay
            .clone()
            .unwrap();
        assert_eq!(
            version
                .features
                .offset_checkpoint
                .max_offset_checkpoint_emission_delay,
            std::time::Duration::try_from(raw_delay).unwrap()
        );
    }
}

/// Checks experimental feature flags map to their corresponding server fields.
#[tokio::test]
async fn experimental_version_features_match_real_participant() {
    let ctx = Context::new().await;
    let raw = ok(
        p::version_service_client::VersionServiceClient::new(ctx.channel.clone())
            .get_ledger_api_version(p::GetLedgerApiVersionRequest {}),
    )
    .await
    .into_inner();
    let version = ok(ctx.client.version().get_ledger_api_version()).await;
    let experimental = raw
        .features
        .as_ref()
        .unwrap()
        .experimental
        .as_ref()
        .unwrap();
    assert_eq!(
        version.features.experimental.static_time,
        experimental
            .static_time
            .as_ref()
            .map(|feature| feature.supported)
    );
    assert_eq!(
        version.features.experimental.command_inspection_service,
        experimental
            .command_inspection_service
            .as_ref()
            .map(|feature| feature.supported),
        "SDK feature values must come from their corresponding real server fields"
    );
}
