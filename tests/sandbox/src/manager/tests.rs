use super::*;
use crate::{
    __private::{self, ExpectedPanic, Mode},
    Endpoints,
    backend::{Running, Work},
    pool::Pool,
};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Semaphore;

struct Fake {
    starts: AtomicUsize,
    live: AtomicUsize,
    peak: AtomicUsize,
    stops: AtomicUsize,
    start_gate: Semaphore,
    stop_gate: Semaphore,
    fail_start: AtomicBool,
    panic_start: AtomicBool,
    fail_stop: AtomicBool,
    crashes: Mutex<HashMap<String, mpsc::UnboundedSender<()>>>,
}

impl Fake {
    fn new() -> Arc<Self> {
        Self::gated(10000, 10000)
    }

    fn gated(starts: usize, stops: usize) -> Arc<Self> {
        Arc::new(Self {
            starts: AtomicUsize::new(0),
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            stops: AtomicUsize::new(0),
            start_gate: Semaphore::new(starts),
            stop_gate: Semaphore::new(stops),
            fail_start: AtomicBool::new(false),
            panic_start: AtomicBool::new(false),
            fail_stop: AtomicBool::new(false),
            crashes: Mutex::new(HashMap::new()),
        })
    }
}

impl Backend for Arc<Fake> {
    fn start(
        &self,
        _config: Arc<SandboxConfig>,
        cancel: CancellationToken,
    ) -> Work<'static, Result<Box<dyn Running>, Error>> {
        let fake = self.clone();
        Box::pin(async move {
            assert!(
                !fake.panic_start.load(Ordering::SeqCst),
                "injected backend panic"
            );
            let id = fake.starts.fetch_add(1, Ordering::SeqCst) + 1;
            let live = fake.live.fetch_add(1, Ordering::SeqCst) + 1;
            fake.peak.fetch_max(live, Ordering::SeqCst);
            let allowed = tokio::select! {
                _ = cancel.cancelled() => false,
                permit = fake.start_gate.acquire() => { permit.unwrap().forget(); true },
            };
            if !allowed || fake.fail_start.load(Ordering::SeqCst) {
                fake.stops.fetch_add(1, Ordering::SeqCst);
                fake.stop_gate.acquire().await.unwrap().forget();
                fake.live.fetch_sub(1, Ordering::SeqCst);
                let primary = if allowed {
                    Error::Startup {
                        message: "injected startup failure".into(),
                        artifacts: "fake".into(),
                    }
                } else {
                    Error::Closed
                };
                let cleanup = if fake.fail_stop.load(Ordering::SeqCst) {
                    Err(Error::Cleanup("injected cleanup failure".into()))
                } else {
                    Ok(())
                };
                return Err(Error::combine(primary, cleanup));
            }
            let info = info(format!("fake-{id}"), PathBuf::from("fake"));
            let (crash, crashed) = mpsc::unbounded_channel();
            fake.crashes.lock().unwrap().insert(info.id.clone(), crash);
            Ok(Box::new(FakeRunning {
                fake,
                info,
                crashed,
            }) as Box<dyn Running>)
        })
    }
}

fn info(id: String, artifacts: PathBuf) -> Arc<Sandbox> {
    Arc::new(Sandbox {
        id,
        artifacts,
        endpoints: Endpoints {
            ledger_api: "http://127.0.0.1:1".into(),
            admin_api: "http://127.0.0.1:2".into(),
            json_api: "http://127.0.0.1:3".into(),
            sequencer_public: "http://127.0.0.1:4".into(),
            sequencer_admin: "http://127.0.0.1:5".into(),
            mediator_admin: "http://127.0.0.1:6".into(),
        },
    })
}

struct FakeRunning {
    fake: Arc<Fake>,
    info: Arc<Sandbox>,
    crashed: mpsc::UnboundedReceiver<()>,
}
impl Running for FakeRunning {
    fn sandbox(&self) -> Arc<Sandbox> {
        self.info.clone()
    }
    fn exited(&mut self) -> Work<'_, Error> {
        Box::pin(async {
            self.crashed.recv().await;
            Error::ProcessExit {
                message: "injected exit".into(),
                artifacts: "fake".into(),
            }
        })
    }
    fn stop(self: Box<Self>) -> Work<'static, Result<(), Error>> {
        Box::pin(async move {
            self.fake.stops.fetch_add(1, Ordering::SeqCst);
            self.fake.stop_gate.acquire().await.unwrap().forget();
            self.fake.live.fetch_sub(1, Ordering::SeqCst);
            self.fake.crashes.lock().unwrap().remove(&self.info.id);
            if self.fake.fail_stop.load(Ordering::SeqCst) {
                Err(Error::Cleanup("injected cleanup failure".into()))
            } else {
                Ok(())
            }
        })
    }
}

fn pool(fake: &Arc<Fake>, capacity: usize) -> Arc<Pool> {
    Arc::new(Pool::with_backend(
        SandboxConfig::builder()
            .sdk_version("fake")
            .max_sandboxes(capacity)
            .idle_timeout(Duration::ZERO)
            .build()
            .unwrap(),
        Arc::new(fake.clone()),
    ))
}

async fn until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !condition() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("condition was not reached");
}

#[tokio::test]
async fn same_group_shares_different_groups_and_fresh_are_isolated() {
    let fake = Fake::new();
    let pool = pool(&fake, 2);
    let (first, second) = tokio::join!(pool.shared(), pool.shared_in_group("0"));
    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(first.id(), second.id());
    let other = pool.shared_in_group("other").await.unwrap();
    assert_ne!(first.id(), other.id());
    first.release().await.unwrap();
    second.release().await.unwrap();
    other.release().await.unwrap();
    let first = pool.fresh().await.unwrap();
    let id = first.id().to_owned();
    first.release().await.unwrap();
    let second = pool.fresh().await.unwrap();
    assert_ne!(id, second.id());
    second.release().await.unwrap();
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    assert!(fake.peak.load(Ordering::SeqCst) <= 2);
}

#[tokio::test]
async fn capacity_includes_stopping_and_admission_is_fifo() {
    let fake = Fake::gated(10000, 0);
    let pool = pool(&fake, 1);
    let first = pool.shared().await.unwrap();
    let releasing = tokio::spawn(first.release());
    let (first_reply, first_receive) = oneshot::channel();
    let (second_reply, mut second_receive) = oneshot::channel();
    pool.core
        .send(Command::Acquire {
            id: 100,
            group: None,
            reply: first_reply,
        })
        .unwrap();
    pool.core
        .send(Command::Acquire {
            id: 101,
            group: None,
            reply: second_reply,
        })
        .unwrap();
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    assert_eq!(fake.starts.load(Ordering::SeqCst), 1);
    fake.stop_gate.add_permits(1);
    releasing.await.unwrap().unwrap();
    let first = first_receive.await.unwrap().unwrap();
    assert_eq!(first.id(), "fake-2");
    assert!(matches!(
        second_receive.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    let (reply, receive) = oneshot::channel();
    pool.core.send(Command::Release { id: 100, reply }).unwrap();
    fake.stop_gate.add_permits(1);
    receive.await.unwrap().unwrap();
    assert_eq!(second_receive.await.unwrap().unwrap().id(), "fake-3");
    fake.stop_gate.add_permits(1);
    pool.shutdown().await.unwrap();
    assert_eq!(fake.peak.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn idle_delay_reacquisition_and_capacity_eviction() {
    let fake = Fake::new();
    let (send, receive) = mpsc::unbounded_channel();
    let task = tokio::spawn(
        Manager::new(
            SandboxConfig::builder()
                .sdk_version("fake")
                .max_sandboxes(1)
                .build()
                .unwrap(),
            Arc::new(fake.clone()),
            receive,
        )
        .run(),
    );
    let acquire = |id, group: &str| {
        let (reply, receive) = oneshot::channel();
        send.send(Command::Acquire {
            id,
            group: Some(group.into()),
            reply,
        })
        .unwrap();
        receive
    };
    let first = acquire(1, "a").await.unwrap().unwrap();
    let (reply, mut release) = oneshot::channel();
    send.send(Command::Release { id: 1, reply }).unwrap();
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(matches!(
        release.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    let second = acquire(2, "a").await.unwrap().unwrap();
    assert_eq!(first.id(), second.id());
    release.await.unwrap().unwrap();
    let (reply, release) = oneshot::channel();
    send.send(Command::Release { id: 2, reply }).unwrap();
    // A new group evicts idle a before its three-second grace expires.
    let third = acquire(3, "b").await.unwrap().unwrap();
    release.await.unwrap().unwrap();
    assert_ne!(third.id(), first.id());
    let (reply, mut release) = oneshot::channel();
    send.send(Command::Release { id: 3, reply }).unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(matches!(
        release.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    tokio::time::advance(Duration::from_secs(1)).await;
    release.await.unwrap().unwrap();
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    drop(send);
    task.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_startup_release_and_dropped_leases_leaves_no_claims() {
    let fake = Fake::gated(0, 10000);
    let pool = pool(&fake, 1);
    let acquiring = tokio::spawn({
        let pool = pool.clone();
        async move { pool.shared().await }
    });
    until(|| fake.live.load(Ordering::SeqCst) == 1).await;
    acquiring.abort();
    let _ = acquiring.await;
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    fake.start_gate.add_permits(100);
    let lease = pool.shared().await.unwrap();
    drop(lease);
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);

    let pool = Arc::new(Pool::with_backend(
        SandboxConfig::builder()
            .sdk_version("fake")
            .idle_timeout(Duration::from_secs(60))
            .build()
            .unwrap(),
        Arc::new(fake.clone()),
    ));
    let lease = pool.shared().await.unwrap();
    let releasing = tokio::spawn(lease.release());
    tokio::time::sleep(Duration::from_millis(20)).await;
    releasing.abort();
    let _ = releasing.await;
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    pool.fresh().await.unwrap().release().await.unwrap();
}

#[tokio::test]
async fn failures_are_reported_and_cleanup_failure_closes_pool() {
    let fake = Fake::new();
    let pool = pool(&fake, 1);
    fake.fail_start.store(true, Ordering::SeqCst);
    assert!(matches!(pool.shared().await, Err(Error::Startup { .. })));
    fake.fail_start.store(false, Ordering::SeqCst);
    let lease = pool.shared().await.unwrap();
    let second = pool.shared().await.unwrap();
    fake.crashes.lock().unwrap()[lease.id()].send(()).unwrap();
    until(|| fake.live.load(Ordering::SeqCst) == 0).await;
    // A request racing the exit notification can still observe the old instance,
    // but must never be handed a replacement while the original claims survive.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match pool.shared().await {
                Err(Error::ProcessExit { .. }) => break,
                Ok(racing) => {
                    assert_eq!(racing.id(), lease.id());
                    let _ = racing.release().await;
                }
                _ => panic!("unexpected acquisition result"),
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        lease.release().await,
        Err(Error::ProcessExit { .. })
    ));
    assert!(matches!(
        pool.shared().await,
        Err(Error::ProcessExit { .. })
    ));
    assert!(matches!(
        second.release().await,
        Err(Error::ProcessExit { .. })
    ));
    let lease = pool.shared().await.unwrap();
    fake.fail_stop.store(true, Ordering::SeqCst);
    assert!(matches!(lease.release().await, Err(Error::Cleanup(_))));
    assert!(matches!(pool.shared().await, Err(Error::Cleanup(_))));
    assert!(pool.shutdown().await.is_err());
}

#[test]
fn first_callers_runtime_can_end_while_another_lease_remains() {
    let fake = Fake::new();
    let pool = pool(&fake, 2);
    let first_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (first, second) = first_runtime
        .block_on(async { (pool.shared().await.unwrap(), pool.shared().await.unwrap()) });
    let id = first.id().to_owned();
    first_runtime.block_on(first.release()).unwrap();
    drop(first_runtime);
    let next_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    next_runtime.block_on(async {
        let next = pool.shared().await.unwrap();
        assert_eq!(next.id(), id);
        second.release().await.unwrap();
        next.release().await.unwrap();
    });
    assert_eq!(fake.starts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn original_returned_error_survives_cleanup_failure() {
    let fake = Fake::new();
    fake.fail_stop.store(true, Ordering::SeqCst);
    let pool = pool(&fake, 1);
    let result = __private::run_with_pool(
        &pool,
        Mode::Fresh,
        "double failure",
        ExpectedPanic::No,
        async |_sandbox: &Sandbox| Err::<(), _>("primary failure"),
    )
    .await;
    assert_eq!(result.unwrap_err(), "primary failure");
}

#[tokio::test]
async fn cancellation_after_lost_grant_and_during_pool_close_waits_for_cleanup() {
    for close in [false, true] {
        let fake = Fake::gated(0, 0);
        let (send, receive) = mpsc::unbounded_channel();
        let manager = tokio::spawn(
            Manager::new(
                SandboxConfig::builder()
                    .sdk_version("fake")
                    .build()
                    .unwrap(),
                Arc::new(fake.clone()),
                receive,
            )
            .run(),
        );
        let (reply, receive) = oneshot::channel();
        send.send(Command::Acquire {
            id: 1,
            group: Some("a".into()),
            reply,
        })
        .unwrap();
        until(|| fake.live.load(Ordering::SeqCst) == 1).await;
        let mut closing = None;
        if close {
            let (reply, finished) = oneshot::channel();
            closing = Some(finished);
            send.send(Command::Close { reply }).unwrap();
            assert!(receive.await.unwrap().is_err());
        } else {
            drop(receive);
            fake.start_gate.add_permits(1);
        }
        until(|| fake.stops.load(Ordering::SeqCst) == 1).await;
        let (reply, acknowledgement) = blocking::channel();
        send.send(Command::Cancel { id: 1, reply }).unwrap();
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert!(matches!(
            acknowledgement.try_recv(),
            Err(blocking::TryRecvError::Empty)
        ));
        fake.stop_gate.add_permits(1);
        until(|| acknowledgement.try_recv().is_ok()).await;
        assert_eq!(fake.live.load(Ordering::SeqCst), 0);
        if let Some(closing) = closing {
            closing.await.unwrap().unwrap();
        }
        drop(send);
        manager.await.unwrap();
    }
}

#[tokio::test]
async fn dropping_one_coalesced_request_does_not_stop_other_users() {
    let fake = Fake::gated(0, 10000);
    let (send, receive) = mpsc::unbounded_channel();
    let manager = tokio::spawn(
        Manager::new(
            SandboxConfig::builder()
                .sdk_version("fake")
                .idle_timeout(Duration::ZERO)
                .build()
                .unwrap(),
            Arc::new(fake.clone()),
            receive,
        )
        .run(),
    );
    let (reply, abandoned) = oneshot::channel();
    send.send(Command::Acquire {
        id: 1,
        group: Some("a".into()),
        reply,
    })
    .unwrap();
    let (reply, active) = oneshot::channel();
    send.send(Command::Acquire {
        id: 2,
        group: Some("a".into()),
        reply,
    })
    .unwrap();
    until(|| fake.live.load(Ordering::SeqCst) == 1).await;
    drop(abandoned);
    fake.start_gate.add_permits(1);
    active.await.unwrap().unwrap();
    let (reply, acknowledgement) = blocking::channel();
    send.send(Command::Cancel { id: 1, reply }).unwrap();
    until(|| acknowledgement.try_recv().is_ok()).await;
    assert_eq!(fake.live.load(Ordering::SeqCst), 1);
    assert_eq!(fake.starts.load(Ordering::SeqCst), 1);
    let (reply, released) = oneshot::channel();
    send.send(Command::Release { id: 2, reply }).unwrap();
    released.await.unwrap().unwrap();
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    drop(send);
    manager.await.unwrap();
}

#[tokio::test]
async fn startup_and_cleanup_errors_are_both_retained() {
    let fake = Fake::new();
    fake.fail_start.store(true, Ordering::SeqCst);
    fake.fail_stop.store(true, Ordering::SeqCst);
    let pool = pool(&fake, 1);
    let error = match pool.shared().await {
        Err(error) => error,
        Ok(_) => panic!("startup unexpectedly passed"),
    };
    let message = error.to_string();
    assert!(message.contains("injected startup failure"));
    assert!(message.contains("injected cleanup failure"));
    assert!(pool.shared().await.is_err());
}

#[tokio::test]
async fn process_task_panic_closes_pool_instead_of_stranding_requests() {
    let fake = Fake::new();
    fake.panic_start.store(true, Ordering::SeqCst);
    let pool = pool(&fake, 1);
    assert!(matches!(pool.shared().await, Err(Error::Cleanup(_))));
    assert!(matches!(pool.shared().await, Err(Error::Cleanup(_))));
    assert!(pool.shutdown().await.is_err());
}

mod process;
