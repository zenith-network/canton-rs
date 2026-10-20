use std::num::NonZeroU32;

use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, GetPackageRequest, GetPackageStatusRequest, ListPackagesRequest,
    ListPackagesResponse, ListVettedPackagesRequest, ListVettedPackagesResponse,
    package_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::PackageId,
    v2::{PackageMetadataFilter, Page, PageStringToken, TopologyStateFilter, VettedPackages},
    value::v2::errors::IntoValueError as _,
};
use protobuf_utils::InvalidProtoField as _;

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::PackageServiceClient`]
#[derive(Clone, Debug)]
pub struct PackageServiceClient {
    service: svc_proto::PackageServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
    max_vetted_packages_page_size: Option<NonZeroU32>,
}

impl PackageServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::PackageServiceClient<InterceptedService>,
        retry_handler: RetryHandler,
        max_vetted_packages_page_size: Option<NonZeroU32>,
    ) -> Self {
        Self {
            service,
            retry_handler,
            max_vetted_packages_page_size,
        }
    }

    /// Set retry config for the client
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// Returns the contents of a single package
    pub async fn get_package(&mut self, package_id: PackageId) -> Result<Vec<u8>, CantonError> {
        let request = GetPackageRequest {
            package_id: package_id.into(),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_package(req).await
            })
            .await?;

        Ok(response.archive_payload)
    }

    /// Returns `true` if package status is `Registered`
    pub async fn package_registered(&mut self, package_id: PackageId) -> Result<bool, CantonError> {
        let request = GetPackageStatusRequest {
            package_id: package_id.into(),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_package_status(req).await
            })
            .await?;

        // FIXME: replace unwrap with error
        let status = proto::PackageStatus::try_from(response.package_status).unwrap();

        Ok(status == proto::PackageStatus::Registered)
    }

    /// Returns the identifiers of all supported packages.
    ///
    /// Note: Packages can be uploaded to participant without vetting. Not every package listed here
    /// can participate in transactions on this participant. Use
    /// [`PackageServiceClient::list_vetted_packages()`] to get the list of vetted packages.
    pub async fn list_packages(&mut self) -> Result<Vec<PackageId>, CantonError> {
        let response = self
            .retry_handler
            .call(&self.service, &(), |mut svc, _| async move {
                svc.list_packages(ListPackagesRequest {}).await
            })
            .await?;

        response
            .package_ids
            .into_iter()
            .enumerate()
            .map(|(idx, package_id)| {
                PackageId::new(package_id)
                    .validated_of::<ListPackagesResponse>("package_ids")
                    .with_msg_owned(format!("failed to convert package_ids[{idx}]"))
                    .map_err(CantonError::value_error)
            })
            .collect::<Result<_, _>>()
    }

    /// Lists which participant node vetted what packages on which synchronizer.
    ///
    /// Can be called by any authenticated user.
    ///
    /// ## Arguments
    ///
    /// - `package_metadata_filter` - Package metadata filter the returned vetted packages set must
    ///   satisfy.
    /// - `topology_state_filter` - Topology filter the returned vetted packages set must satisfy.
    /// - `page_token`- Pagination token to determine the specific page to fetch. Using the token
    ///   guarantees that [`VettedPackages`] on a subsequent page are all greater
    ///   ([`VettedPackages`] are sorted by synchronizer ID then participant ID) than the last
    ///   [`VettedPackages`] on a previous page.
    ///
    ///   The server does not store intermediate results between calls chained by a series of page
    ///   tokens. As a consequence, if new vetted packages are being added and a page is requested
    ///   twice using the same token, more packages can be returned on the second call.
    ///
    ///   Use [`PageStringToken::first()`] (equals to `None`) to fetch the first page.
    /// - `page_size` - Maximum number of [`VettedPackages`] results to return in a single page.
    ///
    ///   If the `page_size` is unspecified, the server will decide the number of results to be
    ///   returned.
    ///
    ///   If the `page_size` exceeds the maximum supported by the server, an error will be returned.
    ///
    ///   To obtain the server's maximum consult the
    ///   [`PackageFeature`][ledger_api_types::v2::PackageFeature] descriptor available in the
    ///   [`VersionServiceClient`][crate::grpc::v2::services::VersionServiceClient].
    ///
    ///   `max_vetted_packages_page_size` is also accepted on the creation of this service and
    ///   propagated from
    ///   [`CantonClient::package()`][crate::grpc::v2::client::CantonClient::package()], if the
    ///   client resolved server capabilities before (see
    ///   [`CantonClient::resolve_server_capabilities()`][crate::grpc::v2::client::CantonClient::resolve_server_capabilities()]).
    pub async fn list_vetted_packages(
        &mut self,
        package_metadata_filter: PackageMetadataFilter,
        topology_state_filter: TopologyStateFilter,
        page_token: Option<PageStringToken>,
        page_size: Option<NonZeroU32>,
    ) -> Result<Page<PageStringToken, VettedPackages>, CantonError> {
        // If max page size is set, it must be respected
        let page_size = page_size.map(|size| {
            self.max_vetted_packages_page_size
                .map_or(size, |max_size| size.min(max_size))
        });

        let request = ListVettedPackagesRequest {
            // Tiny optimization: encode filters to `None` to omit proto field completely.
            // On participant it's processes the same way, if we would set it to Some(default).
            package_metadata_filter: (!package_metadata_filter.is_any())
                .then(|| package_metadata_filter.into()),
            topology_state_filter: (!topology_state_filter.is_any())
                .then(|| topology_state_filter.into()),
            page_token: PageStringToken::into_proto(page_token),
            page_size: page_size.map(Into::into).unwrap_or(0),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_vetted_packages(req).await
            })
            .await?;

        let items = response
            .vetted_packages
            .into_iter()
            .enumerate()
            .map(|(idx, package)| {
                package
                    .try_into()
                    .validated_of::<ListVettedPackagesResponse>("vetted_packages")
                    .with_msg_owned(format!("failed to convert vetted_packages[{idx}]"))
            })
            .collect::<Result<_, _>>()?;
        let next_page_token = PageStringToken::try_from_proto(response.next_page_token)
            .validated_of::<ListVettedPackagesResponse>("next_page_token")
            .no_msg()?;

        Ok(Page {
            items,
            next_page_token,
        })
    }
}
