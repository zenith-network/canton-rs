//! Package and DAR metadata, validation, uploads, vetting, and real server errors.

use std::{collections::BTreeSet, io::Cursor};

use admin_api::v30::{
    AdminApiClient,
    services::{PackageClient, package::UploadDarData},
};
use admin_api_proto::com::digitalasset::canton::admin::participant::v30::{
    self as p, package_service_client::PackageServiceClient as RawClient,
};
use canton_base::retry::RetryConfig;
use canton_proto::com::digitalasset::canton::topology::admin::v30 as t;
use canton_types::{
    PackageId,
    topology::{PhysicalSynchronizerId, SynchronizerId},
};
use daml_lf::dar::DarFile;
use pretty_assertions::assert_eq;
use tonic::transport::{Channel, Endpoint};

use crate::support::{
    Sandbox, ok, sandbox,
    topology::{RawRead, assert_error, head, timestamp},
    within,
};

const PACKAGE_FIXTURE: Fixture = Fixture {
    bytes: include_bytes!("../../fixtures/package-service.dar"),
    name: "package-service",
    version: "0.1.0",
};
const BATCH_FIXTURE: Fixture = Fixture {
    bytes: include_bytes!("../../fixtures/package-service-batch.dar"),
    name: "package-service-batch",
    version: "0.1.0",
};

/// Raw uploads seed independent listing checks, including every metadata field and both filters.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn packages_and_dars_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    for fixture in [&PACKAGE_FIXTURE, &BATCH_FIXTURE] {
        let response = ok(
            &ctx.endpoint,
            ctx.raw().upload_dar(p::UploadDarRequest {
                dars: vec![p::upload_dar_request::UploadDarData {
                    bytes: fixture.bytes.to_vec(),
                    description: Some(fixture.name.into()),
                    expected_main_package_id: Some(fixture.id().to_string()),
                }],
                vet_all_packages: false,
                synchronize_vetting: false,
                synchronizer_id: None,
            }),
        )
        .await
        .into_inner();
        assert_eq!(response.dar_ids, vec![fixture.id().to_string()]);
    }

    for (limit, filter) in [
        (None, None),
        (Some(1), None),
        (None, Some("")),
        (None, Some("package-service")),
        (None, Some(BATCH_FIXTURE.name)),
        (None, Some("integration-absent")),
        (Some(1), Some("package-service")),
        (Some(1), Some("integration-absent")),
    ] {
        let mut expected = ok(
            &ctx.endpoint,
            ctx.raw().list_packages(p::ListPackagesRequest {
                limit: limit.unwrap_or_default(),
                filter_name: filter.unwrap_or_default().into(),
            }),
        )
        .await
        .into_inner()
        .package_descriptions;
        let mut actual = ok(
            &ctx.endpoint,
            ctx.service.list_packages(
                limit.map(|value| value.try_into().unwrap()),
                filter.map(Into::into),
            ),
        )
        .await;
        expected.sort_by(|a, b| a.package_id.cmp(&b.package_id));
        actual.sort_by(|a, b| a.package_id.cmp(&b.package_id));
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(&expected) {
            assert_eq!(actual.package_id.as_str(), expected.package_id);
            assert_eq!(actual.name.as_str(), expected.name);
            assert_eq!(actual.version.to_string(), expected.version);
            assert_eq!(
                actual.uploaded_at,
                timestamp(expected.uploaded_at.as_ref().unwrap())
            );
            assert_eq!(actual.size, expected.size);
        }

        let mut expected = ok(
            &ctx.endpoint,
            ctx.raw().list_dars(p::ListDarsRequest {
                limit: limit.unwrap_or_default(),
                filter_name: filter.unwrap_or_default().into(),
            }),
        )
        .await
        .into_inner()
        .dars;
        let mut actual = ok(
            &ctx.endpoint,
            ctx.service.list_dars(
                limit.map(|value| value.try_into().unwrap()),
                filter.map(Into::into),
            ),
        )
        .await;
        expected.sort_by(|a, b| a.main.cmp(&b.main));
        actual.sort_by(|a, b| a.main.cmp(&b.main));
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(&expected) {
            assert_eq!(actual.main.as_str(), expected.main);
            assert_eq!(actual.name.as_str(), expected.name);
            assert_eq!(actual.version.to_string(), expected.version);
            assert_eq!(actual.description, expected.description);
        }
        if limit.is_none() && filter == Some("package-service") {
            assert_eq!(actual.len(), 2);
        }
    }

    for fixture in [&PACKAGE_FIXTURE, &BATCH_FIXTURE] {
        ctx.assert_stored(fixture, Some(fixture.name)).await;
    }
}

/// Validation returns the main package without persisting packages, DARs, or vetting changes.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn validate_dar_leaves_participant_state_unchanged(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let synchronizer = ctx.synchronizer().await;
    let packages = ctx.packages().await;
    let dars = ctx.dars().await;
    let vetted = ctx.vetted(&synchronizer).await;
    assert!(
        packages
            .iter()
            .all(|package| package.package_id != PACKAGE_FIXTURE.id().as_str())
    );

    for synchronizer_id in [None, Some(synchronizer.clone())] {
        let filename = "package-service.dar";
        let expected = ok(
            &ctx.endpoint,
            ctx.raw().validate_dar(p::ValidateDarRequest {
                data: PACKAGE_FIXTURE.bytes.to_vec(),
                filename: filename.into(),
                synchronizer_id: synchronizer_id.as_ref().map(ToString::to_string),
            }),
        )
        .await
        .into_inner();
        let actual = ok(
            &ctx.endpoint,
            ctx.service.validate_dar(
                PACKAGE_FIXTURE.bytes.to_vec(),
                filename.into(),
                synchronizer_id,
            ),
        )
        .await;
        assert_eq!(actual, PACKAGE_FIXTURE.id());
        assert_eq!(actual.as_str(), expected.main_package_id);
        assert_eq!(ctx.packages().await, packages);
        assert_eq!(ctx.dars().await, dars);
        assert_eq!(ctx.vetted(&synchronizer).await, vetted);
    }
}

/// A single DAR is stored without vetting, then can be uploaded again with synchronized vetting.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn upload_dar_stores_metadata_and_controls_vetting(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let synchronizer = ctx.synchronizer().await;
    let vetted = ctx.vetted(&synchronizer).await;
    let mut service = ctx.service.clone();
    service.set_retry_config(RetryConfig::no_retry());
    let id = ok(
        &ctx.endpoint,
        service.upload_dar(
            UploadDarData::new(PACKAGE_FIXTURE.bytes.to_vec())
                .with_description("single DAR upload".into())
                .with_expected_main_package_id(PACKAGE_FIXTURE.id()),
            false,
            false,
            Some(synchronizer.clone()),
        ),
    )
    .await;
    assert_eq!(id, PACKAGE_FIXTURE.id());
    ctx.assert_stored(&PACKAGE_FIXTURE, Some("single DAR upload"))
        .await;
    assert_eq!(ctx.vetted(&synchronizer).await, vetted);
    assert!(!vetted.contains(id.as_str()));

    let repeated = ok(
        &ctx.endpoint,
        service.upload_dar(PACKAGE_FIXTURE.bytes.to_vec(), true, true, None),
    )
    .await;
    assert_eq!(repeated, id);
    assert!(
        PACKAGE_FIXTURE
            .package_ids()
            .is_subset(&ctx.vetted(&synchronizer).await)
    );
    assert_eq!(
        ctx.dars()
            .await
            .iter()
            .filter(|dar| dar.main == id.as_str())
            .count(),
        1
    );
}

/// Multiple DARs preserve request order and per-DAR metadata, with an explicit vetting synchronizer.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn upload_dars_preserves_entries_and_synchronizes_vetting(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let synchronizer = ctx.synchronizer().await;
    let ids = ok(
        &ctx.endpoint,
        ctx.service.upload_dars(
            vec![
                UploadDarData::new(BATCH_FIXTURE.bytes.to_vec())
                    .with_expected_main_package_id(BATCH_FIXTURE.id()),
                UploadDarData::new(PACKAGE_FIXTURE.bytes.to_vec())
                    .with_description("batch upload".into()),
            ],
            true,
            true,
            Some(synchronizer.clone()),
        ),
    )
    .await;
    assert_eq!(ids, vec![BATCH_FIXTURE.id(), PACKAGE_FIXTURE.id()]);
    ctx.assert_stored(&PACKAGE_FIXTURE, Some("batch upload"))
        .await;
    ctx.assert_stored(&BATCH_FIXTURE, None).await;
    let vetted = ctx.vetted(&synchronizer).await;
    assert!(PACKAGE_FIXTURE.package_ids().is_subset(&vetted));
    assert!(BATCH_FIXTURE.package_ids().is_subset(&vetted));

    let repeated = ok(
        &ctx.endpoint,
        ctx.service.upload_dars(
            vec![PACKAGE_FIXTURE.bytes.to_vec(), BATCH_FIXTURE.bytes.to_vec()],
            false,
            false,
            None,
        ),
    )
    .await;
    assert_eq!(repeated, vec![PACKAGE_FIXTURE.id(), BATCH_FIXTURE.id()]);
}

/// Invalid archives, expected package mismatches, and absent synchronizers retain server errors.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn package_errors_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let synchronizer = ctx.synchronizer().await;
    let absent = SynchronizerId::new(
        "integration-absent".into(),
        synchronizer.namespace().clone(),
    )
    .unwrap();
    assert_ne!(absent, synchronizer);
    let packages = ctx.packages().await;
    let dars = ctx.dars().await;
    let vetted = ctx.vetted(&synchronizer).await;

    for (data, synchronizer_id) in [
        (b"not a DAR".to_vec(), None),
        (PACKAGE_FIXTURE.bytes.to_vec(), Some(absent.clone())),
    ] {
        let expected = within(
            &ctx.endpoint,
            ctx.raw().validate_dar(p::ValidateDarRequest {
                data: data.clone(),
                filename: "invalid.dar".into(),
                synchronizer_id: synchronizer_id.as_ref().map(ToString::to_string),
            }),
        )
        .await
        .unwrap_err();
        let actual = within(
            &ctx.endpoint,
            ctx.service
                .validate_dar(data, "invalid.dar".into(), synchronizer_id),
        )
        .await
        .unwrap_err();
        assert_error(actual, &expected);
    }

    for (bytes, expected_id, synchronizer_id) in [
        (b"not a DAR".to_vec(), None, None),
        (
            PACKAGE_FIXTURE.bytes.to_vec(),
            Some(PackageId::new("0".repeat(64)).unwrap()),
            None,
        ),
        (PACKAGE_FIXTURE.bytes.to_vec(), None, Some(absent)),
    ] {
        let expected = within(
            &ctx.endpoint,
            ctx.raw().upload_dar(p::UploadDarRequest {
                dars: vec![p::upload_dar_request::UploadDarData {
                    bytes: bytes.clone(),
                    description: Some("invalid upload".into()),
                    expected_main_package_id: expected_id.as_ref().map(ToString::to_string),
                }],
                vet_all_packages: true,
                synchronize_vetting: true,
                synchronizer_id: synchronizer_id.as_ref().map(ToString::to_string),
            }),
        )
        .await
        .unwrap_err();
        let mut dar = UploadDarData::new(bytes).with_description("invalid upload".into());
        if let Some(expected_id) = expected_id {
            dar = dar.with_expected_main_package_id(expected_id);
        }
        let actual = within(
            &ctx.endpoint,
            ctx.service.upload_dar(dar, true, true, synchronizer_id),
        )
        .await
        .unwrap_err();
        assert_error(actual, &expected);
    }
    assert_eq!(ctx.packages().await, packages);
    assert_eq!(ctx.dars().await, dars);
    assert_eq!(ctx.vetted(&synchronizer).await, vetted);
}

struct Fixture {
    bytes: &'static [u8],
    name: &'static str,
    version: &'static str,
}

impl Fixture {
    fn id(&self) -> PackageId {
        DarFile::read(Cursor::new(self.bytes))
            .unwrap()
            .main_dalf()
            .unwrap()
            .hash()
            .to_package_id()
    }

    fn package_ids(&self) -> BTreeSet<String> {
        DarFile::read(Cursor::new(self.bytes))
            .unwrap()
            .dalfs()
            .unwrap()
            .into_iter()
            .map(|dalf| dalf.hash().to_string())
            .collect()
    }
}

/// SDK and independently constructed protobuf requests on the same participant.
struct Context {
    endpoint: String,
    channel: Channel,
    service: PackageClient,
}

impl Context {
    async fn new(sandbox: &Sandbox) -> Self {
        let endpoint = sandbox.endpoints().admin_api.clone();
        let client = ok(
            &endpoint,
            AdminApiClient::builder(&endpoint)
                .with_retry_config(RetryConfig::no_retry())
                .connect(),
        )
        .await;
        let channel = ok(
            &endpoint,
            Endpoint::from_shared(endpoint.clone()).unwrap().connect(),
        )
        .await;
        Self {
            endpoint,
            channel,
            service: client.package(),
        }
    }

    fn raw(&self) -> RawClient<Channel> {
        RawClient::new(self.channel.clone())
    }

    async fn packages(&self) -> Vec<p::PackageDescription> {
        let mut packages = ok(
            &self.endpoint,
            self.raw().list_packages(p::ListPackagesRequest::default()),
        )
        .await
        .into_inner()
        .package_descriptions;
        packages.sort_by(|a, b| a.package_id.cmp(&b.package_id));
        packages
    }

    async fn dars(&self) -> Vec<p::DarDescription> {
        let mut dars = ok(
            &self.endpoint,
            self.raw().list_dars(p::ListDarsRequest::default()),
        )
        .await
        .into_inner()
        .dars;
        dars.sort_by(|a, b| a.main.cmp(&b.main));
        dars
    }

    async fn synchronizer(&self) -> SynchronizerId {
        let response = ok(
            &self.endpoint,
            p::participant_status_service_client::ParticipantStatusServiceClient::new(
                self.channel.clone(),
            )
            .participant_status(p::ParticipantStatusRequest::default()),
        )
        .await
        .into_inner();
        let Some(p::participant_status_response::Kind::Status(status)) = response.kind else {
            panic!("expected initialized participant");
        };
        assert_eq!(status.connected_synchronizers.len(), 1);
        PhysicalSynchronizerId::parse(&status.connected_synchronizers[0].physical_synchronizer_id)
            .unwrap()
            .logical()
            .clone()
    }

    async fn vetted(&self, synchronizer: &SynchronizerId) -> BTreeSet<String> {
        let results = ok(
            &self.endpoint,
            RawRead::new(self.channel.clone()).list_vetted_packages(t::ListVettedPackagesRequest {
                base_query: Some(head(Some(t::StoreId {
                    store: Some(t::store_id::Store::Synchronizer(t::Synchronizer {
                        kind: Some(t::synchronizer::Kind::Id(synchronizer.to_string())),
                    })),
                }))),
                filter_participant: String::new(),
            }),
        )
        .await
        .into_inner()
        .results;
        results
            .into_iter()
            .flat_map(|result| {
                let mapping = result.item.expect("vetted packages mapping");
                mapping
                    .packages
                    .into_iter()
                    .map(|package| package.package_id)
            })
            .collect()
    }

    async fn assert_stored(&self, fixture: &Fixture, description: Option<&str>) {
        let packages = self.packages().await;
        let package = packages
            .iter()
            .find(|package| package.package_id == fixture.id().as_str())
            .expect("uploaded main package");
        assert_eq!(package.name, fixture.name);
        assert_eq!(package.version, fixture.version);
        assert!(package.size > 0);
        assert!(package.uploaded_at.is_some());

        let dars = self.dars().await;
        let dar = dars
            .iter()
            .find(|dar| dar.main == fixture.id().as_str())
            .expect("uploaded DAR");
        assert_eq!(dar.name, fixture.name);
        assert_eq!(dar.version, fixture.version);
        if let Some(description) = description {
            assert_eq!(dar.description, description);
        } else {
            assert!(!dar.description.is_empty());
        }
        let downloaded = ok(
            &self.endpoint,
            self.raw().get_dar(p::GetDarRequest {
                main_package_id: fixture.id().to_string(),
            }),
        )
        .await
        .into_inner();
        assert_eq!(downloaded.payload, fixture.bytes);
        assert_eq!(downloaded.data.as_ref(), Some(dar));
    }
}
