use std::ffi::c_void;
use windows_sys::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLOR_WINDOW, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, FF_DONTCARE, GetSysColorBrush, HBRUSH, HDC,
    HFONT, OUT_DEFAULT_PRECIS, SetBkColor, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::SystemServices::{
    SS_CENTER, SS_CENTERIMAGE, SS_NOPREFIX, SS_PATHELLIPSIS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST,
    CBS_HASSTRINGS, CreateWindowExW, GetDlgCtrlID, SendMessageW, SetWindowTextW, WM_SETFONT,
    WS_BORDER, WS_CHILD, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

pub const IDC_TIMEZONE: i32 = 101;
pub const IDC_LOCATE: i32 = 102;
pub const IDC_SAVE: i32 = 103;
pub const IDC_SAVE_LAUNCH: i32 = 104;

const IDC_BADGE: i32 = 201;
const IDC_TITLE: i32 = 202;
const IDC_SUBTITLE: i32 = 203;
const IDC_INFO: i32 = 204;
const IDC_HELP: i32 = 205;
const IDC_CLIENT_PATH: i32 = 206;
const IDC_STATUS: i32 = 207;
const IDC_FOOTNOTE: i32 = 208;

const COLOR_TEXT: COLORREF = rgb(28, 38, 53);
const COLOR_MUTED: COLORREF = rgb(94, 105, 120);
const COLOR_BLUE: COLORREF = rgb(31, 111, 235);
const COLOR_BLUE_TEXT: COLORREF = rgb(31, 78, 121);
const COLOR_INFO_BG: COLORREF = rgb(237, 245, 255);

pub struct UiControls {
    pub timezone: HWND,
    pub locate: HWND,
    status: HWND,
    regular_font: HFONT,
    title_font: HFONT,
    badge_font: HFONT,
    info_brush: HBRUSH,
    badge_brush: HBRUSH,
    status_color: COLORREF,
}

impl UiControls {
    pub fn create(
        parent: HWND,
        instance: HINSTANCE,
        zones: &[String],
        selected_zone: &str,
        client_text: &str,
        initial_status: &str,
    ) -> Result<Self, String> {
        let regular_font = create_font(-17, 400);
        let title_font = create_font(-28, 600);
        let badge_font = create_font(-18, 700);
        if regular_font.is_null() || title_font.is_null() || badge_font.is_null() {
            return Err("无法创建界面字体。".into());
        }

        let badge = create_control(
            parent,
            instance,
            "STATIC",
            "TZ",
            WS_CHILD | WS_VISIBLE | SS_CENTER | SS_CENTERIMAGE,
            (34, 28, 42, 38),
            IDC_BADGE,
        )?;
        let title = create_control(
            parent,
            instance,
            "STATIC",
            "Codex 时区启动器",
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
            (90, 27, 636, 38),
            IDC_TITLE,
        )?;
        let subtitle = create_control(
            parent,
            instance,
            "STATIC",
            "只为本次启动的 Codex 设置时区，不更改 Windows 系统时区。",
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
            (34, 72, 692, 24),
            IDC_SUBTITLE,
        )?;
        let info = create_control(
            parent,
            instance,
            "STATIC",
            "  时区环境变量仅在进程启动时读取。若 Codex 已运行，请先保存内容并自行退出，\r\n  再回来启动；本工具绝不会强制结束进程。",
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
            (34, 108, 692, 62),
            IDC_INFO,
        )?;
        let timezone_label = create_control(
            parent,
            instance,
            "STATIC",
            "时区(&T)",
            WS_CHILD | WS_VISIBLE,
            (34, 198, 102, 26),
            0,
        )?;
        let timezone = create_control(
            parent,
            instance,
            "COMBOBOX",
            "",
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_VSCROLL
                | CBS_DROPDOWNLIST as u32
                | CBS_HASSTRINGS as u32,
            (150, 192, 418, 280),
            IDC_TIMEZONE,
        )?;
        let locate = create_control(
            parent,
            instance,
            "BUTTON",
            "自动定位(&A)",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            (580, 191, 146, 34),
            IDC_LOCATE,
        )?;
        let help = create_control(
            parent,
            instance,
            "STATIC",
            "支持完整 IANA 时区，例如 Asia/Shanghai、America/New_York。自动定位只访问固定 HTTPS 服务，且不保存 IP。",
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
            (150, 233, 576, 40),
            IDC_HELP,
        )?;
        let divider = create_control(
            parent,
            instance,
            "STATIC",
            "",
            WS_CHILD | WS_VISIBLE | WS_BORDER,
            (34, 282, 692, 1),
            0,
        )?;
        let client_label = create_control(
            parent,
            instance,
            "STATIC",
            "客户端",
            WS_CHILD | WS_VISIBLE,
            (34, 306, 102, 26),
            0,
        )?;
        let client_path = create_control(
            parent,
            instance,
            "STATIC",
            client_text,
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX | SS_PATHELLIPSIS,
            (150, 306, 576, 26),
            IDC_CLIENT_PATH,
        )?;
        let status = create_control(
            parent,
            instance,
            "STATIC",
            initial_status,
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
            (34, 347, 692, 38),
            IDC_STATUS,
        )?;
        let footnote = create_control(
            parent,
            instance,
            "STATIC",
            "适用于使用运行时默认时区的 ChatGPT/Electron 显示与逻辑；极少数直接调用 Windows 原生时区 API 的功能仍可能使用系统时区。",
            WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
            (34, 393, 692, 42),
            IDC_FOOTNOTE,
        )?;
        let save = create_control(
            parent,
            instance,
            "BUTTON",
            "仅保存设置(&S)",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
            (34, 454, 160, 40),
            IDC_SAVE,
        )?;
        let save_launch = create_control(
            parent,
            instance,
            "BUTTON",
            "保存并启动 Codex(&L)",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
            (486, 450, 240, 44),
            IDC_SAVE_LAUNCH,
        )?;

        for control in [
            subtitle,
            info,
            timezone_label,
            timezone,
            locate,
            help,
            divider,
            client_label,
            client_path,
            status,
            footnote,
            save,
            save_launch,
        ] {
            set_font(control, regular_font);
        }
        set_font(title, title_font);
        set_font(badge, badge_font);

        for zone in zones {
            let zone = wide(zone);
            unsafe { SendMessageW(timezone, CB_ADDSTRING, 0, zone.as_ptr() as LPARAM) };
        }
        let selection = zones
            .iter()
            .position(|zone| zone == selected_zone)
            .unwrap_or(0);
        unsafe { SendMessageW(timezone, CB_SETCURSEL, selection, 0) };

        Ok(Self {
            timezone,
            locate,
            status,
            regular_font,
            title_font,
            badge_font,
            info_brush: unsafe { CreateSolidBrush(COLOR_INFO_BG) },
            badge_brush: unsafe { CreateSolidBrush(COLOR_BLUE) },
            status_color: COLOR_MUTED,
        })
    }

    pub fn selected_index(&self) -> Option<usize> {
        let index = unsafe { SendMessageW(self.timezone, CB_GETCURSEL, 0, 0) };
        (index >= 0).then_some(index as usize)
    }

    pub fn select_index(&self, index: usize) {
        unsafe { SendMessageW(self.timezone, CB_SETCURSEL, index, 0) };
    }

    pub fn set_status(&mut self, text: &str, color: COLORREF) {
        self.status_color = color;
        let text = wide(text);
        unsafe { SetWindowTextW(self.status, text.as_ptr()) };
    }

    pub fn static_brush(&self, control: HWND, device: HDC) -> HBRUSH {
        let id = unsafe { GetDlgCtrlID(control) };
        unsafe { SetBkMode(device, TRANSPARENT as i32) };
        match id {
            IDC_BADGE => {
                unsafe {
                    SetBkColor(device, COLOR_BLUE);
                    SetTextColor(device, rgb(255, 255, 255));
                }
                self.badge_brush
            }
            IDC_INFO => {
                unsafe {
                    SetBkColor(device, COLOR_INFO_BG);
                    SetTextColor(device, COLOR_BLUE_TEXT);
                }
                self.info_brush
            }
            IDC_TITLE => {
                unsafe { SetTextColor(device, COLOR_TEXT) };
                unsafe { GetSysColorBrush(COLOR_WINDOW) }
            }
            IDC_STATUS => {
                unsafe { SetTextColor(device, self.status_color) };
                unsafe { GetSysColorBrush(COLOR_WINDOW) }
            }
            IDC_SUBTITLE | IDC_HELP | IDC_FOOTNOTE => {
                unsafe { SetTextColor(device, COLOR_MUTED) };
                unsafe { GetSysColorBrush(COLOR_WINDOW) }
            }
            _ => {
                unsafe { SetTextColor(device, COLOR_TEXT) };
                unsafe { GetSysColorBrush(COLOR_WINDOW) }
            }
        }
    }
}

impl Drop for UiControls {
    fn drop(&mut self) {
        for object in [
            self.regular_font as *mut c_void,
            self.title_font as *mut c_void,
            self.badge_font as *mut c_void,
            self.info_brush as *mut c_void,
            self.badge_brush as *mut c_void,
        ] {
            if !object.is_null() {
                unsafe { DeleteObject(object) };
            }
        }
    }
}

pub const STATUS_INFO: COLORREF = rgb(48, 94, 139);
pub const STATUS_SUCCESS: COLORREF = rgb(24, 120, 76);
pub const STATUS_WARNING: COLORREF = rgb(162, 91, 0);
pub const STATUS_ERROR: COLORREF = rgb(185, 45, 45);

fn create_control(
    parent: HWND,
    instance: HINSTANCE,
    class: &str,
    text: &str,
    style: u32,
    bounds: (i32, i32, i32, i32),
    id: i32,
) -> Result<HWND, String> {
    let class = wide(class);
    let text = wide(text);
    let control = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            style,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            parent,
            id as usize as *mut c_void,
            instance,
            std::ptr::null(),
        )
    };
    if control.is_null() {
        Err("无法创建界面控件。".into())
    } else {
        Ok(control)
    }
}

fn create_font(height: i32, weight: i32) -> HFONT {
    let face = wide("Microsoft YaHei UI");
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32,
            face.as_ptr(),
        )
    }
}

fn set_font(control: HWND, font: HFONT) {
    unsafe { SendMessageW(control, WM_SETFONT, font as usize, 1) };
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
