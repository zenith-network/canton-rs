use crate::retry::retry_config::RetryConfig;

#[derive(Clone, Debug)]
pub struct RetryHandler {
    config: RetryConfig,
}

impl RetryHandler {
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    pub async fn call<
        S: Clone,
        Req: Clone,
        Resp,
        F: Fn(S, Req) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, Status>>,
    >(
        &mut self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, CantonError> {
        let mut attempt = 0;

        loop {
            let result = func(service.clone(), request.clone())
                .await
                .map_err(CantonError::from);

            match result {
                Ok(response) => {
                    self.config.success();
                    return Ok(response.into_inner());
                }
                Err(error) => {
                    let Some(delay) = self.config.retry_after(&error, attempt) else {
                        return Err(error);
                    };

                    // We log only if the error is not returned, to avoid log duplication
                    #[cfg(feature = "tracing")]
                    tracing::trace!(attempt, ?error, "Client encountered error");

                    attempt += 1;
                    delay.await;
                }
            }
        }
    }

    /// Same as [`Self::call`], but the closure has access to attempt number
    pub async fn call_with_attempt<
        S: Clone,
        Req: Clone,
        Resp,
        F: Fn(S, Req, usize) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, Status>>,
    >(
        &mut self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, CantonError> {
        let mut attempt = 0;

        loop {
            let result = func(service.clone(), request.clone(), attempt)
                .await
                .map_err(CantonError::from);

            match result {
                Ok(response) => {
                    self.config.success();
                    return Ok(response.into_inner());
                }
                Err(error) => {
                    let Some(delay) = self.config.retry_after(&error, attempt) else {
                        return Err(error);
                    };

                    // We log only if the error is not returned, to avoid log duplication
                    #[cfg(feature = "tracing")]
                    tracing::trace!(attempt, ?error, "Client encountered error");

                    attempt += 1;
                    delay.await;
                }
            }
        }
    }
}
