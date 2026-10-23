//! Instance scheduling on a runtime independent of individual tests.
//!
//! Commands change ownership and admission state; lifecycle tasks report ready
//! and stopped events. A slot consumes capacity until its stopped event arrives,
//! including throughout startup and teardown. The coordinator handles signals
//! and keeps this runtime alive when a test's own runtime is dropped.
use std::{
    collections::{HashMap, HashSet, VecDeque},
    panic::AssertUnwindSafe,
    sync::{Arc, OnceLock, mpsc as blocking},
    thread::ThreadId,
};

use futures_util::FutureExt;
use tokio::{
    runtime::Handle,
    sync::{mpsc, oneshot},
    time::{Instant, sleep_until},
};
use tokio_util::sync::CancellationToken;

use crate::{
    Error, Sandbox, SandboxConfig,
    backend::{Backend, Canton},
};

type Reply<T> = oneshot::Sender<Result<T, Error>>;

/// Requests sent from test threads to a manager's event loop.
pub(crate) enum Command {
    /// Queue a claim; `None` selects a fresh instance and `Some` a shared group.
    Acquire {
        id: u64,
        group: Option<String>,
        reply: Reply<Arc<Sandbox>>,
    },
    /// Give up a claim, awaiting reuse or any cleanup owed by its final user.
    Release {
        id: u64,
        /// Reuse or teardown must acknowledge the final user's release.
        reply: Reply<()>,
    },
    /// Remove an abandoned claim and acknowledge any necessary teardown to Drop.
    Cancel {
        id: u64,
        reply: blocking::Sender<Result<(), Error>>,
    },
    /// Stop admission and instances, answering after all slots have finished.
    Close {
        /// Completes once every instance has finished stopping.
        reply: Reply<()>,
    },
    /// Close from the final client-side handle and retire the manager task.
    CloseBlocking {
        reply: blocking::Sender<Result<(), Error>>,
    },
}

/// Cross-thread manager handle held by the pool and its claim guards.
///
/// Blocking cancellation is safe because the worker runtime runs on a different
/// thread from the test that owns the guard.
pub(crate) struct Core {
    sender: mpsc::UnboundedSender<Command>,
    worker: ThreadId,
}

impl Core {
    pub(crate) fn send(&self, command: Command) -> Result<(), Error> {
        self.sender.send(command).map_err(|_| Error::Closed)
    }

    #[cfg(test)]
    pub(crate) async fn shutdown(&self) -> Result<(), Error> {
        let (reply, receive) = oneshot::channel();
        self.send(Command::Close { reply })?;
        receive.await.map_err(|_| Error::Closed)?
    }

    /// Detach a dropped acquisition or lease, waiting for cleanup when it leaves
    /// an instance unowned. This must never block the manager's own worker thread.
    pub(crate) fn cancel_blocking(&self, id: u64) {
        let (reply, receive) = blocking::channel();
        if self.send(Command::Cancel { id, reply }).is_ok() {
            // Public claims are constructed on the caller, never by the manager.
            assert_ne!(
                std::thread::current().id(),
                self.worker,
                "a sandbox claim was dropped on its manager thread"
            );
            if let Ok(Err(error)) = receive.recv() {
                eprintln!("{error}");
            }
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        let (reply, receive) = blocking::channel();
        if self.send(Command::CloseBlocking { reply }).is_ok()
            && std::thread::current().id() != self.worker
            && let Ok(Err(error)) = receive.recv()
        {
            eprintln!("{error}");
        }
    }
}

/// Process-wide worker runtime and registry used for the signal shutdown sweep.
struct Coordinator {
    runtime: Handle,
    worker: ThreadId,
    pools: mpsc::UnboundedSender<mpsc::UnboundedSender<Command>>,
}

/// Lazily start the independent worker and install SIGINT/SIGTERM handling.
/// The first signal closes registration, shuts down registered managers, and
/// exits with code 130 or 143 after cleanup. Repeated signals are drained meanwhile.
fn coordinator() -> Result<&'static Coordinator, Error> {
    static INSTANCE: OnceLock<Result<Coordinator, Error>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let (ready, receive) = blocking::channel();
        std::thread::Builder::new().name("canton-sandbox-manager".into()).spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => { let _ = ready.send(Err(Error::Configuration(error.to_string()))); return; }
            };
            runtime.block_on(async {
                let signals = (|| {
                    use tokio::signal::unix::{signal, SignalKind};
                    Ok::<_, std::io::Error>((signal(SignalKind::interrupt())?, signal(SignalKind::terminate())?))
                })();
                let (mut interrupt, mut terminate) = match signals {
                    Ok(signals) => signals,
                    Err(error) => { let _ = ready.send(Err(Error::Configuration(error.to_string()))); return; }
                };
                let (pools, mut incoming) = mpsc::unbounded_channel::<mpsc::UnboundedSender<Command>>();
                let _ = ready.send(Ok(Coordinator { runtime: Handle::current(), worker: std::thread::current().id(), pools }));
                let mut registry = Vec::new();
                let code = loop {
                    tokio::select! {
                        biased;
                        _ = interrupt.recv() => break 130,
                        _ = terminate.recv() => break 143,
                        Some(pool) = incoming.recv() => {
                            registry.retain(|pool: &mpsc::UnboundedSender<Command>| !pool.is_closed());
                            registry.push(pool);
                        }
                    }
                };
                // Prevent a concurrent registration from escaping the shutdown sweep.
                incoming.close();
                while let Ok(pool) = incoming.try_recv() { registry.push(pool); }
                let cleanup = registry.into_iter().map(|pool| async move {
                    let (reply, receive) = oneshot::channel();
                    if pool.send(Command::Close { reply }).is_err() { return Ok(()); }
                    receive.await.unwrap_or(Ok(()))
                });
                // Keep draining repeated signals while cleanup is in progress. They
                // must not restore the default action and strand a live child.
                let cleanup = futures_util::future::join_all(cleanup);
                tokio::pin!(cleanup);
                loop {
                    tokio::select! {
                        result = &mut cleanup => {
                            for error in result.into_iter().filter_map(Result::err) { eprintln!("{error}"); }
                            break;
                        }
                        _ = interrupt.recv() => (),
                        _ = terminate.recv() => (),
                    }
                }
                std::process::exit(code);
            });
        }).map_err(|error| Error::Configuration(error.to_string()))?;
        receive.recv().map_err(|_| Error::Configuration("manager failed to initialize".into()))?
    }).as_ref().map_err(Clone::clone)
}

/// Register a manager using validated settings and the real Canton backend.
pub(crate) fn register(config: SandboxConfig) -> Result<Arc<Core>, Error> {
    register_backend(config, Arc::new(Canton))
}

/// Register before starting lifecycle tasks so signal shutdown cannot miss them.
/// Backend substitution lets tests exercise scheduling without launching Canton.
pub(crate) fn register_backend(
    config: SandboxConfig,
    backend: Arc<dyn Backend>,
) -> Result<Arc<Core>, Error> {
    let coordinator = coordinator()?;
    let (sender, receive) = mpsc::unbounded_channel();
    // Register before spawning anything. A signal closes this admission channel.
    coordinator
        .pools
        .send(sender.clone())
        .map_err(|_| Error::Interrupted)?;
    let core = Arc::new(Core {
        sender,
        worker: coordinator.worker,
    });
    coordinator
        .runtime
        .spawn(Manager::new(config, backend, receive).run());
    Ok(core)
}

/// One acquisition whose guard remains on the requesting test's thread.
struct Request {
    id: u64,
    group: Option<String>,
    reply: Reply<Arc<Sandbox>>,
}

/// Scheduling state of a capacity slot, including time spent stopping its process.
enum Phase {
    /// Requests for the same group coalesce until startup reports readiness.
    Starting,
    /// The instance has at least one granted claim.
    Ready,
    /// Last release time, used to calculate the shared-instance retention deadline.
    Idle(Instant),
    /// Cancellation was requested; capacity is retained until teardown reports back.
    Stopping,
}

/// Ownership and reply state for one instance generation.
struct Slot {
    group: Option<String>,
    phase: Phase,
    sandbox: Option<Arc<Sandbox>>,
    clients: HashSet<u64>,
    /// Lost or rejected acquisition replies still awaiting guard cancellation.
    /// Keeping these IDs lets a dropped guard wait for teardown when necessary.
    abandoned: HashSet<u64>,
    pending: Vec<Request>,
    /// Final-user release replies held until reuse or confirmed shutdown.
    releases: HashMap<u64, Reply<()>>,
    cancellations: Vec<blocking::Sender<Result<(), Error>>>,
    cancel: CancellationToken,
}

/// Lifecycle outcomes reported back to the event loop by backend tasks.
enum Event {
    Ready(u64, Arc<Sandbox>),
    Stopped(u64, Result<(), Error>),
}

/// Serializes queue admission, shared groups, and cleanup acknowledgements.
///
/// Backend operations run in separate tasks on the same worker runtime, leaving
/// the event loop free to cancel startup or admit unrelated work concurrently.
struct Manager {
    config: Arc<SandboxConfig>,
    backend: Arc<dyn Backend>,
    commands: mpsc::UnboundedReceiver<Command>,
    events: mpsc::UnboundedReceiver<Event>,
    event_sender: mpsc::UnboundedSender<Event>,
    queue: VecDeque<Request>,
    slots: HashMap<u64, Slot>,
    groups: HashMap<String, u64>,
    /// Preserve a stopped instance's failure until each issued claim is released.
    finished_claims: HashMap<u64, Error>,
    /// Prevent replacing a failed group while its old claims remain outstanding.
    failed_groups: HashMap<String, (HashSet<u64>, Error)>,
    next_slot: u64,
    closed: bool,
    /// Unlike closed admission alone, retirement ends the task after slots drain.
    retired: bool,
    cleanup_error: Option<Error>,
    closers: Vec<Reply<()>>,
    blocking_closers: Vec<blocking::Sender<Result<(), Error>>>,
}

impl Manager {
    fn new(
        config: SandboxConfig,
        backend: Arc<dyn Backend>,
        commands: mpsc::UnboundedReceiver<Command>,
    ) -> Self {
        let (event_sender, events) = mpsc::unbounded_channel();
        Self {
            config: Arc::new(config),
            backend,
            commands,
            events,
            event_sender,
            queue: VecDeque::new(),
            slots: HashMap::new(),
            groups: HashMap::new(),
            finished_claims: HashMap::new(),
            failed_groups: HashMap::new(),
            next_slot: 0,
            closed: false,
            retired: false,
            cleanup_error: None,
            closers: Vec::new(),
            blocking_closers: Vec::new(),
        }
    }

    async fn run(mut self) {
        let mut disconnected = false;
        loop {
            self.schedule();
            self.notify_closed();
            if (disconnected || self.retired) && self.slots.is_empty() {
                return;
            }
            let deadline = self
                .slots
                .values()
                .filter_map(|slot| match slot.phase {
                    Phase::Idle(since) => Some(since + self.config.idle),
                    _ => None,
                })
                .min();
            tokio::select! {
                command = self.commands.recv(), if !disconnected => match command {
                    Some(command) => self.command(command),
                    None => { disconnected = true; self.close(); }
                },
                Some(event) = self.events.recv() => self.event(event),
                _ = async { match deadline { Some(deadline) => sleep_until(deadline).await, None => std::future::pending().await } } => {
                    for slot in self.slots.values_mut() {
                        if matches!(slot.phase, Phase::Idle(since) if since + self.config.idle <= Instant::now()) {
                            stop(slot);
                        }
                    }
                }
            }
        }
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Acquire { id, group, reply } => {
                if self.closed {
                    let _ = reply.send(Err(self.closed_error()));
                } else {
                    self.queue.push_back(Request { id, group, reply });
                }
            }
            Command::Release { id, reply } => self.release(id, reply),
            Command::Cancel { id, reply } => self.cancel(id, reply),
            Command::Close { reply } => {
                self.closers.push(reply);
                self.close();
            }
            Command::CloseBlocking { reply } => {
                self.blocking_closers.push(reply);
                self.retired = true;
                self.close();
            }
        }
    }

    fn closed_error(&self) -> Error {
        self.cleanup_error.clone().unwrap_or(Error::Closed)
    }

    /// Reject queued work and cancel every slot. Outstanding claim guards remain
    /// attached until stopped events make their cleanup outcome available.
    fn close(&mut self) {
        self.closed = true;
        let error = self.closed_error();
        for request in self.queue.drain(..) {
            let _ = request.reply.send(Err(error.clone()));
        }
        for slot in self.slots.values_mut() {
            for request in slot.pending.drain(..) {
                // Keep the acquisition guard attached until teardown finishes.
                slot.abandoned.insert(request.id);
                let _ = request.reply.send(Err(error.clone()));
            }
            stop(slot);
        }
    }

    /// Reply to shutdown waiters only after all capacity slots have disappeared.
    fn notify_closed(&mut self) {
        if self.closed && self.slots.is_empty() {
            let result = self.cleanup_error.clone().map_or(Ok(()), Err);
            for reply in self.closers.drain(..) {
                let _ = reply.send(result.clone());
            }
            for reply in self.blocking_closers.drain(..) {
                let _ = reply.send(result.clone());
            }
        }
    }

    /// Reuse shared slots before admitting new instances in queue order.
    /// If a new instance needs capacity, evict the oldest idle slot and wait for
    /// its stopped event before starting a replacement.
    fn schedule(&mut self) {
        while !self.closed {
            let index = self
                .queue
                .iter()
                .position(|request| {
                    request
                        .group
                        .as_ref()
                        .and_then(|group| self.groups.get(group))
                        .is_some_and(|id| !matches!(self.slots[id].phase, Phase::Stopping))
                })
                .unwrap_or(0);
            let Some(request) = self.queue.get(index) else {
                return;
            };
            if request.reply.is_closed() {
                self.queue.remove(index);
                continue;
            }
            if let Some((_, error)) = request
                .group
                .as_ref()
                .and_then(|group| self.failed_groups.get(group))
            {
                let error = error.clone();
                let request = self.queue.remove(index).unwrap();
                let _ = request.reply.send(Err(error));
                continue;
            }
            if let Some(slot_id) = request
                .group
                .as_ref()
                .and_then(|group| self.groups.get(group))
                .copied()
            {
                let slot = self.slots.get_mut(&slot_id).unwrap();
                match slot.phase {
                    Phase::Starting => slot.pending.push(self.queue.remove(index).unwrap()),
                    Phase::Ready | Phase::Idle(_) => {
                        grant(slot, self.queue.remove(index).unwrap());
                        if slot.clients.is_empty() {
                            stop(slot);
                        }
                    }
                    Phase::Stopping => return,
                }
                continue;
            }
            if self.slots.len() == self.config.capacity.get() {
                if let Some((_, slot)) = self
                    .slots
                    .iter_mut()
                    .filter_map(|(id, slot)| match slot.phase {
                        Phase::Idle(since) => Some(((since, *id), slot)),
                        _ => None,
                    })
                    .min_by_key(|(key, _)| *key)
                {
                    stop(slot);
                }
                return;
            }
            let request = self.queue.remove(index).unwrap();
            self.next_slot += 1;
            let id = self.next_slot;
            if let Some(group) = &request.group {
                self.groups.insert(group.clone(), id);
            }
            let cancel = CancellationToken::new();
            self.slots.insert(
                id,
                Slot {
                    group: request.group.clone(),
                    phase: Phase::Starting,
                    sandbox: None,
                    clients: HashSet::new(),
                    abandoned: HashSet::new(),
                    pending: vec![request],
                    releases: HashMap::new(),
                    cancellations: Vec::new(),
                    cancel: cancel.clone(),
                },
            );
            let backend = self.backend.clone();
            let config = self.config.clone();
            let events = self.event_sender.clone();
            tokio::spawn(async move {
                let lifecycle = async {
                    match backend.start(config, cancel.clone()).await {
                        Err(error) => Err(error),
                        Ok(mut process) => {
                            let _ = events.send(Event::Ready(id, process.sandbox()));
                            let primary = tokio::select! {
                                biased;
                                _ = cancel.cancelled() => None,
                                error = process.exited() => Some(error),
                            };
                            let cleanup = process.stop().await;
                            match primary {
                                Some(primary) => Err(Error::combine(primary, cleanup)),
                                None => cleanup,
                            }
                        }
                    }
                };
                let result = AssertUnwindSafe(lifecycle)
                    .catch_unwind()
                    .await
                    .unwrap_or_else(|_| {
                        Err(Error::Cleanup(
                            "sandbox process task panicked; teardown could not be confirmed".into(),
                        ))
                    });
                let _ = events.send(Event::Stopped(id, result));
            });
        }
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Ready(id, sandbox) => {
                let slot = self.slots.get_mut(&id).unwrap();
                slot.sandbox = Some(sandbox);
                if matches!(slot.phase, Phase::Stopping) {
                    return;
                }
                slot.phase = Phase::Ready;
                for request in std::mem::take(&mut slot.pending) {
                    grant(slot, request);
                }
                if slot.clients.is_empty() {
                    stop(slot);
                }
            }
            Event::Stopped(id, outcome) => {
                let mut slot = self.slots.remove(&id).unwrap();
                let error = outcome.clone().err().unwrap_or(Error::Closed);
                if let Some(group) = slot.group {
                    self.groups.remove(&group);
                    if !slot.clients.is_empty() {
                        self.failed_groups
                            .insert(group, (slot.clients.clone(), error.clone()));
                    }
                }
                for request in slot.pending {
                    let _ = request.reply.send(Err(error.clone()));
                }
                for client in slot.clients {
                    self.finished_claims.insert(client, error.clone());
                }
                for (_, reply) in slot.releases.drain() {
                    let _ = reply.send(outcome.clone());
                }
                for reply in slot.cancellations {
                    let _ = reply.send(outcome.clone());
                }
                if let Err(error) = outcome
                    && error.cleanup_failed()
                {
                    self.cleanup_error.get_or_insert(error);
                    self.close();
                }
            }
        }
    }

    /// Release one issued claim. The final shared claim starts the idle grace
    /// period and keeps its reply pending; a final fresh claim stops immediately.
    fn release(&mut self, id: u64, reply: Reply<()>) {
        if let Some(error) = self.forget_finished_claim(id) {
            let _ = reply.send(Err(error));
            return;
        }
        for slot in self.slots.values_mut() {
            if slot.clients.remove(&id) {
                if !slot.clients.is_empty() {
                    let _ = reply.send(Ok(()));
                    return;
                }
                slot.releases.insert(id, reply);
                if slot.group.is_none() || self.closed || matches!(slot.phase, Phase::Stopping) {
                    stop(slot);
                } else {
                    slot.phase = Phase::Idle(Instant::now());
                }
                return;
            }
        }
        let _ = reply.send(Err(self.closed_error()));
    }

    /// Remove a dropped guard from whichever phase it reached. An unowned slot
    /// bypasses idle retention, and its cancellation reply waits for teardown.
    fn cancel(&mut self, id: u64, reply: blocking::Sender<Result<(), Error>>) {
        if let Some(error) = self.forget_finished_claim(id) {
            let _ = reply.send(Err(error));
            return;
        }
        self.queue.retain(|request| request.id != id);
        for slot in self.slots.values_mut() {
            let had_pending = slot.pending.iter().any(|request| request.id == id);
            slot.pending.retain(|request| request.id != id);
            let had_client = slot.clients.remove(&id);
            let had_release = slot.releases.remove(&id).is_some();
            let had_abandoned = slot.abandoned.remove(&id);
            if had_pending || had_client || had_release || had_abandoned {
                if slot.clients.is_empty() && slot.pending.is_empty() {
                    slot.cancellations.push(reply);
                    stop(slot);
                } else {
                    let _ = reply.send(Ok(()));
                }
                return;
            }
        }
        let _ = reply.send(Ok(()));
    }

    /// Consume a claim's retained failure and allow group replacement once the
    /// final claim from the failed generation has been acknowledged.
    fn forget_finished_claim(&mut self, id: u64) -> Option<Error> {
        let error = self.finished_claims.remove(&id)?;
        self.failed_groups.retain(|_, (claims, _)| {
            claims.remove(&id);
            !claims.is_empty()
        });
        Some(error)
    }
}

/// Request teardown while keeping the slot counted against capacity.
fn stop(slot: &mut Slot) {
    slot.phase = Phase::Stopping;
    slot.cancel.cancel();
}

/// Transfer ownership to a new claim and acknowledge releases waiting for reuse.
/// Failed reply delivery remains tracked for the caller's cancellation guard.
fn grant(slot: &mut Slot, request: Request) {
    if request.reply.is_closed() {
        slot.abandoned.insert(request.id);
        return;
    }
    slot.clients.insert(request.id);
    if request
        .reply
        .send(Ok(slot.sandbox.as_ref().unwrap().clone()))
        .is_err()
    {
        slot.clients.remove(&request.id);
        slot.abandoned.insert(request.id);
        return;
    }
    slot.phase = Phase::Ready;
    for (_, reply) in slot.releases.drain() {
        let _ = reply.send(Ok(()));
    }
}

#[cfg(test)]
mod tests;
