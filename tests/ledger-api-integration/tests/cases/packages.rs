//! Package discovery, registration, and payloads.

use std::io::Cursor;

use canton::types::PackageId;
use daml_lf::dar::DarFile;
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use pretty_assertions::assert_eq;

use crate::support::*;

/// Checks package listing, registration, and downloaded payloads against the fixture DAR.
#[sandbox(shared, config = sandbox_config)]
#[tokio::test]
async fn packages_match_fixture_dar(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let package = ledger_api_integration::PACKAGE_ID;
    let main_dalf = DarFile::read(Cursor::new(ledger_api_integration::DAR_BYTES))
        .expect("read fixture DAR")
        .main_dalf()
        .expect("read main fixture DALF");
    assert_eq!(main_dalf.hash().to_package_id(), package);
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
    let payload = ok(service.get_package(package)).await;
    main_dalf
        .hash()
        .validate(&payload)
        .expect("GetPackage payload must match the fixture DALF hash");
    assert!(!ok(service.package_registered(PackageId::new("0".repeat(64)).unwrap())).await);
}
