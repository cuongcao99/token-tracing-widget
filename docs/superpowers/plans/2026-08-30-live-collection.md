# Live Collection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Biến one-shot startup collection thành live collection loop cho native Claude Code và Codex, gồm Windows filesystem notifications, coalescing, reconciliation mỗi 30 giây, retry/backoff bounded, và shutdown seam rõ ràng.

**Architecture:** Giữ `CollectionCoordinator::collect` là đường collection duy nhất. Một adapter Windows native dùng `ReadDirectoryChangesW` chỉ gửi `Provider` signal, không gửi path hay record; một scheduler thuần Rust coalesces burst notification trong 200 ms, chạy reconciliation độc lập mỗi 30 giây, và chặn notification mới vượt qua retry backoff. `AppState` được chia sẻ bằng `Arc`, live loop chạy trong thread nội bộ cùng executable, publish `UsageSummary` chỉ sau khi `collect` trả về `CollectionReport` sau commit thành công.

**Tech Stack:** Rust 2021, Tauri 2, Win32 `ReadDirectoryChangesW` qua FFI nội bộ, `rusqlite`, `serde`, React 19, TypeScript, Vite, Vitest, và plain CSS hiện có. Không thêm crate watcher hay dependency mới.

**Spec:** `docs/superpowers/specs/2026-08-29-token-tracing-widget-design.md`

## Global Constraints

- Version 1 remains local-only and Windows 11-only.
- Use Tauri 2, Rust, React, TypeScript, Vite, plain CSS, and SQLite as already approved; do not add a frontend state library, CSS framework, ORM, background service, sidecar, network client, or telemetry.
- Rust owns all filesystem and SQLite access; the React webview receives typed `UsageSummary` values only.
- Native roots remain `%USERPROFILE%\.claude\projects` and `%USERPROFILE%\.codex\sessions` in this slice.
- Provider-specific formats remain behind the existing adapters; live loop calls existing bounded discovery, checkpoint, normalization, delta conversion, deduplication, and transactional persistence.
- Prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, raw JSON, and absolute source paths never enter normalized events, SQLite, diagnostics, or frontend/event payloads.
- Cumulative observations are converted to deltas; cumulative values are never summed directly. A counter decrease starts a new monotonic segment and never creates a negative delta.
- A successful live attempt emits `usage-summary-changed` only after the corresponding event/checkpoint transaction and post-commit summary query succeed.
- A SQLite failure leaves the existing `Stale` summary available, emits no fresh summary, and retries with `1 s`, `2 s`, `4 s`, `8 s`, `16 s`, then a maximum of `30 s` between attempts.
- A raw filesystem notification never triggers one collection call by itself; notification bursts are coalesced with a maximum `200 ms` debounce window.
- Reconciliation remains an independent `30 s` monotonic deadline and is not postponed by successful notification-driven collections.
- Native watcher failures are provider-scoped, path-free, and do not stop the other provider or the reconciliation loop.
- Use Win32 FFI already available from the Windows platform. Do not add `notify`, `windows`, `windows-sys`, or another watcher crate. No `Cargo.toml` or `Cargo.lock` dependency change is part of this plan.
- Do not implement tray actions, Settings, remembered position, startup registration, single-instance enforcement, installer/uninstaller, explicit WSL UNC roots, WSL discovery, clear-index recovery, or database backup/rebuild in this slice.

---

## Context and baseline

Handoff `token-tracing-widget-handoff-2026-08-30-live-collection.md` identifies clean branch `dev` at `23cc0a7` (`docs: record window usability plan`). Existing Rust/frontend gates passed after the previous slice: 10 frontend tests, 79 Rust tests across all targets, TypeScript/Vite build, integrated no-bundle Tauri release build, and the Windows overlay smoke check.

Current seams:

- `src-tauri/src/app/runtime.rs` owns `AppState`, `CollectionCoordinator<IndexStore>`, native profile-root resolution, bounded discovery limits, and `AppState::collect_once`.
- `src-tauri/src/collection/mod.rs` already performs provider-independent collection, one event/checkpoint/source/diagnostic transaction, and summary computation only after commit.
- `src-tauri/src/sources/session_files.rs` already bounds discovery to the two native roots and exposes Rust-private filesystem handles.
- `src-tauri/src/sources/file_watcher.rs` is an empty scaffold.
- `src-tauri/src/app/tray.rs`, `startup.rs`, and `window.rs` remain empty shell scaffolds and stay untouched.
- `src-tauri/src/lib.rs` currently performs one collection during Tauri setup and emits the initial post-commit summary, but starts no live worker.
- `src-tauri/src/commands/usage_summary.rs` already owns the stable `get_usage_summary` command and `usage-summary-changed` event adapter.
- Existing `src-tauri/tests/collection_core.rs` covers append, partial final line, rotation/truncation, restart, cumulative deltas, provider independence, and stale-on-storage-failure behavior. Preserve those tests and run them as regression gates.

The live slice therefore adds scheduling, notification delivery, worker lifetime, and Tauri startup/shutdown wiring. It must not create a second parser, aggregation path, SQLite query path, or frontend polling path.

## Design decisions

### Native watcher

Use one in-process worker per currently existing native provider root. Each worker opens its directory with `FILE_LIST_DIRECTORY`, shared read/write/delete access, `FILE_FLAG_OVERLAPPED`, and recursive `ReadDirectoryChangesW`. Notify filters include file-name, directory-name, size, and last-write changes. A bounded 64 KiB buffer is enough for the signal adapter; overflow or malformed notification data sends a generic provider change signal so the 30-second reconciliation repairs the exact state.

The watcher never parses filenames and never forwards a path. It sends only:

```rust
pub(crate) enum WatchSignal {
    Changed(Provider),
    WatchUnavailable(Provider),
    Shutdown,
}
```

`WatchUnavailable(Provider)` causes an ordinary collection attempt after debounce, allowing existing discovery to update that provider's sanitized health while the other provider continues. Missing roots are omitted from the watcher and discovered again during reconciliation.

### Scheduler and retry

`LiveScheduler` uses `Instant` deadlines, not wall-clock timestamps. It owns four facts: the first notification deadline in the current burst, the next independent reconciliation deadline, the next retry deadline, and the current retry attempt. A later notification never moves an existing debounce deadline later, so an unbounded write burst cannot postpone collection forever. A pending retry blocks notification-driven attempts until the retry deadline is reached.

Default policy is exact:

```text
notification debounce: 200 ms
reconciliation interval: 30 s
retry delays: 1 s, 2 s, 4 s, 8 s, 16 s, 30 s, 30 s, ...
```

Successful collection resets retry state. Failed collection keeps `AppState`'s existing stale summary and schedules the next attempt. An event-emission failure is not a collection failure: SQLite has already committed, so the scheduler resets retry state and logs only the existing path-free `summary_event:emit` category.

### Worker lifetime

`LiveCollectionHandle` owns a channel sender and the worker `JoinHandle` behind mutexes. `shutdown()` sends `WatchSignal::Shutdown`, the loop stops waiting, closes/cancels native watcher I/O, joins watcher workers, then joins the collector worker. Shutdown is idempotent and is called from `RunEvent::Exit`; `Drop` calls the same method as a final safety net. No tray or close-to-hide behavior is added here.

### Deep module seam

The live loop hides timing, retry, channel, watcher, backend, and event-publishing details behind these internal interfaces:

```rust
trait CollectionBackend: Send {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError>;
    fn watch_roots(&self) -> Vec<WatchRoot>;
}

trait SummaryPublisher: Send {
    fn publish(
        &mut self,
        summary: &UsageSummary,
    ) -> Result<(), SummaryEventError>;
}
```

Production adapters wrap `AppState` and `AppHandle`. Tests replace them with scripted backends and recording publishers, so scheduler and commit-gating behavior can be proved without sleeping or constructing a Tauri window.

## File map

- Create `src-tauri/src/app/live_collection.rs`: scheduler, retry policy, backend/publisher interfaces, worker loop, production adapters, `LiveCollectionHandle`, and deterministic unit tests.
- Modify `src-tauri/src/app/mod.rs`: expose `live_collection` beside the existing runtime/scaffold modules.
- Modify `src-tauri/src/app/runtime.rs`: make `AppState` cheaply clonable through shared runtime state and expose a Rust-only `watch_roots()` method that resolves only the existing native roots.
- Complete `src-tauri/src/sources/file_watcher.rs`: `WatchRoot`, path-free `WatchSignal`, native Win32 watcher, cancellation, refresh, sanitized watcher categories, and Windows notification tests.
- Modify `src-tauri/src/lib.rs`: keep the initial collection/event, start the live handle after setup collection, manage the handle, and shut it down on `tauri::RunEvent::Exit`.
- Review `src-tauri/src/collection/mod.rs`, `src-tauri/src/commands/usage_summary.rs`, `src-tauri/src/types/usage_summary.rs`, and `src-tauri/src/types/file_checkpoint.rs`: no behavior or wire-contract changes are expected; existing post-commit, stale, partial-line, and checkpoint rules remain the only collection path.
- Preserve `src-tauri/tests/collection_core.rs` and `src-tauri/tests/runtime_integration.rs` as regression gates; new live-loop tests live in `src-tauri/src/app/live_collection.rs`.
- Do not modify React files, `package.json`, `Cargo.toml`, `Cargo.lock`, Tauri capabilities, or window configuration.

---

### Task 1: Add deterministic live scheduler and test seam

**Files:**
- Create: `src-tauri/src/app/live_collection.rs`
- Modify: `src-tauri/src/app/mod.rs`
- Test: `src-tauri/src/app/live_collection.rs` (`#[cfg(test)]` module)

**Interfaces:**
- `LiveCollectionConfig::default()` returns `200 ms` notification debounce, `30 s` reconciliation interval, `1 s` retry base, and `30 s` retry maximum.
- `CollectionReason` has exactly `Notification`, `Reconciliation`, and `Retry` variants.
- `LiveScheduler::new(start: Instant, config: LiveCollectionConfig) -> LiveScheduler` schedules the first reconciliation at `start + reconciliation_interval`.
- `LiveScheduler::mark_changed(now: Instant)` records the first notification deadline in a burst without moving it later.
- `LiveScheduler::next_deadline() -> Instant` returns the earliest active notification, retry, or reconciliation deadline.
- `LiveScheduler::take_due(now: Instant) -> Option<CollectionReason>` consumes one due trigger, advances reconciliation by whole intervals, and never bypasses a pending retry deadline.
- `LiveScheduler::record_success()` clears retry state and returns the scheduler to notification/reconciliation mode.
- `LiveScheduler::record_failure(now: Instant)` retains retry state and schedules the exact bounded exponential delay.

- [x] **Step 1: Register the live module without changing runtime behavior.**

Add only this declaration to `src-tauri/src/app/mod.rs`:

```rust
pub mod live_collection;
```

Keep `runtime`, `startup`, `tray`, and `window` declarations unchanged. The module initially contains only the scheduler and its tests; Tauri setup remains one-shot until Task 4.

- [x] **Step 2: Write the failing scheduler tests.**

Add a test-only configuration helper and these exact cases:

```rust
fn test_config() -> LiveCollectionConfig {
    LiveCollectionConfig {
        notification_debounce: Duration::from_millis(200),
        reconciliation_interval: Duration::from_secs(30),
        retry_base: Duration::from_secs(1),
        retry_max: Duration::from_secs(30),
    }
}

#[test]
fn notification_burst_has_one_bounded_debounce_deadline() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());

    scheduler.mark_changed(start);
    scheduler.mark_changed(start + Duration::from_millis(50));

    assert!(scheduler
        .take_due(start + Duration::from_millis(199))
        .is_none());
    assert_eq!(
        scheduler.take_due(start + Duration::from_millis(200)),
        Some(CollectionReason::Notification)
    );
    assert!(scheduler
        .take_due(start + Duration::from_millis(201))
        .is_none());
}

#[test]
fn reconciliation_deadline_is_not_reset_by_notification_collection() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());

    scheduler.mark_changed(start);
    assert_eq!(
        scheduler.take_due(start + Duration::from_millis(200)),
        Some(CollectionReason::Notification)
    );
    scheduler.record_success();

    assert_eq!(
        scheduler.take_due(start + Duration::from_secs(30)),
        Some(CollectionReason::Reconciliation)
    );
}

#[test]
fn retry_backoff_is_exponential_and_capped_at_thirty_seconds() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());
    let delays = [1_u64, 2, 4, 8, 16, 30, 30];
    let mut failure_at = start;

    for delay in delays {
        scheduler.record_failure(failure_at);
        assert_eq!(
            scheduler.next_deadline(),
            failure_at + Duration::from_secs(delay)
        );
        assert_eq!(
            scheduler.take_due(failure_at + Duration::from_secs(delay)),
            Some(CollectionReason::Retry)
        );
        failure_at += Duration::from_secs(delay);
    }
}

#[test]
fn notification_cannot_bypass_pending_retry() {
    let start = Instant::now();
    let mut scheduler = LiveScheduler::new(start, test_config());

    scheduler.record_failure(start);
    scheduler.mark_changed(start + Duration::from_millis(1));

    assert!(scheduler
        .take_due(start + Duration::from_millis(201))
        .is_none());
    assert_eq!(
        scheduler.take_due(start + Duration::from_secs(1)),
        Some(CollectionReason::Retry)
    );
}

#[test]
fn idle_scheduler_waits_until_reconciliation_without_busy_polling() {
    let start = Instant::now();
    let scheduler = LiveScheduler::new(start, test_config());

    assert_eq!(
        scheduler.next_deadline(),
        start + Duration::from_secs(30)
    );
}
```

The tests use `Instant` arithmetic only; no test sleeps. They prove coalescing, independent reconciliation, retry ordering, cap, and idle waiting before any worker thread or Win32 code exists.

- [x] **Step 3: Run the focused tests and verify the scheduler seam is absent.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
```

Expected: the new test module fails to compile because `LiveCollectionConfig`, `CollectionReason`, and `LiveScheduler` do not exist.

- [x] **Step 4: Implement the minimal scheduler.**

Use these state fields and rules:

```rust
struct LiveScheduler {
    config: LiveCollectionConfig,
    notification_deadline: Option<Instant>,
    reconciliation_deadline: Instant,
    retry_deadline: Option<Instant>,
    retry_attempt: u32,
}
```

`mark_changed` sets `notification_deadline` to `Some(existing.min(now + debounce))` or `Some(now + debounce)` when no burst is pending. `take_due` returns `None` while a retry exists in the future; returns `Retry` when retry is due; then returns due `Notification`; then returns due `Reconciliation`. When reconciliation is due, advance its deadline in a loop until it is strictly after `now`, so a delayed worker does not create a tight catch-up loop. `record_failure` computes `min(retry_max, retry_base * 2^retry_attempt)` with checked multiplication, increments the attempt with saturation, and stores `now + delay`. `record_success` clears retry deadline and resets the attempt counter to zero.

- [x] **Step 5: Run the scheduler tests and formatting.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
```

Expected: all five scheduler tests pass; no dependency or existing runtime behavior changes.

- [x] **Step 6: Commit the scheduler seam.**

```powershell
git add src-tauri/src/app/mod.rs src-tauri/src/app/live_collection.rs
git commit -m "feat: add live collection scheduler"
```

### Task 2: Implement the path-free native Windows watcher

**Files:**
- Modify: `src-tauri/src/sources/file_watcher.rs`
- Test: `src-tauri/src/sources/file_watcher.rs` (`#[cfg(test)]` module)

**Interfaces:**
- `WatchRoot::new(provider: Provider, path: PathBuf) -> WatchRoot` stores a Rust-private resolved directory.
- `WatchRoot::provider() -> Provider` and `WatchRoot::path() -> &Path` are Rust-only accessors.
- `WatchSignal` is exactly `Changed(Provider)`, `WatchUnavailable(Provider)`, or `Shutdown`; it has no path, filename, record, or error payload.
- `FileWatcher::start(roots: Vec<WatchRoot>, sender: Sender<WatchSignal>) -> FileWatcher` starts one worker per usable root and reports per-root startup failure through `WatchUnavailable`.
- `FileWatcher::replace_roots(&mut self, roots: Vec<WatchRoot>)` stops current workers, joins them, and starts workers for the refreshed set.
- `FileWatcher::shutdown(&mut self)` is idempotent, signals native cancellation, joins all workers, and never panics when a worker already exited.

- [x] **Step 1: Write failing watcher contract and Windows notification tests.**

Add the path-free signal types and tests before implementing their methods. The Windows integration test must use a temporary directory and assert that the received signal contains only the provider:

```rust
#[cfg(windows)]
#[test]
fn native_watcher_reports_file_change_without_forwarding_path() {
    let root = tempfile::tempdir().expect("watch root should be created");
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut watcher = FileWatcher::start(
        vec![WatchRoot::new(Provider::Claude, root.path().to_path_buf())],
        sender,
    );

    std::fs::write(root.path().join("session.jsonl"), b"metadata-only\n")
        .expect("session file should be written");

    let signal = receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("native watcher should report the change");
    assert_eq!(signal, WatchSignal::Changed(Provider::Claude));
    assert!(!format!("{signal:?}").contains("session.jsonl"));

    watcher.shutdown();
}

#[cfg(windows)]
#[test]
fn one_unusable_root_does_not_stop_a_usable_provider_watcher() {
    let root = tempfile::tempdir().expect("watch root should be created");
    let missing = root.path().join("missing");
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut watcher = FileWatcher::start(
        vec![
            WatchRoot::new(Provider::Claude, root.path().to_path_buf()),
            WatchRoot::new(Provider::Codex, missing),
        ],
        sender,
    );

    std::fs::write(root.path().join("session.jsonl"), b"metadata-only\n")
        .expect("session file should be written");

    let mut saw_claude = false;
    for _ in 0..3 {
        if let Ok(signal) = receiver.recv_timeout(std::time::Duration::from_secs(2)) {
            if signal == WatchSignal::Changed(Provider::Claude) {
                saw_claude = true;
                break;
            }
        }
    }
    assert!(saw_claude);
    watcher.shutdown();
}
```

The test writes only synthetic metadata, never a conversational record. The second test accepts a startup `WatchUnavailable(Codex)` signal before the valid Claude change; provider independence is the assertion.

- [x] **Step 2: Run focused tests to verify the watcher implementation is absent.**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher --offline
```

Expected: compilation fails because `WatchRoot`, `WatchSignal`, and `FileWatcher` are not implemented.

- [x] **Step 3: Add the bounded native Win32 handle and FFI layer.**

Keep all declarations under `#[cfg(windows)]` in `src-tauri/src/sources/file_watcher.rs`. Use these constants and native calls; do not add a crate:

```rust
const FILE_LIST_DIRECTORY: u32 = 0x0001;
const FILE_SHARE_READ: u32 = 0x0001;
const FILE_SHARE_WRITE: u32 = 0x0002;
const FILE_SHARE_DELETE: u32 = 0x0004;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
const FILE_NOTIFY_CHANGE_FILE_NAME: u32 = 0x0000_0001;
const FILE_NOTIFY_CHANGE_DIR_NAME: u32 = 0x0000_0002;
const FILE_NOTIFY_CHANGE_SIZE: u32 = 0x0000_0008;
const FILE_NOTIFY_CHANGE_LAST_WRITE: u32 = 0x0000_0010;
const ERROR_IO_PENDING: u32 = 997;
const ERROR_OPERATION_ABORTED: u32 = 995;
const ERROR_NOTIFY_ENUM_DIR: u32 = 1022;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_FAILED: u32 = 0xffff_ffff;
const INFINITE: u32 = 0xffff_ffff;
```

Define `OVERLAPPED` with `#[repr(C)]`, an `OwnedHandle` wrapper that closes each valid handle exactly once, a shared manual-reset stop event, and an auto-reset event per outstanding directory read. Declare only the needed `kernel32` functions: `CreateEventW`, `SetEvent`, `CreateFileW`, `ReadDirectoryChangesW`, `WaitForMultipleObjects`, `GetOverlappedResult`, `CancelIoEx`, `CloseHandle`, and `GetLastError`. Convert `Path` to a nul-terminated UTF-16 buffer with `OsStrExt::encode_wide`; never log or serialize that buffer.

- [x] **Step 4: Implement one cancellable worker per existing root.**

Use `CreateFileW` with `FILE_LIST_DIRECTORY`, all three share flags, `OPEN_EXISTING`, `FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED`. For each read, use a fixed `[u8; 64 * 1024]`, recursive watch `TRUE`, and the four notify filters above. The worker sequence is:

```text
create directory handle and per-worker read event
issue ReadDirectoryChangesW
wait on [read event, shared stop event]
stop event: CancelIoEx, wait for GetOverlappedResult completion, exit without sending a signal
read event: GetOverlappedResult
success or ERROR_NOTIFY_ENUM_DIR: validate bounded buffer, send Changed(provider)
other error: send WatchUnavailable(provider), exit worker
repeat
```

Treat `ERROR_IO_PENDING` as normal overlapped behavior. Treat `ERROR_OPERATION_ABORTED` during shutdown as normal. Validate only the bounded `FILE_NOTIFY_INFORMATION` headers: each record must fit within the returned byte count, `next_entry_offset` must be four-byte aligned and remain within the buffer, and `file_name_length` must be even and fit within its record. Do not decode or forward the filename. If a record is invalid, the buffer has zero usable bytes, or the system reports `ERROR_NOTIFY_ENUM_DIR`, send `Changed(provider)` so reconciliation repairs state. A closed receiver ends the worker. No worker uses polling or sleeps.

- [x] **Step 5: Implement refresh and idempotent shutdown.**

`replace_roots` must set the shared stop event, let each worker call `CancelIoEx` and wait for its outstanding read to complete, join old workers, then replace the stop event and start new workers. `shutdown` performs the same cancellation/join sequence once, clears the worker list, and is idempotent. Missing or inaccessible roots are skipped after sending one path-free `WatchUnavailable(provider)`, while usable roots continue starting. The watcher `Drop` implementation calls `shutdown` so every native handle closes on all exit paths.

- [x] **Step 6: Run watcher tests and the existing provider/source checks.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib sources::file_watcher --offline
cargo test --manifest-path src-tauri/Cargo.toml --test source_discovery --test provider_readers --offline
```

Expected: native watcher tests pass on Windows, source discovery/provider tests stay green, and `Cargo.toml`/`Cargo.lock` remain unchanged.

- [x] **Step 7: Commit the native watcher.**

```powershell
git add src-tauri/src/sources/file_watcher.rs
git commit -m "feat: watch native provider roots"
```

### Task 3: Connect live loop to `AppState`, commit gating, reconciliation, and retry

**Files:**
- Modify: `src-tauri/src/app/live_collection.rs`
- Modify: `src-tauri/src/app/runtime.rs`
- Test: `src-tauri/src/app/live_collection.rs` (`#[cfg(test)]` module)
- Review: `src-tauri/tests/collection_core.rs`
- Review: `src-tauri/tests/runtime_integration.rs`

**Interfaces:**
- `AppState` becomes `Clone` by storing `Arc<Mutex<Option<Runtime>>>`; all existing `from_paths`, `unavailable`, `collect_once`, and `summary` semantics remain unchanged.
- `AppState::watch_roots(&self) -> Vec<WatchRoot>` resolves only existing native `.claude/projects` and `.codex/sessions` roots through the existing safe-root code. It omits only unavailable provider targets, returns an empty vector for an unavailable/poisoned runtime, and never exposes the vector to Tauri commands or React.
- `RuntimeBackend::new(state: AppState) -> RuntimeBackend` calls `state.collect_once(&WindowsClock::current())` and delegates `watch_roots` to the same state.
- `TauriSummaryPublisher::new(app: AppHandle) -> TauriSummaryPublisher` calls the existing `emit_usage_summary`; it publishes no report, source health internals, observations, or paths.
- `LiveCollectionLoop::new(backend, publisher, start: Instant, config: LiveCollectionConfig) -> LiveCollectionLoop` creates the loop with no immediate collection.
- `LiveCollectionLoop::on_signal(&mut self, signal: WatchSignal, now: Instant) -> bool` maps change/unavailable signals to the scheduler and returns `false` for `Shutdown`.
- `LiveCollectionLoop::process_due(&mut self, now: Instant) -> Option<CollectionReason>` runs one due backend attempt, publishes only on `Ok(CollectionReport)`, records success/failure in the scheduler, and returns the consumed reason for watcher refresh decisions.
- `LiveCollectionLoop::run(self, receiver: Receiver<WatchSignal>, watcher: FileWatcher)` blocks on the next scheduler deadline or channel signal; it has no busy loop and exits on `Shutdown` or a disconnected channel.

- [x] **Step 1: Write the failing loop tests with scripted backend and publisher.**

Add private test doubles implementing the exact internal interfaces:

```rust
struct RecordingPublisher {
    summaries: Vec<UsageSummary>,
}

impl SummaryPublisher for RecordingPublisher {
    fn publish(
        &mut self,
        summary: &UsageSummary,
    ) -> Result<(), SummaryEventError> {
        self.summaries.push(summary.clone());
        Ok(())
    }
}

struct FailingPublisher;

impl SummaryPublisher for FailingPublisher {
    fn publish(
        &mut self,
        _summary: &UsageSummary,
    ) -> Result<(), SummaryEventError> {
        Err(SummaryEventError::Emit)
    }
}

struct ScriptedBackend {
    attempts: usize,
    results: std::collections::VecDeque<Result<CollectionReport, RuntimeError>>,
}

impl CollectionBackend for ScriptedBackend {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
        self.attempts += 1;
        self.results
            .pop_front()
            .expect("test backend should have a scripted result")
    }

    fn watch_roots(&self) -> Vec<WatchRoot> {
        Vec::new()
    }
}

fn test_report(today_tokens: u64) -> CollectionReport {
    CollectionReport {
        summary: UsageSummary {
            state: UsageState::Active,
            provider: Some("Claude Code".to_owned()),
            current_session_tokens: Some(today_tokens),
            today_tokens,
            last_updated_at: Some("2026-01-01T00:00:00Z".to_owned()),
            source_health: Vec::new(),
        },
        accepted_event_count: 1,
        source_health: Vec::new(),
    }
}
```

Add these behavior tests:

```rust
#[test]
fn successful_attempt_publishes_only_post_commit_summary() {
    let start = Instant::now();
    let mut live = LiveCollectionLoop::new(
        ScriptedBackend {
            attempts: 0,
            results: std::collections::VecDeque::from([Ok(test_report(20))]),
        },
        RecordingPublisher { summaries: Vec::new() },
        start,
        test_config(),
    );
    live.scheduler.mark_changed(start);

    assert_eq!(
        live.process_due(start + Duration::from_millis(200)),
        Some(CollectionReason::Notification)
    );
    assert_eq!(live.publisher.summaries[0].today_tokens, 20);
    assert_eq!(live.backend.attempts, 1);
}

#[test]
fn publisher_failure_does_not_turn_committed_collection_into_retry() {
    let start = Instant::now();
    let mut live = LiveCollectionLoop::new(
        ScriptedBackend {
            attempts: 0,
            results: std::collections::VecDeque::from([Ok(test_report(20))]),
        },
        FailingPublisher,
        start,
        test_config(),
    );
    live.scheduler.mark_changed(start);

    assert_eq!(
        live.process_due(start + Duration::from_millis(200)),
        Some(CollectionReason::Notification)
    );
    assert_eq!(live.backend.attempts, 1);
    assert!(live
        .scheduler
        .take_due(start + Duration::from_secs(1))
        .is_none());
}

#[test]
fn failed_storage_attempt_publishes_nothing_and_retries_after_backoff() {
    let start = Instant::now();
    let mut live = LiveCollectionLoop::new(
        ScriptedBackend {
            attempts: 0,
            results: std::collections::VecDeque::from([
                Err(RuntimeError::Collection(CollectionError::Storage(
                    StorageError::Write,
                ))),
                Ok(test_report(30)),
            ]),
        },
        RecordingPublisher { summaries: Vec::new() },
        start,
        test_config(),
    );
    live.scheduler.mark_changed(start);

    live.process_due(start + Duration::from_millis(200));
    assert!(live.publisher.summaries.is_empty());
    assert!(live
        .process_due(start + Duration::from_millis(1_199))
        .is_none());

    assert_eq!(
        live.process_due(start + Duration::from_millis(1_200)),
        Some(CollectionReason::Retry)
    );
    assert_eq!(live.publisher.summaries[0].today_tokens, 30);
    assert_eq!(live.backend.attempts, 2);
}
```

The first test proves the publisher is downstream of the backend result. The second proves a storage error does not publish stale/uncommitted data and that the next attempt waits for the configured `1 s` backoff.

- [x] **Step 2: Add an actual `AppState` notification test for appended records.**

Use a temporary profile containing only synthetic Claude metadata, collect one complete record with `FixedClock`, append a second complete record, and run the loop's deterministic `process_due` after `WatchSignal::Changed(Provider::Claude)`:

```rust
let profile = write_profile_with_claude_record(20);
let database = tempfile::tempdir().expect("database directory should be created");
let state = AppState::from_paths(
    profile.path().to_path_buf(),
    &database.path().join("index.sqlite"),
    DiscoveryLimits::new(10, 10_000),
)
.expect("runtime should open");
let clock = FixedClock::new("2026-01-01T00:00:30Z", "2026-01-01");
assert_eq!(state.collect_once(&clock).unwrap().summary.today_tokens, 20);

append_claude_record(profile.path(), 10, "2026-01-01T00:00:01Z");
let start = Instant::now();
let mut live = LiveCollectionLoop::new(
    FixedClockBackend { state: state.clone(), clock, reports: Vec::new() },
    RecordingPublisher { summaries: Vec::new() },
    start,
    test_config(),
);
live.on_signal(WatchSignal::Changed(Provider::Claude), start);
live.process_due(start + Duration::from_millis(200));

assert_eq!(live.publisher.summaries[0].today_tokens, 30);
```

`FixedClockBackend` calls `state.collect_once(&clock)`, so the test proves the live loop reuses checkpointed incremental reading and does not add a second aggregation implementation. `write_profile_with_claude_record` and `append_claude_record` must write only the existing synthetic `message.usage.input_tokens`, `message.usage.output_tokens`, `sessionId`, `message.id`, and `timestamp` fields.

Add the deterministic backend used by the integration-shaped unit tests:

```rust
struct FixedClockBackend {
    state: AppState,
    clock: FixedClock,
    reports: Vec<CollectionReport>,
}

impl CollectionBackend for FixedClockBackend {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
        let report = self.state.collect_once(&self.clock)?;
        self.reports.push(report.clone());
        Ok(report)
    }

    fn watch_roots(&self) -> Vec<WatchRoot> {
        self.state.watch_roots()
    }
}
```

Define the test helpers in the same `#[cfg(test)]` module so the fixture is reproducible and contains no conversational content:

```rust
fn claude_record(event_key: &str, timestamp: &str, total: u64) -> String {
    let input_tokens = total / 2;
    format!(
        "{{\"message\":{{\"id\":\"{event_key}\",\"type\":\"message\",\"usage\":{{\"input_tokens\":{input_tokens},\"output_tokens\":{output_tokens}}}}},\"sessionId\":\"session-a\",\"timestamp\":\"{timestamp}\"}}\n",
        output_tokens = total - input_tokens
    )
}

fn write_profile_with_claude_record(total: u64) -> tempfile::TempDir {
    let profile = tempfile::tempdir().expect("profile should be created");
    let root = profile.path().join(r".claude\projects");
    std::fs::create_dir_all(&root).expect("Claude root should be created");
    std::fs::write(
        root.join("session.jsonl"),
        claude_record("event-1", "2026-01-01T00:00:00Z", total),
    )
    .expect("Claude fixture should be written");
    profile
}

fn append_claude_record(profile: &std::path::Path, total: u64, timestamp: &str) {
    use std::io::Write;

    let path = profile.join(r".claude\projects\session.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("Claude fixture should be opened");
    file.write_all(claude_record("event-2", timestamp, total).as_bytes())
        .expect("Claude fixture should be appended");
}
```

- [x] **Step 3: Add the missed-notification reconciliation test.**

Reuse the same `FixedClockBackend` helper, append the second record without calling `on_signal`, then call `process_due(start + Duration::from_secs(30))`. Assert the consumed reason is `Reconciliation`, the published `today_tokens` is `30`, and backend attempts equal `1`. This proves reconciliation deadline is sufficient even when the watcher sends no signal.

- [x] **Step 4: Add the partial-final-line live regression.**

Write a complete 20-token Claude record followed by the first half of a 10-token JSONL record. Run the initial `AppState::collect_once` and assert `20`. Append the remaining bytes, send `Changed(Provider::Claude)`, run the debounced attempt, and assert `30`. Also assert the backend's second report has `accepted_event_count == 1`; the incomplete record must not be counted before completion. Keep the existing `collection_core.rs::partial_write_completion_is_collected_on_the_next_scan` test unchanged.

- [x] **Step 5: Extend `AppState` with shared state and Rust-only watcher roots.**

Change the runtime storage shape without changing public collection behavior:

```rust
#[derive(Clone)]
pub struct AppState {
    runtime: Arc<Mutex<Option<Runtime>>>,
    fallback_summary: UsageSummary,
}
```

Add a private `Runtime::watch_roots()` that iterates exactly `Provider::Claude` and `Provider::Codex`, calls the existing `resolve_native_root`, and converts successful roots to `WatchRoot::new(provider, filesystem_path.to_path_buf())`. Add `pub(crate) AppState::watch_roots()` that returns that vector while holding the existing mutex; on lock poison, unavailable runtime, missing root, or invalid root it returns no target. Do not return relative or absolute paths from any command, event, `UsageSummary`, or diagnostic.

- [x] **Step 6: Implement backend, publisher, deterministic loop, and blocking worker loop.**

Production adapters must have this shape:

```rust
struct RuntimeBackend {
    state: AppState,
}

impl CollectionBackend for RuntimeBackend {
    fn collect(&mut self) -> Result<CollectionReport, RuntimeError> {
        self.state.collect_once(&WindowsClock::current())
    }

    fn watch_roots(&self) -> Vec<WatchRoot> {
        self.state.watch_roots()
    }
}

struct TauriSummaryPublisher {
    app: tauri::AppHandle,
}

impl SummaryPublisher for TauriSummaryPublisher {
    fn publish(
        &mut self,
        summary: &UsageSummary,
    ) -> Result<(), SummaryEventError> {
        emit_usage_summary(&self.app, summary)
    }
}
```

`process_due` must call `backend.collect()` first. On `Ok(report)`, call `publisher.publish(&report.summary)` and then `scheduler.record_success()`. If publish fails, keep the success state and emit only `summary_event:emit`; SQLite already committed. On `Err(_)`, call `scheduler.record_failure(now)` and do not call the publisher. The existing `CollectionCoordinator` has already changed its last summary to `Stale` for storage failure; do not manufacture or emit another summary.

The blocking loop must use `Receiver::recv_timeout` with `scheduler.next_deadline().saturating_duration_since(Instant::now())`. Handle signals as follows:

```rust
match receiver.recv_timeout(wait) {
    Ok(WatchSignal::Changed(_))
    | Ok(WatchSignal::WatchUnavailable(_)) => {
        self.scheduler.mark_changed(Instant::now());
    }
    Ok(WatchSignal::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
    Err(RecvTimeoutError::Timeout) => {}
}
```

After a `Reconciliation` attempt, call `watcher.replace_roots(self.backend.watch_roots())` so a source directory created after startup receives a watcher on the next reconciliation. After any loop exit, call `watcher.shutdown()` exactly once through the idempotent method.

- [x] **Step 7: Implement `LiveCollectionHandle` and deterministic shutdown tests.**

Start the collector worker with one `RuntimeBackend`, one `TauriSummaryPublisher`, one `FileWatcher`, and the receiver side of a channel. The handle stores the sender and join handle in `Mutex<Option<...>>`. `shutdown()` takes and sends `WatchSignal::Shutdown`, then takes and joins the worker. A second call must find both options empty and return without blocking or panicking.

Add this test with a test-only worker that waits on the receiver:

```rust
#[test]
fn shutdown_is_idempotent_and_joins_worker() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let joined = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let joined_by_worker = joined.clone();
    let worker = std::thread::spawn(move || {
        assert_eq!(receiver.recv().unwrap(), WatchSignal::Shutdown);
        joined_by_worker.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    let handle = LiveCollectionHandle::from_parts(sender, worker);

    handle.shutdown();
    handle.shutdown();

    assert!(joined.load(std::sync::atomic::Ordering::SeqCst));
}
```

`from_parts` is test-visible within the module; production uses `start_live_collection`. The test proves the future tray Quit action has a usable shutdown seam without implementing tray behavior now.

- [x] **Step 8: Run live-loop, runtime, collection, and privacy checks.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib app::live_collection --offline
cargo test --manifest-path src-tauri/Cargo.toml --test collection_core --test runtime_integration --offline
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
```

Expected: notification, reconciliation, partial-line, append, retry, shutdown, restart, rotation, provider-independence, stale-state, and privacy tests pass. `UsageSummary`, `CollectionReport`, SQLite schema, and diagnostics retain their current allow-listed fields.

- [x] **Step 9: Commit the runtime live loop.**

```powershell
git add src-tauri/src/app/live_collection.rs src-tauri/src/app/runtime.rs
git commit -m "feat: run live collection with reconciliation"
```

### Task 4: Wire Tauri setup and application exit to the live handle

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Review: `src-tauri/src/app/live_collection.rs`
- Test: `src-tauri/src/lib.rs` existing summary/privacy tests and the live handle tests
- Verify unchanged: `src-tauri/src/commands/usage_summary.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`

**Interfaces:**
- `run()` still performs the initial collection and emits the same `UsageSummary` event only after a successful initial commit.
- Tauri manages one `AppState` and one `LiveCollectionHandle`.
- The exit callback invokes `LiveCollectionHandle::shutdown()` on `tauri::RunEvent::Exit`; no exit interception, tray action, close behavior, or new command is introduced.

- [x] **Step 1: Capture the current setup sequence.**

Review `src-tauri/src/lib.rs` and preserve the current sequence: initialize `AppState`, manage it, run one `WindowsClock::current()` collection, and emit only a successful `CollectionReport`. Record that the current builder has no live handle or exit callback; no source code change is made in this step.

- [x] **Step 2: Start the live handle after the initial collection.**

Refactor only the setup closure to share the state and start the worker:

```rust
.setup(|app| {
    let state = app::runtime::initialize_from_app(app.handle());
    app.manage(state.clone());

    let managed = app.state::<app::runtime::AppState>();
    if let Ok(report) = managed.collect_once(&collection::WindowsClock::current()) {
        if commands::usage_summary::emit_usage_summary(app.handle(), &report.summary).is_err() {
            eprintln!("summary_event:emit");
        }
    }

    let live_handle =
        app::live_collection::start_live_collection(state, app.handle().clone());
    app.manage(live_handle);
    Ok(())
})
```

Starting after the initial pass keeps existing startup behavior and still has the independent 30-second reconciliation for a change occurring during worker startup. Do not emit an event from the worker before its first notification/reconciliation deadline.

- [x] **Step 3: Add the exit shutdown callback.**

Replace the current shorthand `Builder::run` call with the explicit `build(...).run(callback)` form. Tauri 2's callback belongs to `App::run`, while `Builder::run` accepts no callback. Use this exact shape:

```rust
tauri::Builder::default()
    // keep the existing setup, invoke_handler, and all preceding builder calls
    .build(tauri::generate_context!())
    .expect("error while building token tracing widget")
    .run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app_handle
                .state::<app::live_collection::LiveCollectionHandle>()
                .shutdown();
        }
    });
```

Use the Tauri 2 `RunEvent::Exit` callback. Do not prevent exit, intercept `ExitRequested`, add a tray menu, or change the existing window close behavior. Keep the existing `.setup(...)` and `.invoke_handler(...)` calls before `.build(...)`; the snippet shows only the terminal replacement.

- [x] **Step 4: Verify startup and shutdown compile without widening capabilities.**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
```

Expected: managed `AppState`/`LiveCollectionHandle`, initial command/event, exit callback, and all privacy tests compile and pass. No Tauri capability or frontend contract changes occur.

- [x] **Step 5: Commit Tauri live-loop wiring.**

```powershell
git add src-tauri/src/lib.rs
git commit -m "feat: start live collector with tauri"
```

### Task 5: Run full gates and Windows live-collection smoke verification

**Files:**
- Review: `src-tauri/src/app/live_collection.rs`
- Review: `src-tauri/src/app/runtime.rs`
- Review: `src-tauri/src/sources/file_watcher.rs`
- Review: `src-tauri/src/lib.rs`
- Verify unchanged: `src-tauri/src/collection/mod.rs`, `src-tauri/src/commands/usage_summary.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src/App.tsx`, `src/lib/usage-summary.ts`

**Interfaces:**
- The released app remains one Tauri executable with one in-process worker set; no app-managed sidecar or service exists.
- The only live event payload remains the existing `UsageSummary` under `usage-summary-changed`.
- Native watcher signals carry only `Provider`; source paths stay inside Rust.

- [x] **Step 1: Run all repository automated checks.**

Run:

```powershell
git diff --check
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --offline
cargo check --manifest-path src-tauri/Cargo.toml --offline
```

Expected: existing 10 frontend tests plus all Rust tests pass, TypeScript/Vite build succeeds, no whitespace errors, and no new dependency appears. If the environment reports the known esbuild `spawn EPERM`, rerun the exact frontend command in the approved elevated execution context; do not alter application code for that environment failure.

- [x] **Step 2: Build the integrated release executable.**

Run:

```powershell
npm run tauri build -- --no-bundle
```

Expected: `src-tauri/target/release/token-tracing-widget.exe` is produced from the updated frontend and Rust code. The artifact remains one executable with no app-managed sidecar or background service process.

- [ ] **Step 3: Verify live notification behavior on Windows 11.** (blocked: isolated smoke instance was not targetable through the available CUA surface; automated watcher/loop coverage passed)

Launch the release executable and verify:

- Initial summary still appears through the existing command/event contract.
- Appending one valid record under an already existing native Claude or Codex source updates the overlay after one coalesced notification window, without manual refresh.
- Several writes in one short burst produce one visible update cycle, not one collection per raw notification.
- A partial final JSONL line does not change totals until its remaining bytes complete the line; the completion is picked up by the next notification/reconciliation pass.
- A missed notification is repaired by the 30-second reconciliation; no busy loop is visible while files are idle.
- A source root that is absent or blocked does not stop the other provider or prevent the overlay from remaining alive.
- A storage failure leaves the last totals with stale semantics and causes bounded retries; no uncommitted summary appears.
- The process exits cleanly after `RunEvent::Exit`; no watcher thread or native directory handle remains.

Use only synthetic metadata if a manual fixture is needed. Do not place prompts, responses, reasoning, tool payloads, credentials, repository paths, working directories, raw JSON, or absolute source paths in the fixture or visual output.

- [x] **Step 4: Review privacy and scope boundaries.**

Inspect the final diff and confirm:

```powershell
git status --short --branch
git diff --name-only 23cc0a7..HEAD
git diff -- src-tauri/Cargo.toml src-tauri/Cargo.lock
```

Expected tracked implementation changes are limited to the live app/runtime module, the native watcher, and Tauri setup wiring. `Cargo.toml` and `Cargo.lock` have no changes. No React, tray, Settings, startup, window, capabilities, provider-parser, schema, or frontend state changes appear. `WatchSignal`, `UsageSummary`, diagnostics, SQLite rows, and logs contain no raw source path or private provider content.

- [x] **Step 5: Record completion without merging to `main`.**

Leave the work on `dev`. Do not reset, merge, push, or modify `main`; integration remains a separate user-approved action after the live slice passes its gates.

## Acceptance criteria for this slice

1. Native Claude and Codex roots receive filesystem notifications through an in-process Windows watcher without a new watcher dependency.
2. Notification bursts are coalesced within `200 ms`; idle sources do not cause a continuous busy loop.
3. A separate `30 s` reconciliation catches missed notifications and refreshes watcher roots without postponement from notification-driven collections.
4. Existing bounded discovery, checkpoints, provider readers, cumulative delta conversion, deduplication, SQLite transaction, and summary aggregation remain the only data path.
5. `usage-summary-changed` is emitted only after `CollectionCoordinator::collect` returns a post-commit `CollectionReport`; storage failures publish nothing and retry with capped backoff.
6. Partial final lines, appends, restart, truncation/rotation, duplicate scans, cumulative resets, and independent provider failures remain correct through the existing Rust tests plus live notification/reconciliation tests.
7. Cancellation is explicit, idempotent, joins worker threads, and closes native directory handles before application exit.
8. `UsageSummary`, watcher signals, SQLite, diagnostics, and logs contain no prompts, responses, reasoning, tool payloads, credentials, repository contents, working directories, raw JSON, or absolute source paths.
9. Frontend tests/build, Rust format/tests/check, integrated Tauri release build, and Windows 11 live smoke verification pass.

## Explicitly deferred

The following remain later approved slices: tray Show/Hide/Settings/Quit, close-to-hide ownership, single-instance enforcement, launch-on-login, settings persistence, explicit WSL UNC selection, source-root recovery UX, clear-index confirmation, backup/rebuild, remembered position, opacity/always-on-top settings, installer, and clean uninstall.

## Plan self-review

### Spec coverage

- Source discovery boundary remains the current safe native roots; watcher refresh never scans or serializes arbitrary directories.
- Provider adapters and collection core remain unchanged call sites; live triggers only invoke the existing coordinator.
- Storage keeps one event/checkpoint transaction and computes summaries only after commit.
- Presentation keeps the existing typed command/event contract and adds no frontend polling.
- Error handling covers missing/blocked roots, watcher failure, malformed/partial records through existing coordinator behavior, SQLite stale state, bounded retry, and cancellation.
- Security/privacy checks cover Rust-only filesystem access, path-free signals, typed summaries, no network, no telemetry, and no new capability.
- Test coverage includes deterministic scheduler timing, native watcher delivery, coalescing, append, reconciliation, partial line completion, provider independence, retry, commit gating, restart/rotation regressions, privacy allow-list, full build, and Windows smoke behavior.
- Tray, Settings, explicit WSL roots, startup registration, position, installer, and recovery behavior are explicitly outside this plan.

### Placeholder and consistency check

Every task names concrete files, internal interfaces, constants, test bodies, commands, expected outcomes, and commit boundaries. Names stay consistent: `WatchSignal`, `WatchRoot`, `LiveScheduler`, `CollectionReason`, `CollectionBackend`, `SummaryPublisher`, `LiveCollectionLoop`, `RuntimeBackend`, `TauriSummaryPublisher`, and `LiveCollectionHandle`. No task introduces a second summary path or a new frontend contract.
