use tonic::Response;

use crate::retry::{RetryConfig, RetryableError};

/// Base retry handler
#[derive(Clone, Debug)]
pub struct RetryHandler {
    config: RetryConfig,
}

impl RetryHandler {
    /// Create handler with given config
    pub fn from_config(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Reference to underlying configuration
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Call with retry
    pub async fn call<E, S, Req, Resp, F, Fut>(
        &self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, E>
    where
        E: RetryableError,
        S: Clone,
        Req: Clone,
        F: Fn(S, Req) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, E>>,
    {
        RetryExecutor::new(self.config.clone())
            .call(service, request, func)
            .await
    }

    /// Same as [`Self::call`], but the closure has access to attempt number
    pub async fn call_with_attempt<E, S, Req, Resp, F, Fut>(
        &self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, E>
    where
        E: RetryableError,
        S: Clone,
        Req: Clone,
        F: Fn(S, Req, usize) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, E>>,
    {
        RetryExecutor::new(self.config.clone())
            .call_with_attempt(service, request, func)
            .await
    }
}

/// Retry handler context
#[derive(Clone, Debug, Default)]
pub struct RetryCtx {
    attempt: usize,
}

impl RetryCtx {
    /// New fresh retry context
    pub fn new() -> Self {
        Self { attempt: 0 }
    }

    /// Number of the current attempt
    pub fn attempt(&self) -> usize {
        self.attempt
    }
}

/// Executor of a single call
struct RetryExecutor {
    /// Owning the config here allows to share between calls only the values which are intended
    /// to be shared (e.g. tps budget). At the same time things like exponential backoff are kept
    /// isolated per call.
    config: RetryConfig,
    ctx: RetryCtx,
}

impl RetryExecutor {
    fn new(config: RetryConfig) -> Self {
        Self {
            config,
            ctx: RetryCtx::new(),
        }
    }

    async fn call<E, S, Req, Resp, F, Fut>(
        &mut self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, E>
    where
        E: RetryableError,
        S: Clone,
        Req: Clone,
        F: Fn(S, Req) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, E>>,
    {
        loop {
            let result = func(service.clone(), request.clone()).await;

            if let Some(resp) = self.handle_result(result).await? {
                return Ok(resp);
            }
        }
    }

    async fn call_with_attempt<E, S, Req, Resp, F, Fut>(
        &mut self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, E>
    where
        E: RetryableError,
        S: Clone,
        Req: Clone,
        F: Fn(S, Req, usize) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, E>>,
    {
        loop {
            let result = func(service.clone(), request.clone(), self.ctx.attempt).await;

            if let Some(resp) = self.handle_result(result).await? {
                return Ok(resp);
            }
        }
    }

    /// Handle result of a call
    ///
    /// If result is matching the retry config, this function will wait for the delay, returning
    /// `None`.
    async fn handle_result<E, Resp>(
        &mut self,
        result: Result<Response<Resp>, E>,
    ) -> Result<Option<Resp>, E>
    where
        E: RetryableError,
    {
        match result {
            Ok(response) => {
                self.config.success();
                Ok(Some(response.into_inner()))
            }
            Err(mut error) => {
                error.set_ctx(&self.ctx);

                let Some(delay) = error.retry_after(&mut self.config, &self.ctx) else {
                    return Err(error);
                };

                // We log only if the error is not returned, to avoid log duplication
                #[cfg(feature = "tracing")]
                tracing::trace!(
                    attempt = self.ctx.attempt,
                    ?error,
                    "Client encountered an error"
                );

                self.ctx.attempt += 1;
                delay.await;

                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        collections::VecDeque,
        pin::pin,
        rc::Rc,
        sync::Arc,
        task::{Context, Waker},
        time::Duration,
    };

    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use tokio::time::{self, Instant, Sleep};
    use tonic::{Code, Response, Status};

    use crate::retry::{
        CantonRetryPolicy, RetryConfig, RetryCtx, RetryPolicy, RetryableError,
        budget::{Budget as _, TpsBudget},
    };

    use super::RetryHandler;

    const RETRY_DELAY: Duration = Duration::from_secs(1);

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct SyntheticRequest {
        values: Vec<u32>,
    }

    // The response deliberately does not implement Clone.
    #[derive(Debug, PartialEq, Eq)]
    struct SyntheticResponse {
        value: String,
    }

    impl SyntheticResponse {
        fn new(value: &str) -> Self {
            Self {
                value: value.to_owned(),
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, thiserror::Error)]
    #[error("{code}: {message}")]
    struct SyntheticError {
        code: Code,
        message: String,
        attempt: Option<usize>,
    }

    impl From<Status> for SyntheticError {
        fn from(status: Status) -> Self {
            Self {
                code: status.code(),
                message: status.message().to_owned(),
                attempt: None,
            }
        }
    }

    impl RetryableError for SyntheticError {
        fn retry_after(&self, config: &mut RetryConfig, ctx: &RetryCtx) -> Option<Sleep> {
            assert_eq!(
                self.attempt,
                Some(ctx.attempt()),
                "error context must be set before evaluating a retry",
            );

            if self.code != Code::Unavailable {
                return None;
            }

            config.network_mut()?.next_backoff(ctx.attempt())
        }

        fn set_ctx(&mut self, ctx: &RetryCtx) {
            self.attempt = Some(ctx.attempt());
        }
    }

    #[derive(Debug)]
    struct Invocation {
        service_values: Vec<u32>,
        request: SyntheticRequest,
        attempt: Option<usize>,
        at: Instant,
    }

    #[derive(Debug, Default)]
    struct ServiceState {
        outcomes: VecDeque<Result<SyntheticResponse, Status>>,
        invocations: Vec<Invocation>,
    }

    #[derive(Clone, Debug)]
    struct SyntheticService {
        values: Vec<u32>,
        // Sharing only the script and observations keeps input mutations local to each clone.
        state: Rc<RefCell<ServiceState>>,
    }

    impl SyntheticService {
        fn enqueue(&self, outcomes: impl IntoIterator<Item = Result<SyntheticResponse, Status>>) {
            self.state.borrow_mut().outcomes.extend(outcomes);
        }

        async fn send(
            mut self,
            mut request: SyntheticRequest,
            attempt: Option<usize>,
        ) -> Result<Response<SyntheticResponse>, SyntheticError> {
            let mut state = self.state.borrow_mut();
            state.invocations.push(Invocation {
                service_values: self.values.clone(),
                request: request.clone(),
                attempt,
                at: Instant::now(),
            });

            self.values.push(99);
            request.values.push(99);

            state
                .outcomes
                .pop_front()
                .expect("unexpected invocation: synthetic script exhausted")
                .map(Response::new)
                .map_err(SyntheticError::from)
        }

        fn assert_invocations(&self, request: &SyntheticRequest, attempts: &[Option<usize>]) {
            let state = self.state.borrow();
            assert_eq!(state.invocations.len(), attempts.len());
            for (invocation, attempt) in state.invocations.iter().zip(attempts) {
                assert_eq!(invocation.service_values, self.values);
                assert_eq!(&invocation.request, request);
                assert_eq!(&invocation.attempt, attempt);
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum CallMethod {
        Call,
        WithAttempt,
    }

    impl CallMethod {
        async fn invoke(
            self,
            handler: &mut RetryHandler,
            service: &SyntheticService,
            request: &SyntheticRequest,
        ) -> Result<SyntheticResponse, SyntheticError> {
            match self {
                Self::Call => {
                    handler
                        .call(service, request, |service, request| {
                            service.send(request, None)
                        })
                        .await
                }
                Self::WithAttempt => {
                    handler
                        .call_with_attempt(service, request, |service, request, attempt| {
                            service.send(request, Some(attempt))
                        })
                        .await
                }
            }
        }

        fn attempts(self, count: usize) -> Vec<Option<usize>> {
            (0..count)
                .map(|attempt| match self {
                    Self::Call => None,
                    Self::WithAttempt => Some(attempt),
                })
                .collect()
        }
    }

    #[fixture]
    fn service() -> SyntheticService {
        SyntheticService {
            values: vec![1, 2],
            state: Rc::new(RefCell::new(ServiceState::default())),
        }
    }

    #[fixture]
    fn request() -> SyntheticRequest {
        SyntheticRequest {
            values: vec![10, 20],
        }
    }

    #[fixture]
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::new(RETRY_DELAY, RETRY_DELAY, 0.0).expect("valid deterministic backoff")
    }

    #[fixture]
    fn handler(retry_policy: RetryPolicy) -> RetryHandler {
        RetryHandler {
            config: RetryConfig::new().with_network_policy(Some(retry_policy.with_max(3))),
        }
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn immediate_success_returns_payload(
        #[values(CallMethod::Call, CallMethod::WithAttempt)] method: CallMethod,
        mut handler: RetryHandler,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        service.enqueue([Ok(SyntheticResponse::new("success"))]);
        let start = Instant::now();

        let result = method.invoke(&mut handler, &service, &request).await;

        assert_eq!(result, Ok(SyntheticResponse::new("success")));
        service.assert_invocations(&request, &method.attempts(1));
        assert_eq!(Instant::now(), start);
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn disabled_retry_returns_converted_error(
        #[values(CallMethod::Call, CallMethod::WithAttempt)] method: CallMethod,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        let mut handler = RetryHandler {
            config: RetryConfig::no_retry(),
        };
        service.enqueue([
            Err(Status::unavailable("terminal failure")),
            Ok(SyntheticResponse::new("must not be reached")),
        ]);
        let start = Instant::now();

        let result = method.invoke(&mut handler, &service, &request).await;

        assert_eq!(
            result,
            Err(SyntheticError {
                code: Code::Unavailable,
                message: "terminal failure".to_owned(),
                attempt: Some(0),
            }),
        );
        service.assert_invocations(&request, &method.attempts(1));
        assert_eq!(Instant::now(), start);
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn exhausted_limit_returns_last_error_with_context(
        #[values(CallMethod::Call, CallMethod::WithAttempt)] method: CallMethod,
        retry_policy: RetryPolicy,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        let max_retries = 2;
        let mut handler = RetryHandler {
            config: RetryConfig::new()
                .with_network_policy(Some(retry_policy.with_max(max_retries))),
        };
        service.enqueue(
            (0..=max_retries)
                .map(|attempt| Err(Status::unavailable(format!("failure {attempt}"))))
                .chain([Ok(SyntheticResponse::new("must not be reached"))]),
        );

        let result = method.invoke(&mut handler, &service, &request).await;

        assert_eq!(
            result,
            Err(SyntheticError {
                code: Code::Unavailable,
                message: format!("failure {max_retries}"),
                attempt: Some(max_retries),
            }),
        );
        service.assert_invocations(&request, &method.attempts(max_retries + 1));
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn reused_handler_starts_each_call_at_attempt_zero(
        #[values(CallMethod::Call, CallMethod::WithAttempt)] method: CallMethod,
        retry_policy: RetryPolicy,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        let mut handler = RetryHandler {
            config: RetryConfig::new().with_network_policy(Some(retry_policy.with_max(1))),
        };
        service.enqueue([
            Err(Status::unavailable("first retry")),
            Err(Status::unavailable("first terminal failure")),
            Err(Status::unavailable("second retry")),
            Ok(SyntheticResponse::new("second success")),
            Err(Status::unavailable("third retry")),
            Ok(SyntheticResponse::new("third success")),
        ]);

        let first = method.invoke(&mut handler, &service, &request).await;
        let second = method.invoke(&mut handler, &service, &request).await;
        let third = method.invoke(&mut handler, &service, &request).await;

        assert_eq!(
            first,
            Err(SyntheticError {
                code: Code::Unavailable,
                message: "first terminal failure".to_owned(),
                attempt: Some(1),
            }),
        );
        assert_eq!(second, Ok(SyntheticResponse::new("second success")));
        assert_eq!(third, Ok(SyntheticResponse::new("third success")));
        let attempts = method.attempts(2).repeat(3);
        service.assert_invocations(&request, &attempts);
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn reused_handler_restarts_backoff_at_minimum(
        #[values(CallMethod::Call, CallMethod::WithAttempt)] method: CallMethod,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        let policy = RetryPolicy::new(RETRY_DELAY, RETRY_DELAY * 8, 0.0)
            .expect("valid deterministic backoff");
        let mut handler = RetryHandler::from_config(
            RetryConfig::new().with_network_policy(Some(policy.with_max(1))),
        );

        for _ in 0..2 {
            service.enqueue([
                Err(Status::unavailable("temporary failure")),
                Ok(SyntheticResponse::new("success")),
            ]);
            let start = Instant::now();

            let result = method.invoke(&mut handler, &service, &request).await;

            assert_eq!(result, Ok(SyntheticResponse::new("success")));
            assert_eq!(Instant::now() - start, RETRY_DELAY);
        }

        service.assert_invocations(&request, &method.attempts(2).repeat(2));
    }

    #[rstest]
    #[tokio::test(start_paused = true)]
    async fn retries_wait_and_use_fresh_input_clones_until_success(
        #[values(CallMethod::Call, CallMethod::WithAttempt)] method: CallMethod,
        mut handler: RetryHandler,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        let original_service_values = service.values.clone();
        let original_request = request.clone();
        service.enqueue([
            Err(Status::unavailable("first retry")),
            Err(Status::unavailable("second retry")),
            Ok(SyntheticResponse::new("success")),
        ]);
        let start = Instant::now();
        let mut call = pin!(method.invoke(&mut handler, &service, &request));
        let mut cx = Context::from_waker(Waker::noop());

        // Poll explicitly so the runtime cannot automatically advance past a missing delay.
        for invocation_count in 1..=2 {
            assert!(call.as_mut().poll(&mut cx).is_pending());
            assert_eq!(service.state.borrow().invocations.len(), invocation_count);

            time::advance(RETRY_DELAY - Duration::from_millis(1)).await;
            assert!(call.as_mut().poll(&mut cx).is_pending());
            assert_eq!(service.state.borrow().invocations.len(), invocation_count);

            time::advance(Duration::from_millis(1)).await;
        }

        assert_eq!(call.await, Ok(SyntheticResponse::new("success")));
        service.assert_invocations(&original_request, &method.attempts(3));
        assert_eq!(service.values, original_service_values);
        assert_eq!(request, original_request);
        let invocation_times: Vec<_> = service
            .state
            .borrow()
            .invocations
            .iter()
            .map(|invocation| invocation.at)
            .collect();
        assert_eq!(
            invocation_times,
            vec![start, start + RETRY_DELAY, start + RETRY_DELAY * 2],
        );
    }

    #[rstest]
    #[case::success(true)]
    #[case::failure(false)]
    #[tokio::test(start_paused = true)]
    async fn only_success_replenishes_network_and_canton_budgets(
        #[case] succeeds: bool,
        retry_policy: RetryPolicy,
        service: SyntheticService,
        request: SyntheticRequest,
    ) {
        // No reserve and a 1:1 deposit ratio make each replenishment directly observable.
        let network_budget = Arc::new(TpsBudget::new(Duration::from_secs(60), 0, 1.0));
        let canton_budget = Arc::new(TpsBudget::new(Duration::from_secs(60), 0, 1.0));
        network_budget.deposit();
        let mut handler = RetryHandler {
            config: RetryConfig::new()
                .with_network_policy(Some(retry_policy.with_budget(network_budget.clone())))
                .with_canton_policy(Some(CantonRetryPolicy::auto_with_budget(
                    canton_budget.clone(),
                ))),
        };
        let terminal_outcome = if succeeds {
            Ok(SyntheticResponse::new("success"))
        } else {
            Err(Status::invalid_argument("terminal failure"))
        };
        service.enqueue([
            Err(Status::unavailable("temporary failure")),
            terminal_outcome,
        ]);

        let result = CallMethod::Call
            .invoke(&mut handler, &service, &request)
            .await;

        let expected = if succeeds {
            Ok(SyntheticResponse::new("success"))
        } else {
            Err(SyntheticError {
                code: Code::InvalidArgument,
                message: "terminal failure".to_owned(),
                attempt: Some(1),
            })
        };
        assert_eq!(result, expected);
        service.assert_invocations(&request, &CallMethod::Call.attempts(2));
        for budget in [network_budget, canton_budget] {
            assert_eq!(budget.withdraw(), succeeds);
            assert!(!budget.withdraw(), "only one deposit per successful call");
        }
    }
}
