use ledger_api_proto::com::daml::ledger::api::v2::interactive::{
    GetPreferredPackagesRequest, GetPreferredPackagesResponse,
    interactive_submission_service_client as svc_proto,
};

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::InteractiveSubmissionServiceClient`]
#[derive(Clone, Debug)]
pub struct InteractiveSubmissionServiceClient {
    service: svc_proto::InteractiveSubmissionServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl InteractiveSubmissionServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::InteractiveSubmissionServiceClient<InterceptedService>,
        retry_handler: RetryHandler,
    ) -> Self {
        Self {
            service,
            retry_handler,
        }
    }

    /// Set retry config for the client
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// Compute the preferred packages for the vetting requirements in the request.
    ///
    /// A preferred package is the highest-versioned package of a package name that is vetted
    /// by all the participants hosting the requirement's parties. If no selection satisfies
    /// the requirements, a `FAILED_PRECONDITION` error (`NO_PREFERRED_PACKAGES_FOUND`) is
    /// returned.
    ///
    /// The endpoint is experimental in Canton: it is not guaranteed to stay backwards
    /// compatible.
    pub async fn get_preferred_packages(
        &mut self,
        request: GetPreferredPackagesRequest,
    ) -> Result<GetPreferredPackagesResponse, CantonError> {
        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_preferred_packages(req).await
            })
            .await
    }
}
