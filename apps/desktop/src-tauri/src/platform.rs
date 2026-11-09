#[cfg(windows)]
pub fn protect_settings(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows::{
        Win32::{
            Foundation::{HLOCAL, LocalFree},
            Security::Cryptography::{
                CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
            },
        },
        core::PCWSTR,
    };

    let source = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len()).map_err(|_| "Provider settings are too large.")?,
        pbData: input.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // The input buffer lives through the DPAPI call. Windows owns and allocates the output.
    let result = unsafe {
        if encrypt {
            CryptProtectData(
                &source,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    result.map_err(|_| "Windows could not open the protected provider settings.")?;
    if output.pbData.is_null() || output.cbData == 0 {
        return Err("Windows returned empty provider settings.".into());
    }
    // Copy the returned bytes and release the Windows allocation with its matching API.
    let bytes = unsafe {
        let length = output.cbData as usize;
        let bytes = std::slice::from_raw_parts(output.pbData, length).to_vec();
        if !encrypt {
            std::ptr::write_bytes(output.pbData, 0, length);
        }
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        bytes
    };
    Ok(bytes)
}

#[cfg(not(windows))]
pub fn protect_settings(_: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Err("Protected provider settings are available on Windows.".into())
}

#[cfg(windows)]
pub fn open_browser(url: &str) -> Result<(), String> {
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::{HSTRING, PCWSTR, w},
    };

    let url = HSTRING::from(url);
    // The only callers supply fixed HTTPS destinations or the HTTPS authorization URL.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &url,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("Windows could not open your default browser.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn open_browser(_: &str) -> Result<(), String> {
    Err("Open the provider in your browser on Windows.".into())
}

#[cfg(windows)]
pub fn set_capture_exclusion(window: &tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowDisplayAffinity, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
        WINDOW_DISPLAY_AFFINITY,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let requested = if enabled {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    let mut previous = 0;
    // This is a live top-level OpenKeet window, and the output variables are writable.
    unsafe { GetWindowDisplayAffinity(hwnd, &mut previous) }
        .map_err(|error| format!("Windows could not read screen-share privacy. {error}"))?;
    // A hidden window can report WDA_NONE while retaining its affinity on reveal.
    unsafe { SetWindowDisplayAffinity(hwnd, requested) }
        .map_err(|error| format!("Windows rejected screen-share privacy. {error}"))?;
    let mut actual = 0;
    if unsafe { GetWindowDisplayAffinity(hwnd, &mut actual) }.is_ok() && actual == requested.0 {
        return Ok(());
    }
    unsafe { SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(previous)) }.map_err(
        |error| format!("Windows could not confirm or restore screen-share privacy. {error}"),
    )?;
    Err("Windows did not confirm the requested screen-share privacy setting.".into())
}

#[cfg(not(windows))]
pub fn set_capture_exclusion(_: &tauri::WebviewWindow, _: bool) -> Result<(), String> {
    Err("Screen-share privacy is available on Windows.".into())
}

#[cfg(windows)]
pub fn configure_tool_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows::Win32::{
        UI::Shell::SetWindowSubclass,
        UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos,
        },
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    // Both callers run on the owning UI thread before the window first becomes visible.
    unsafe {
        if !SetWindowSubclass(hwnd, Some(keep_tool_style), 0, 0).as_bool() {
            return Err("Windows could not maintain the companion window style.".into());
        }
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = tool_style(current as u32) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) != style {
            return Err("Windows could not apply the companion window style.".into());
        }
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(windows)]
fn tool_style(style: u32) -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{WS_EX_APPWINDOW, WS_EX_TOOLWINDOW};
    (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0
}

#[cfg(windows)]
unsafe extern "system" fn keep_tool_style(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    id: usize,
    _: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass},
        WindowsAndMessaging::{GWL_EXSTYLE, STYLESTRUCT, WM_NCDESTROY, WM_STYLECHANGING},
    };

    // Windows supplies the writable STYLESTRUCT. Tao rebuilds its styles on show, hide and pin.
    if message == WM_STYLECHANGING && wparam.0 as isize == GWL_EXSTYLE.0 as isize && lparam.0 != 0 {
        let styles = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
        styles.styleNew = tool_style(styles.styleNew);
    } else if message == WM_NCDESTROY {
        let _ = unsafe { RemoveWindowSubclass(hwnd, Some(keep_tool_style), id) };
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[cfg(not(windows))]
pub fn configure_tool_window(_: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}

/// Tells WebView2 a window is hidden (`low`) so it can trim memory, or visible again.
#[cfg(windows)]
pub fn memory_target(window: &tauri::WebviewWindow, low: bool) {
    let _ = window.with_webview(move |webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_19,
        };
        use windows::core::Interface;
        let level = if low {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
        } else {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
        };
        // SAFETY: called on the webview's thread with its live controller; older runtimes lack
        // ICoreWebView2_19 and simply keep their default level.
        unsafe {
            if let Ok(core) = webview.controller().CoreWebView2()
                && let Ok(webview) = core.cast::<ICoreWebView2_19>()
            {
                let _ = webview.SetMemoryUsageTargetLevel(level);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn memory_target(_: &tauri::WebviewWindow, _: bool) {}
