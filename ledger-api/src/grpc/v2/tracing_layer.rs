use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use http::Request;
use tower::Service;

/// A tower [`Service`] wrapper that traces every gRPC call with method name,
/// elapsed time, and success/error status.
#[derive(Clone)]
pub struct GrpcTracing<S> {
    inner: S,
}

impl<S> GrpcTracing<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ReqBody> Service<Request<ReqBody>> for GrpcTracing<S>
where
    S: Service<Request<ReqBody>> + Clone,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: std::fmt::Display + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let method = req.uri().path().to_owned();
        let span = tracing::info_span!("grpc", method = %method);

        let start = std::time::Instant::now();
        let fut = self.inner.call(req);

        Box::pin(tracing::Instrument::instrument(
            async move {
                let result = fut.await;
                let elapsed_ms = start.elapsed().as_millis() as u64;
                match &result {
                    Ok(_) => tracing::info!(elapsed_ms, "ok"),
                    Err(e) => tracing::warn!(elapsed_ms, error = %e, "error"),
                }
                result
            },
            span,
        ))
    }
}
