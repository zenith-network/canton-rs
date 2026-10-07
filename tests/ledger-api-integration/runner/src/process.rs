//! Own one child and its containment group until shutdown and output draining.

use processkit::{ProcessGroup, ProcessGroupOptions};
use std::{fs::OpenOptions, io, path::Path, process::ExitStatus, time::Duration};
use tokio::{
    net::unix::pipe,
    process::{Child, Command},
    task::JoinHandle,
    time::Instant,
};

use crate::{
    cancellation::Cancellation,
    error::{Failure, Result, after_cleanup},
    output,
};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) enum LogMode {
    Overwrite,
    Append,
}

/// ProcessGroup provides containment and a kill-on-drop fallback. Normal paths
/// explicitly finish so the child is reaped and all output has been flushed.
pub(crate) struct Process {
    group: ProcessGroup,
    child: Child,
    output: Option<JoinHandle<io::Result<()>>>,
}

impl Process {
    fn spawn(mut command: Command) -> Result<Self> {
        let group = ProcessGroup::with_options(
            ProcessGroupOptions::default()
                .shutdown_timeout(SHUTDOWN_TIMEOUT)
                .escalate_to_kill(true),
        )?;
        // Cgroup containment does not isolate terminal signals. Use a separate
        // Unix process group as well so only the runner handles Ctrl-C.
        command.process_group(0);
        let child = group.spawn(command)?;
        Ok(Self {
            group,
            child,
            output: None,
        })
    }

    pub(crate) fn spawn_logged(
        mut command: Command,
        log: &Path,
        mode: LogMode,
        cancellation: &Cancellation,
    ) -> Result<Self> {
        cancellation.check()?;
        let append = matches!(mode, LogMode::Append);
        let output = OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .truncate(!append)
            .open(log)?;
        command.stdout(output.try_clone()?).stderr(output);
        Self::spawn(command)
    }

    async fn spawn_forwarded(
        mut command: Command,
        log: &Path,
        cancellation: &Cancellation,
    ) -> Result<Self> {
        cancellation.check()?;
        println!(
            "+ {} {}",
            command.as_std().get_program().to_string_lossy(),
            command
                .as_std()
                .get_args()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        );
        let log = tokio::fs::File::create(log).await?;
        let (sender, receiver) = pipe::pipe()?;
        // Child programs expect blocking stdout/stderr. Both share one pipe,
        // avoiding competing forwarding tasks and preserving merged byte order.
        let writer = sender.into_blocking_fd()?;
        command.stdout(writer.try_clone()?).stderr(writer);
        cancellation.check()?;
        let mut process = Self::spawn(command)?;
        process.output = Some(tokio::spawn(output::forward(receiver, log)));
        Ok(process)
    }

    /// None indicates the deadline elapsed. Callers must finish even on error.
    pub(crate) async fn wait(
        &mut self,
        cancellation: &Cancellation,
        deadline: Option<Instant>,
    ) -> Result<Option<ExitStatus>> {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(Failure::Interrupted),
            status = self.child.wait() => Ok(Some(status?)),
            _ = async {
                match deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending().await,
                }
            } => Ok(None),
        }
    }

    pub(crate) async fn finish(mut self) -> Result<()> {
        // Always stop descendants, even after the parent exited. Reap the
        // parent concurrently: processkit cannot reap children on our behalf.
        let (group, child) = tokio::join!(self.group.shutdown_ref(), self.child.wait());
        let cleanup = after_cleanup(
            group.map_err(Failure::from),
            child.map(|_| ()).map_err(Failure::from),
        );
        let output = match self.output.take() {
            Some(output) => match output.await {
                Ok(result) => result.map_err(Failure::from),
                Err(error) => {
                    Err(io::Error::other(format!("Output forwarding failed: {error}")).into())
                }
            },
            None => Ok(()),
        };
        after_cleanup(cleanup, output)
    }
}

pub(crate) async fn run(command: Command, log: &Path, cancellation: &Cancellation) -> Result<()> {
    let program = command.as_std().get_program().to_os_string();
    let mut process = Process::spawn_forwarded(command, log, cancellation).await?;
    let outcome = match process.wait(cancellation, None).await {
        Ok(Some(status)) if status.success() => Ok(()),
        Ok(Some(status)) => Err(Failure::command(program, status)),
        Ok(None) => unreachable!("unbounded wait"),
        Err(error) => Err(error),
    };
    after_cleanup(outcome, process.finish().await)
}
