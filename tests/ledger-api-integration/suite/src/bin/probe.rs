use ledger_api_proto::com::daml::ledger::api::v2::{
    GetLedgerApiVersionRequest, GetPackageStatusRequest, PackageStatus,
    package_service_client::PackageServiceClient, version_service_client::VersionServiceClient,
};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let endpoint = std::env::var("CANTON_TEST_ENDPOINT").expect("CANTON_TEST_ENDPOINT");
        let channel = tonic::transport::Endpoint::from_shared(endpoint)?
            .connect()
            .await?;
        VersionServiceClient::new(channel.clone())
            .get_ledger_api_version(GetLedgerApiVersionRequest {})
            .await?;
        let status = PackageServiceClient::new(channel)
            .get_package_status(GetPackageStatusRequest {
                package_id: ledger_api_integration::PACKAGE_ID.into(),
            })
            .await?
            .into_inner();
        if status.package_status != PackageStatus::Registered as i32 {
            return Err("fixture package is not registered".into());
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;
    match result {
        Ok(Ok(())) => (),
        other => {
            eprintln!("Ledger API readiness probe failed: {other:?}");
            std::process::exit(1);
        }
    }
}
