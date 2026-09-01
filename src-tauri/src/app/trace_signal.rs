//! Provider-hook lifecycle ingress.

use std::ffi::OsStr;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc};
use std::thread::{self, JoinHandle};

use serde_json::Value;

use crate::sources::file_watcher::WatchSignal;
use crate::types::provider::Provider;
use crate::types::trace_signal::{
    ProviderEvent, TraceLifecycle, TraceSignal, MAX_OPAQUE_ID_BYTES, TRACE_SIGNAL_SCHEMA_VERSION,
};
use crate::utils::windows_time::current_utc_timestamp;

pub(crate) const HOOK_ARGUMENT: &str = "--hook";
pub(crate) const MAX_HOOK_INPUT_BYTES: usize = 64 * 1024;
pub(crate) const TRACE_PIPE_NAME: &str = r"\\.\pipe\token-tracing-widget-trace-v1";

/// Projects provider stdin into the allow-listed lifecycle contract.
///
/// The input is intentionally accepted as an untyped JSON value. Provider
/// hook payloads contain sensitive fields and are not the app's wire format;
/// only the small projection below is copied into the validated signal.
pub(crate) fn project_hook_payload(
    provider: Provider,
    input: &[u8],
    observed_at: &str,
) -> Option<TraceSignal> {
    if input.len() > MAX_HOOK_INPUT_BYTES {
        return None;
    }

    let value: Value = serde_json::from_slice(input).ok()?;
    let object = value.as_object()?;
    let provider_event_name = object.get("hook_event_name")?.as_str()?;
    let (provider_event, lifecycle) = map_provider_event(provider, provider_event_name)?;

    Some(TraceSignal {
        schema_version: TRACE_SIGNAL_SCHEMA_VERSION,
        provider,
        lifecycle,
        provider_event,
        observed_at: observed_at.to_owned(),
        opaque_session_id: bounded_opaque_id(object.get("session_id")),
        opaque_turn_id: bounded_opaque_id(object.get("turn_id")),
        sequence: None,
    })
}

fn map_provider_event(provider: Provider, event: &str) -> Option<(ProviderEvent, TraceLifecycle)> {
    match (provider, event) {
        (Provider::Claude, "UserPromptSubmit") => Some((
            ProviderEvent::UserPromptSubmit,
            TraceLifecycle::StartOrContinue,
        )),
        (Provider::Claude, "Stop") => Some((ProviderEvent::Stop, TraceLifecycle::Pause)),
        (Provider::Claude, "StopFailure") => {
            Some((ProviderEvent::StopFailure, TraceLifecycle::Pause))
        }
        (Provider::Claude, "SessionEnd") => Some((ProviderEvent::SessionEnd, TraceLifecycle::Stop)),
        (Provider::Codex, "SessionStart") => {
            Some((ProviderEvent::SessionStart, TraceLifecycle::StartOrContinue))
        }
        (Provider::Codex, "UserPromptSubmit") => Some((
            ProviderEvent::UserPromptSubmit,
            TraceLifecycle::StartOrContinue,
        )),
        (Provider::Codex, "Stop") => Some((ProviderEvent::Stop, TraceLifecycle::Pause)),
        (Provider::Codex, "SessionEnd") => Some((ProviderEvent::SessionEnd, TraceLifecycle::Stop)),
        _ => None,
    }
}

fn bounded_opaque_id(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?;
    if value.is_empty()
        || value.len() > MAX_OPAQUE_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return None;
    }
    Some(value.to_owned())
}

/// Returns `true` when the process was invoked in hook mode.
pub fn run_hook_mode() -> bool {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(OsStr::new(HOOK_ARGUMENT)) {
        return false;
    }

    let Some(provider) = arguments
        .next()
        .and_then(|value| value.to_str().and_then(Provider::from_str))
    else {
        return true;
    };

    let mut input = Vec::with_capacity(MAX_HOOK_INPUT_BYTES.min(4096));
    let read_result = std::io::stdin()
        .take((MAX_HOOK_INPUT_BYTES as u64).saturating_add(1))
        .read_to_end(&mut input);
    if read_result.is_err() || input.len() > MAX_HOOK_INPUT_BYTES {
        return true;
    }

    if let Some(signal) = project_hook_payload(provider, &input, &current_utc_timestamp()) {
        let _ = send_signal(&signal);
    }

    true
}

pub(crate) struct HookListener {
    #[cfg(windows)]
    stop: Arc<AtomicBool>,
    #[cfg(windows)]
    worker: Option<JoinHandle<()>>,
}

impl HookListener {
    pub(crate) fn start(sender: Sender<WatchSignal>) -> Self {
        #[cfg(windows)]
        {
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = stop.clone();
            let worker = thread::Builder::new()
                .name("trace-hook-listener".to_owned())
                .spawn(move || listen_loop(sender, worker_stop))
                .expect("trace hook listener should start");
            return Self {
                stop,
                worker: Some(worker),
            };
        }

        #[cfg(not(windows))]
        {
            let _ = sender;
            Self {}
        }
    }

    pub(crate) fn shutdown(&mut self) {
        #[cfg(windows)]
        {
            self.stop.store(true, Ordering::Release);
            poke_pipe();
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for HookListener {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(windows)]
fn listen_loop(sender: Sender<WatchSignal>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        let Some(pipe) = windows_pipe::ServerPipe::create() else {
            return;
        };
        if !pipe.connect() {
            continue;
        }
        let payload = pipe.read_bounded();
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Some(payload) = payload else {
            continue;
        };
        let Ok(signal) = serde_json::from_slice::<TraceSignal>(&payload) else {
            continue;
        };
        let _ = sender.send(WatchSignal::Trace(signal));
    }
}

fn send_signal(signal: &TraceSignal) -> bool {
    #[cfg(windows)]
    {
        return windows_pipe::send(signal);
    }

    #[cfg(not(windows))]
    {
        let _ = signal;
        false
    }
}

#[cfg(windows)]
fn poke_pipe() {
    windows_pipe::poke();
}

#[cfg(windows)]
mod windows_pipe {
    use std::ffi::{c_void, OsStr};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use super::{TraceSignal, MAX_HOOK_INPUT_BYTES, TRACE_PIPE_NAME};

    type Handle = *mut c_void;

    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
    const ERROR_MORE_DATA: u32 = 234;
    const ERROR_PIPE_CONNECTED: u32 = 535;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const OPEN_EXISTING: u32 = 3;
    const PIPE_ACCESS_INBOUND: u32 = 0x0000_0001;
    const PIPE_TYPE_MESSAGE: u32 = 0x0000_0004;
    const PIPE_READMODE_MESSAGE: u32 = 0x0000_0002;
    const PIPE_WAIT: u32 = 0x0000_0000;
    const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x0000_0008;
    const PIPE_UNLIMITED_INSTANCES: u32 = 255;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CloseHandle(object: Handle) -> i32;
        fn ConnectNamedPipe(pipe: Handle, overlapped: *mut c_void) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer_size: u32,
            in_buffer_size: u32,
            default_timeout: u32,
            security_attributes: *const c_void,
        ) -> Handle;
        fn DisconnectNamedPipe(pipe: Handle) -> i32;
        fn GetLastError() -> u32;
        fn ReadFile(
            file: Handle,
            buffer: *mut c_void,
            bytes_to_read: u32,
            bytes_read: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn WriteFile(
            file: Handle,
            buffer: *const c_void,
            bytes_to_write: u32,
            bytes_written: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }

    fn pipe_name() -> Vec<u16> {
        OsStr::new(TRACE_PIPE_NAME)
            .encode_wide()
            .chain(Some(0))
            .collect()
    }

    struct OwnedHandle(Handle);

    impl OwnedHandle {
        fn new(handle: Handle) -> Option<Self> {
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                None
            } else {
                Some(Self(handle))
            }
        }

        fn raw(&self) -> Handle {
            self.0
        }
    }

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub(super) struct ServerPipe {
        handle: OwnedHandle,
    }

    impl ServerPipe {
        pub(super) fn create() -> Option<Self> {
            let name = pipe_name();
            let handle = unsafe {
                CreateNamedPipeW(
                    name.as_ptr(),
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_MESSAGE
                        | PIPE_READMODE_MESSAGE
                        | PIPE_WAIT
                        | PIPE_REJECT_REMOTE_CLIENTS,
                    PIPE_UNLIMITED_INSTANCES,
                    MAX_HOOK_INPUT_BYTES as u32,
                    MAX_HOOK_INPUT_BYTES as u32,
                    0,
                    ptr::null(),
                )
            };
            Some(Self {
                handle: OwnedHandle::new(handle)?,
            })
        }

        pub(super) fn connect(&self) -> bool {
            let connected = unsafe { ConnectNamedPipe(self.handle.raw(), ptr::null_mut()) } != 0;
            connected || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED
        }

        pub(super) fn read_bounded(&self) -> Option<Vec<u8>> {
            let mut buffer = vec![0_u8; MAX_HOOK_INPUT_BYTES + 1];
            let mut bytes_read = 0_u32;
            let result = unsafe {
                ReadFile(
                    self.handle.raw(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut bytes_read,
                    ptr::null_mut(),
                )
            };
            if result == 0 && unsafe { GetLastError() } != ERROR_MORE_DATA {
                return None;
            }
            let bytes_read = usize::try_from(bytes_read).ok()?;
            if bytes_read > MAX_HOOK_INPUT_BYTES {
                return None;
            }
            buffer.truncate(bytes_read);
            Some(buffer)
        }
    }

    impl Drop for ServerPipe {
        fn drop(&mut self) {
            unsafe {
                let _ = DisconnectNamedPipe(self.handle.raw());
            }
        }
    }

    pub(super) fn send(signal: &TraceSignal) -> bool {
        let payload = match serde_json::to_vec(signal) {
            Ok(payload) if payload.len() <= MAX_HOOK_INPUT_BYTES => payload,
            _ => return false,
        };
        let name = pipe_name();
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        let Some(handle) = OwnedHandle::new(handle) else {
            return false;
        };
        let mut bytes_written = 0_u32;
        unsafe {
            WriteFile(
                handle.raw(),
                payload.as_ptr().cast(),
                payload.len() as u32,
                &mut bytes_written,
                ptr::null_mut(),
            ) != 0
                && bytes_written as usize == payload.len()
        }
    }

    pub(super) fn poke() {
        let name = pipe_name();
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        let _ = OwnedHandle::new(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::{project_hook_payload, MAX_HOOK_INPUT_BYTES};
    use crate::types::provider::Provider;
    use crate::types::trace_signal::{ProviderEvent, TraceLifecycle};

    #[test]
    fn projection_discards_sensitive_hook_fields() {
        let input = br#"{
            "hook_event_name":"UserPromptSubmit",
            "session_id":"session-001",
            "turn_id":"turn-001",
            "prompt":"private prompt",
            "transcript_path":"C:/private/transcript.jsonl",
            "cwd":"C:/private/repository",
            "tool_input":{"command":"private command"}
        }"#;

        let signal = project_hook_payload(Provider::Claude, input, "2026-09-01T10:00:00Z")
            .expect("supported hook should project");
        assert_eq!(signal.provider_event, ProviderEvent::UserPromptSubmit);
        assert_eq!(signal.lifecycle, TraceLifecycle::StartOrContinue);
        let serialized = serde_json::to_string(&signal).unwrap();
        assert!(!serialized.contains("private"));
        assert!(!serialized.contains("transcript"));
        assert!(!serialized.contains("cwd"));
        assert!(!serialized.contains("tool_input"));
    }

    #[test]
    fn projection_maps_stop_and_session_end_without_token_data() {
        let stop = project_hook_payload(
            Provider::Codex,
            br#"{"hook_event_name":"Stop","session_id":"session-001","turn_id":"turn-001"}"#,
            "2026-09-01T10:00:00Z",
        )
        .unwrap();
        assert_eq!(stop.lifecycle, TraceLifecycle::Pause);
        assert_eq!(stop.provider_event, ProviderEvent::Stop);

        let end = project_hook_payload(
            Provider::Codex,
            br#"{"hook_event_name":"SessionEnd","session_id":"session-001"}"#,
            "2026-09-01T10:00:00Z",
        )
        .unwrap();
        assert_eq!(end.lifecycle, TraceLifecycle::Stop);
        assert_eq!(end.provider_event, ProviderEvent::SessionEnd);
    }

    #[test]
    fn projection_ignores_unsupported_events_and_oversized_input() {
        assert!(project_hook_payload(
            Provider::Claude,
            br#"{"hook_event_name":"PreToolUse","prompt":"private"}"#,
            "2026-09-01T10:00:00Z",
        )
        .is_none());
        assert!(project_hook_payload(
            Provider::Claude,
            &vec![b'x'; MAX_HOOK_INPUT_BYTES + 1],
            "2026-09-01T10:00:00Z",
        )
        .is_none());
    }

    #[test]
    fn invalid_opaque_ids_are_not_forwarded() {
        let signal = project_hook_payload(
            Provider::Claude,
            br#"{"hook_event_name":"UserPromptSubmit","session_id":"C:/private/path","turn_id":"turn-1"}"#,
            "2026-09-01T10:00:00Z",
        )
        .unwrap();
        assert!(signal.opaque_session_id.is_none());
        assert_eq!(signal.opaque_turn_id.as_deref(), Some("turn-1"));
    }
}
