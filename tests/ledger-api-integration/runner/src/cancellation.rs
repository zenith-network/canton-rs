use processkit::CancellationToken;
use std::io;
use tokio::{
    signal::unix::{SignalKind, signal},
    task::JoinHandle,
};

use crate::error::{Failure, Result};

/// Keep draining signals throughout cleanup; repeated requests are harmless.
pub(crate) struct Cancellation {
    token: CancellationToken,
    listener: JoinHandle<()>,
}

impl Cancellation {
    pub(crate) fn install() -> io::Result<Self> {
        let mut interrupt = signal(SignalKind::interrupt())?;
        let mut terminate = signal(SignalKind::terminate())?;
        let token = CancellationToken::new();
        let requested = token.clone();
        let listener = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = interrupt.recv() => requested.cancel(),
                    _ = terminate.recv() => requested.cancel(),
                }
            }
        });
        Ok(Self { token, listener })
    }

    pub(crate) fn check(&self) -> Result<()> {
        if self.token.is_cancelled() {
            Err(Failure::Interrupted)
        } else {
            Ok(())
        }
    }

    pub(crate) async fn cancelled(&self) {
        self.token.cancelled().await;
    }
}

impl Drop for Cancellation {
    fn drop(&mut self) {
        self.listener.abort();
    }
}
