//! Explicit acceptance check: dpm, Java, protoc, and SDK 3.6 are required.
use ledger_api_proto::com::daml::ledger::api::v2::{self as p, admin as a};
use sandbox::{Sandbox, SandboxConfig, sandbox};
use std::{path::PathBuf, sync::OnceLock, time::Duration};

async fn parties(
    endpoint: &str,
) -> a::party_management_service_client::PartyManagementServiceClient<tonic::transport::Channel> {
    a::party_management_service_client::PartyManagementServiceClient::connect(endpoint.to_owned())
        .await
        .unwrap()
}

async fn allocate(endpoint: &str) -> String {
    parties(endpoint)
        .await
        .allocate_party(a::AllocatePartyRequest {
            party_id_hint: format!("sandbox-smoke-{}", uuid::Uuid::new_v4()),
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner()
        .party_details
        .unwrap()
        .party
}

async fn knows(endpoint: &str, party: String) -> bool {
    !parties(endpoint)
        .await
        .get_parties(a::GetPartiesRequest {
            parties: vec![party],
            ..Default::default()
        })
        .await
        .unwrap()
        .into_inner()
        .party_details
        .is_empty()
}

fn fixture() -> &'static (SandboxConfig, String) {
    static FIXTURE: OnceLock<(SandboxConfig, String)> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let build = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        // Compile a copy so Daml's .daml cache stays out of the source tree.
        let package = build.path().join("package");
        std::fs::create_dir_all(package.join("daml")).unwrap();
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        std::fs::copy(fixtures.join("daml.yaml"), package.join("daml.yaml")).unwrap();
        std::fs::copy(
            fixtures.join("daml/Smoke.daml"),
            package.join("daml/Smoke.daml"),
        )
        .unwrap();
        let dar = dpm_build::Config::default()
            .package_root(&package)
            .output(build.path().join("smoke.dar"))
            .disable_multi_package()
            .build()
            .unwrap()
            .output;
        let package_id = daml_lf::dar::DarFile::read_from(&dar)
            .unwrap()
            .main_dalf()
            .unwrap()
            .hash()
            .to_string();
        let bytes = std::fs::read(dar).unwrap();
        let config = SandboxConfig::builder()
            .sdk_version("3.6.2")
            .with_dar(bytes)
            .max_sandboxes(2)
            .idle_timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        (config, package_id)
    })
}

fn config() -> SandboxConfig {
    fixture().0.clone()
}

#[sandbox(shared, config = config)]
#[tokio::test]
#[ignore = "requires dpm, Java, and installed SDK 3.6.2; run --test live -- --ignored"]
async fn preload_sharing_group_isolation_and_fresh_state(sandbox: &Sandbox) {
    let artifacts = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("canton-rs-sandbox");
    assert!(sandbox.artifacts_dir().starts_with(artifacts));
    eprintln!("sandbox smoke logs: {}", sandbox.artifacts_dir().display());
    let package_id = fixture().1.clone();
    let (first, shared) = tokio::join!(Sandbox::shared(), Sandbox::shared());
    let first = first.unwrap();
    let shared = shared.unwrap();
    assert_eq!(first.id(), shared.id());
    assert_eq!(first.id(), sandbox.id());
    let channel = tonic::transport::Endpoint::from_shared(first.ledger_api_endpoint().to_owned())
        .unwrap()
        .connect()
        .await
        .unwrap();
    let status = p::package_service_client::PackageServiceClient::new(channel)
        .get_package_status(p::GetPackageStatusRequest { package_id })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(status.package_status, p::PackageStatus::Registered as i32);
    let party = allocate(first.ledger_api_endpoint()).await;
    assert!(knows(shared.ledger_api_endpoint(), party.clone()).await);
    let group = Sandbox::shared_in_group("other").await.unwrap();
    assert_ne!(group.id(), first.id());
    assert!(!knows(group.ledger_api_endpoint(), party.clone()).await);
    group.release().await.unwrap();
    first.release().await.unwrap();
    shared.release().await.unwrap();

    let first = Sandbox::fresh().await.unwrap();
    let old_endpoint = first.ledger_api_endpoint().to_owned();
    let old_id = first.id().to_owned();
    let party = allocate(&old_endpoint).await;
    first.release().await.unwrap();
    assert!(
        tonic::transport::Endpoint::from_shared(old_endpoint)
            .unwrap()
            .connect()
            .await
            .is_err()
    );
    let next = Sandbox::fresh().await.unwrap();
    assert_ne!(next.id(), old_id);
    assert!(!knows(next.ledger_api_endpoint(), party).await);
    next.release().await.unwrap();
}
