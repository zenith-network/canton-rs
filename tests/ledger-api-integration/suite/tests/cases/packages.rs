//! Package discovery, registration, and payloads.

use canton::types::PackageId;
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use pretty_assertions::assert_eq;

use crate::support::*;

/// Checks package listing, registration, and downloaded payloads against the fixture DAR.
#[tokio::test]
async fn packages_match_fixture_dar() {
    let ctx = Context::new().await;
    let package = PackageId::new(ledger_api_integration::PACKAGE_ID.to_owned()).unwrap();
    let mut service = ctx.client.package();
    let packages = ok(service.list_packages()).await;
    let raw = ok(
        p::package_service_client::PackageServiceClient::new(ctx.channel.clone())
            .list_packages(p::ListPackagesRequest {}),
    )
    .await
    .into_inner();
    assert_eq!(strings(packages.iter()), strings(raw.package_ids.iter()));
    assert!(packages.contains(&package));
    assert!(ok(service.package_registered(package.clone())).await);
    assert_eq!(
        ok(service.get_package(package)).await,
        ledger_api_integration::PACKAGE_PAYLOAD
    );
    assert!(!ok(service.package_registered(PackageId::new("0".repeat(64)).unwrap())).await);
}
