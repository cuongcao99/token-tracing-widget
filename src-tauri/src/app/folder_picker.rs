//! Native folder selection for source-root configuration.

use std::path::{Path, PathBuf};

pub(crate) fn pick_folder(title: &str, initial: Option<&Path>) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    {
        let title = title.to_owned();
        let initial = initial.map(Path::to_path_buf);
        std::thread::spawn(move || pick_windows_folder(&title, initial.as_deref()))
            .join()
            .map_err(|_| "source_root_open".to_owned())?
    }

    #[cfg(not(windows))]
    {
        let _ = (title, initial);
        Err("source_root_open".to_owned())
    }
}

#[cfg(windows)]
fn pick_windows_folder(title: &str, initial: Option<&Path>) -> Result<Option<PathBuf>, String> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    use windows::core::{HRESULT, PCWSTR};
    use windows::Win32::Foundation::ERROR_CANCELLED;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx,
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileDialog, IShellItem, SHCreateItemFromParsingName, FOS_FORCEFILESYSTEM,
        FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    struct ComApartment;

    impl Drop for ComApartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn path_wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() {
        return Err("source_root_open".to_owned());
    }
    let _com_apartment = ComApartment;

    let dialog: IFileDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
            .map_err(|_| "source_root_open".to_owned())?;

    let options = unsafe { dialog.GetOptions() }.map_err(|_| "source_root_open".to_owned())?;
    unsafe {
        dialog.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)
    }
    .map_err(|_| "source_root_open".to_owned())?;

    let title_wide = wide_null(title);
    unsafe { dialog.SetTitle(PCWSTR(title_wide.as_ptr())) }
        .map_err(|_| "source_root_open".to_owned())?;

    if let Some(initial) = initial {
        let initial_wide = path_wide(initial);
        let item: windows::core::Result<IShellItem> = unsafe {
            SHCreateItemFromParsingName(PCWSTR(initial_wide.as_ptr()), None::<&IBindCtx>)
        };
        if let Ok(item) = item {
            let _ = unsafe { dialog.SetFolder(&item) };
        }
    }

    let show_result = unsafe { dialog.Show(None) };
    match show_result {
        Ok(()) => {}
        Err(error) if error.code().0 == HRESULT::from_win32(ERROR_CANCELLED.0).0 => {
            return Ok(None);
        }
        Err(_) => return Err("source_root_open".to_owned()),
    }

    let item = unsafe { dialog.GetResult() }.map_err(|_| "source_root_open".to_owned())?;
    let display_name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
        .map_err(|_| "source_root_open".to_owned())?;
    if display_name.is_null() {
        return Err("source_root_open".to_owned());
    }

    let path = unsafe { display_name.to_string() };
    unsafe {
        CoTaskMemFree(Some(display_name.as_ptr() as *const c_void));
    }
    let path = path.map_err(|_| "source_root_open".to_owned())?;

    Ok(Some(PathBuf::from(path)))
}
