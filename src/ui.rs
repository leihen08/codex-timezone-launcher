use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{COLOR_WINDOW, GetSysColorBrush, HDC, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DispatchMessageW, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, IDC_ARROW, IDI_APPLICATION,
    IsDialogMessageW, LoadCursorW, LoadIconW, MB_ICONERROR, MB_OK, MSG, MessageBoxW,
    PostQuitMessage, RegisterClassW, SW_SHOW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
    WM_COMMAND, WM_CREATE, WM_CTLCOLORSTATIC, WM_DESTROY, WM_NCDESTROY, WM_TIMER, WNDCLASSW,
    WS_CAPTION, WS_CLIPCHILDREN, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

use crate::ui_app::{AppState, WM_GEO_RESULT};

const WINDOW_CLASS: &str = "ChatGPTTimeZoneLauncherWindow";
const WINDOW_TITLE: &str = "Codex 时区启动器";
const CLIENT_WIDTH: i32 = 760;
const CLIENT_HEIGHT: i32 = 520;

pub fn run() -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let controls = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES,
    };
    unsafe { InitCommonControlsEx(&controls) };

    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err("无法初始化 Windows 应用程序。".into());
    }
    let class_name = wide(WINDOW_CLASS);
    let window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hIcon: unsafe { LoadIconW(null_mut(), IDI_APPLICATION) },
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: unsafe { GetSysColorBrush(COLOR_WINDOW) },
        lpszClassName: class_name.as_ptr(),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&window_class) } == 0 {
        return Err("无法注册启动器窗口。".into());
    }

    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
    let mut bounds = RECT {
        left: 0,
        top: 0,
        right: CLIENT_WIDTH,
        bottom: CLIENT_HEIGHT,
    };
    unsafe { AdjustWindowRectEx(&mut bounds, style, 0, 0) };
    let title = wide(WINDOW_TITLE);
    let window = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            null_mut(),
            null_mut(),
            instance,
            null(),
        )
    };
    if window.is_null() {
        return Err("无法创建启动器窗口。".into());
    }

    unsafe {
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
    }
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == 0 {
            break;
        }
        if result == -1 {
            return Err("Windows 消息循环发生错误。".into());
        }
        if unsafe { IsDialogMessageW(window, &message) } == 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    Ok(())
}

pub fn show_fatal(message: &str) {
    let message = wide(message);
    let title = wide(WINDOW_TITLE);
    unsafe {
        MessageBoxW(
            null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let instance = unsafe { GetModuleHandleW(null()) } as HINSTANCE;
            match AppState::create(window, instance) {
                Ok(state) => {
                    unsafe {
                        SetWindowLongPtrW(
                            window,
                            GWLP_USERDATA,
                            Box::into_raw(Box::new(state)) as isize,
                        )
                    };
                    return 0;
                }
                Err(message) => {
                    show_owned_error(window, &message);
                    return -1;
                }
            }
        }
        WM_COMMAND => {
            let state = unsafe { state_pointer(window) };
            if !state.is_null() {
                let control_id = (wparam & 0xffff) as i32;
                let notification = ((wparam >> 16) & 0xffff) as u32;
                unsafe { (*state).handle_command(window, control_id, notification) };
                return 0;
            }
        }
        WM_GEO_RESULT => {
            let state = unsafe { state_pointer(window) };
            if !state.is_null() {
                unsafe { (*state).handle_geo_result(window) };
                return 0;
            }
        }
        WM_TIMER => {
            let state = unsafe { state_pointer(window) };
            if !state.is_null() {
                unsafe { (*state).handle_timer(window, wparam) };
                return 0;
            }
        }
        WM_CTLCOLORSTATIC => {
            let state = unsafe { state_pointer(window) };
            if !state.is_null() {
                return unsafe { (*state).static_brush(lparam as HWND, wparam as HDC) } as LRESULT;
            }
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            return 0;
        }
        WM_NCDESTROY => {
            let pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *mut AppState;
            if !pointer.is_null() {
                unsafe {
                    SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                    drop(Box::from_raw(pointer));
                }
            }
        }
        _ => {}
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

unsafe fn state_pointer(window: HWND) -> *mut AppState {
    unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut AppState }
}

fn show_owned_error(window: HWND, message: &str) {
    let message = wide(message);
    let title = wide(WINDOW_TITLE);
    unsafe {
        MessageBoxW(
            window,
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        )
    };
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
