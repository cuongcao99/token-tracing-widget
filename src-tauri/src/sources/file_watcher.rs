//! Change notifications and reconciliation for source files.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::types::provider::Provider;

#[derive(Debug, Clone)]
pub(crate) struct WatchRoot {
    provider: Provider,
    path: PathBuf,
}

impl WatchRoot {
    pub(crate) fn new(provider: Provider, path: PathBuf) -> Self {
        Self { provider, path }
    }

    pub(crate) fn provider(&self) -> Provider {
        self.provider
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchSignal {
    Changed(Provider),
    WatchUnavailable(Provider),
    ConfigurationChanged,
    Shutdown,
}

pub(crate) struct FileWatcher {
    sender: Sender<WatchSignal>,
    #[cfg(windows)]
    stop_signal: Option<std::sync::Arc<win32::StopSignal>>,
    #[cfg(windows)]
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl FileWatcher {
    pub(crate) fn start(roots: Vec<WatchRoot>, sender: Sender<WatchSignal>) -> Self {
        let mut watcher = Self {
            sender,
            #[cfg(windows)]
            stop_signal: None,
            #[cfg(windows)]
            workers: Vec::new(),
        };

        #[cfg(windows)]
        watcher.start_workers(roots);
        #[cfg(not(windows))]
        let _ = roots;

        watcher
    }

    pub(crate) fn replace_roots(&mut self, roots: Vec<WatchRoot>) {
        #[cfg(windows)]
        {
            self.stop_workers();
            self.start_workers(roots);
        }
        #[cfg(not(windows))]
        let _ = roots;
    }

    pub(crate) fn shutdown(&mut self) {
        #[cfg(windows)]
        self.stop_workers();
    }

    #[cfg(windows)]
    fn start_workers(&mut self, roots: Vec<WatchRoot>) {
        if roots.is_empty() {
            return;
        }

        let Some(stop_signal) = win32::StopSignal::new() else {
            for root in roots {
                self.send(WatchSignal::WatchUnavailable(root.provider()));
            }
            return;
        };
        self.stop_signal = Some(stop_signal.clone());

        for root in roots {
            let sender = self.sender.clone();
            let stop_signal = stop_signal.clone();
            self.workers.push(std::thread::spawn(move || {
                win32::watch_root(root, sender, stop_signal);
            }));
        }
    }

    #[cfg(windows)]
    fn stop_workers(&mut self) {
        if let Some(stop_signal) = self.stop_signal.take() {
            stop_signal.request();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }

    fn send(&self, signal: WatchSignal) {
        let _ = self.sender.send(signal);
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(windows)]
mod win32 {
    use std::ffi::c_void;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use super::{WatchRoot, WatchSignal};

    type Handle = *mut c_void;

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
    const TRUE: i32 = 1;
    const FALSE: i32 = 0;
    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

    #[repr(C)]
    struct Overlapped {
        internal: usize,
        internal_high: usize,
        offset: u32,
        offset_high: u32,
        h_event: Handle,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CancelIoEx(file: Handle, overlapped: *mut Overlapped) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn CreateEventW(
            security_attributes: *const c_void,
            manual_reset: i32,
            initial_state: i32,
            name: *const u16,
        ) -> Handle;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn GetLastError() -> u32;
        fn GetOverlappedResult(
            file: Handle,
            overlapped: *mut Overlapped,
            transferred: *mut u32,
            wait: i32,
        ) -> i32;
        fn ReadDirectoryChangesW(
            directory: Handle,
            buffer: *mut c_void,
            buffer_length: u32,
            watch_subtree: i32,
            notify_filter: u32,
            transferred: *mut u32,
            overlapped: *mut Overlapped,
            completion_routine: *mut c_void,
        ) -> i32;
        fn SetEvent(event: Handle) -> i32;
        fn WaitForMultipleObjects(
            count: u32,
            handles: *const Handle,
            wait_all: i32,
            milliseconds: u32,
        ) -> u32;
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
            // The handle is owned by this wrapper and is closed after all waiters joined.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub(super) struct StopSignal {
        event: OwnedHandle,
        requested: AtomicBool,
    }

    // Win32 event handles are process-wide kernel objects safe to wait on from worker threads.
    unsafe impl Send for StopSignal {}
    unsafe impl Sync for StopSignal {}

    impl StopSignal {
        pub(super) fn new() -> Option<Arc<Self>> {
            let event = unsafe { CreateEventW(ptr::null(), TRUE, FALSE, ptr::null()) };
            Some(Arc::new(Self {
                event: OwnedHandle::new(event)?,
                requested: AtomicBool::new(false),
            }))
        }

        pub(super) fn raw(&self) -> Handle {
            self.event.raw()
        }

        pub(super) fn request(&self) {
            self.requested.store(true, Ordering::Release);
            unsafe {
                let _ = SetEvent(self.raw());
            }
        }

        fn is_requested(&self) -> bool {
            self.requested.load(Ordering::Acquire)
        }
    }

    pub(super) fn watch_root(
        root: WatchRoot,
        sender: std::sync::mpsc::Sender<WatchSignal>,
        stop_signal: Arc<StopSignal>,
    ) {
        let path = path_to_utf16(root.path());
        let directory = unsafe {
            CreateFileW(
                path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                INVALID_HANDLE_VALUE,
            )
        };
        let Some(directory) = OwnedHandle::new(directory) else {
            send(&sender, WatchSignal::WatchUnavailable(root.provider()));
            return;
        };

        let read_event = unsafe { CreateEventW(ptr::null(), FALSE, FALSE, ptr::null()) };
        let Some(read_event) = OwnedHandle::new(read_event) else {
            send(&sender, WatchSignal::WatchUnavailable(root.provider()));
            return;
        };

        let mut buffer = [0_u8; 64 * 1024];
        loop {
            if stop_signal.is_requested() {
                return;
            }

            let mut overlapped: Overlapped = unsafe { mem::zeroed() };
            overlapped.h_event = read_event.raw();
            let issued = unsafe {
                ReadDirectoryChangesW(
                    directory.raw(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    TRUE,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_SIZE
                        | FILE_NOTIFY_CHANGE_LAST_WRITE,
                    ptr::null_mut(),
                    &mut overlapped,
                    ptr::null_mut(),
                )
            };
            if issued == FALSE {
                let error = unsafe { GetLastError() };
                if error == ERROR_IO_PENDING {
                    // The read is outstanding and will complete through the read event.
                } else if error == ERROR_OPERATION_ABORTED || stop_signal.is_requested() {
                    return;
                } else {
                    send(&sender, WatchSignal::WatchUnavailable(root.provider()));
                    return;
                }
            }

            let handles = [read_event.raw(), stop_signal.raw()];
            let wait_result = unsafe {
                WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), FALSE, INFINITE)
            };
            if wait_result == WAIT_OBJECT_0 + 1 {
                cancel_and_wait(directory.raw(), &mut overlapped);
                return;
            }
            if wait_result == WAIT_FAILED {
                if !stop_signal.is_requested() {
                    send(&sender, WatchSignal::WatchUnavailable(root.provider()));
                }
                return;
            }
            if wait_result != WAIT_OBJECT_0 {
                if !stop_signal.is_requested() {
                    send(&sender, WatchSignal::WatchUnavailable(root.provider()));
                }
                return;
            }

            let mut bytes_transferred = 0_u32;
            let completed = unsafe {
                GetOverlappedResult(
                    directory.raw(),
                    &mut overlapped,
                    &mut bytes_transferred,
                    FALSE,
                )
            };
            if completed == FALSE {
                let error = unsafe { GetLastError() };
                if error == ERROR_NOTIFY_ENUM_DIR {
                    if !send(&sender, WatchSignal::Changed(root.provider())) {
                        return;
                    }
                    continue;
                }
                if error == ERROR_OPERATION_ABORTED || stop_signal.is_requested() {
                    return;
                }
                send(&sender, WatchSignal::WatchUnavailable(root.provider()));
                return;
            }

            let _well_formed =
                notification_buffer_is_well_formed(&buffer, bytes_transferred as usize);
            if !send(&sender, WatchSignal::Changed(root.provider())) {
                return;
            }
        }
    }

    fn path_to_utf16(path: &std::path::Path) -> Vec<u16> {
        let mut encoded: Vec<u16> = path.as_os_str().encode_wide().collect();
        encoded.push(0);
        encoded
    }

    fn cancel_and_wait(directory: Handle, overlapped: &mut Overlapped) {
        unsafe {
            let _ = CancelIoEx(directory, overlapped);
            let mut bytes_transferred = 0_u32;
            let _ = GetOverlappedResult(directory, overlapped, &mut bytes_transferred, TRUE);
        }
    }

    fn send(sender: &std::sync::mpsc::Sender<WatchSignal>, signal: WatchSignal) -> bool {
        sender.send(signal).is_ok()
    }

    fn notification_buffer_is_well_formed(buffer: &[u8], bytes: usize) -> bool {
        if bytes == 0 || bytes > buffer.len() {
            return false;
        }

        let mut offset = 0_usize;
        while offset < bytes {
            let available = bytes - offset;
            if available < 12 {
                return false;
            }

            let next_entry_offset = u32::from_ne_bytes(
                buffer[offset..offset + 4]
                    .try_into()
                    .expect("bounded notification header"),
            ) as usize;
            let file_name_length = u32::from_ne_bytes(
                buffer[offset + 8..offset + 12]
                    .try_into()
                    .expect("bounded notification header"),
            ) as usize;
            if file_name_length % 2 != 0 {
                return false;
            }

            let record_length = if next_entry_offset == 0 {
                available
            } else {
                if next_entry_offset < 12
                    || next_entry_offset % 4 != 0
                    || next_entry_offset > available
                {
                    return false;
                }
                next_entry_offset
            };
            if 12 + file_name_length > record_length {
                return false;
            }
            if next_entry_offset == 0 {
                return true;
            }
            offset += next_entry_offset;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::{FileWatcher, WatchRoot, WatchSignal};
    use crate::types::provider::Provider;

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
}
