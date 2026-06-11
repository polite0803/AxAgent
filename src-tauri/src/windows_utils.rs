// SPDX-License-Identifier: AGPL-3.0-only

//! Windows-specific utilities: native error dialogs for fatal startup failures.

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDOK, MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_OKCANCEL, MessageBoxW,
};

/// Encode a Rust string as a null-terminated UTF-16 vector for Win32 APIs.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Show a native Windows MessageBox with an error icon.
pub fn show_error_dialog(title: &str, message: &str) {
    let wide_title = to_wide(title);
    let wide_msg = to_wide(message);
    // SAFETY: wide_msg and wide_title are properly null-terminated UTF-16 strings
    // (created by to_wide which appends 0); HWND 0 is valid for message boxes
    // (no owner window); MB_OK | MB_ICONERROR are valid flag combinations.
    unsafe {
        MessageBoxW(0 as HWND, wide_msg.as_ptr(), wide_title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

/// Show a native Windows MessageBox with a warning icon and OK/Cancel buttons.
/// Returns `true` if the user clicked OK.
pub fn show_warning_ok_cancel(title: &str, message: &str) -> bool {
    let wide_title = to_wide(title);
    let wide_msg = to_wide(message);
    // SAFETY: Same as above — wide_msg and wide_title are properly null-terminated
    // UTF-16 strings; HWND 0 is valid; MB_OKCANCEL | MB_ICONWARNING are valid flag combinations.
    let result = unsafe {
        MessageBoxW(0 as HWND, wide_msg.as_ptr(), wide_title.as_ptr(), MB_OKCANCEL | MB_ICONWARNING)
    };
    result == IDOK
}
