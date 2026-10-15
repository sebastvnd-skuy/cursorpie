//! "Launch at login" through the per-user `Run` registry key.

use super::util::{run_value, wide};
use crate::about::APP_NAME;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};

/// The key every per-user startup entry lives in.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Opens the `Run` key with `access`.
fn open(access: u32) -> Option<HKEY> {
    let mut key: HKEY = std::ptr::null_mut();
    let path = wide(RUN_KEY);
    // SAFETY: `key` is a valid out-pointer and `path` is zero terminated.
    let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &mut key) };
    (r == ERROR_SUCCESS).then_some(key)
}

/// Whether CursorPie starts with Windows.
pub fn launch_at_login_enabled() -> bool {
    let Some(key) = open(KEY_QUERY_VALUE) else {
        return false;
    };
    let name = wide(APP_NAME);
    // SAFETY: only the existence of the value is queried (no data buffer).
    let r = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    unsafe { RegCloseKey(key) };
    r == ERROR_SUCCESS
}

/// Adds or removes the startup entry. It takes effect at the next login (no second copy is started now).
pub fn set_launch_at_login(on: bool) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let Some(key) = open(KEY_SET_VALUE) else {
        return Err(std::io::Error::last_os_error());
    };
    let name = wide(APP_NAME);
    // SAFETY: the data pointer and its byte length describe the `value` buffer, which outlives the call.
    let code = unsafe {
        if on {
            let value = wide(&run_value(&exe.to_string_lossy()));
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        } else {
            match RegDeleteValueW(key, name.as_ptr()) {
                ERROR_FILE_NOT_FOUND => ERROR_SUCCESS, // already off
                other => other,
            }
        }
    };
    unsafe { RegCloseKey(key) };
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(code as i32))
    }
}
