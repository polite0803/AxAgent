//! Windows-specific utilities: native error dialogs for fatal startup failures.

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDOK, MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_OKCANCEL,
};

/// Encode a Rust string as a null-terminated UTF-16 vector for Win32 APIs.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Show a native Windows MessageBox with an error icon.
pub fn show_error_dialog(title: &str, message: &str) {
    let wide_title = to_wide(title);
    let wide_msg = to_wide(message);
    unsafe {
        MessageBoxW(
            0 as HWND,
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

/// Show a native Windows MessageBox with a warning icon and OK/Cancel buttons.
/// Returns `true` if the user clicked OK.
pub fn show_warning_ok_cancel(title: &str, message: &str) -> bool {
    let wide_title = to_wide(title);
    let wide_msg = to_wide(message);
    let result = unsafe {
        MessageBoxW(
            0 as HWND,
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            MB_OKCANCEL | MB_ICONWARNING,
        )
    };
    result == IDOK
}
