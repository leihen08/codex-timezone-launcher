use std::ffi::c_void;
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    DeleteObject, FF_DONTCARE, HFONT, OUT_DEFAULT_PRECIS,
};
use windows_sys::Win32::System::SystemServices::{
    SS_CENTER, SS_CENTERIMAGE, SS_NOPREFIX, SS_PATHELLIPSIS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CB_ADDSTRING, CB_LIMITTEXT, CB_SETCURSEL, CBS_DROPDOWN,
    CBS_HASSTRINGS, CreateWindowExW, SendMessageW, WM_SETFONT, WS_BORDER, WS_CHILD, WS_TABSTOP,
    WS_VISIBLE, WS_VSCROLL,
};

pub const IDC_TIMEZONE: i32 = 101;
pub const IDC_LOCATE: i32 = 102;
pub const IDC_SAVE: i32 = 103;
pub const IDC_SAVE_LAUNCH: i32 = 104;
pub const IDC_BADGE: i32 = 201;
pub const IDC_TITLE: i32 = 202;
pub const IDC_SUBTITLE: i32 = 203;
pub const IDC_INFO: i32 = 204;
pub const IDC_HELP: i32 = 205;
pub const IDC_CLIENT_PATH: i32 = 206;
pub const IDC_STATUS: i32 = 207;
pub const IDC_FOOTNOTE: i32 = 208;

pub struct FontSet {
    regular: HFONT,
    title: HFONT,
    badge: HFONT,
}

impl FontSet {
    fn create() -> Result<Self, String> {
        let fonts = Self {
            regular: create_font(-17, 400),
            title: create_font(-28, 600),
            badge: create_font(-18, 700),
        };
        if fonts.regular.is_null() || fonts.title.is_null() || fonts.badge.is_null() {
            Err("无法创建界面字体。".into())
        } else {
            Ok(fonts)
        }
    }
}

impl Drop for FontSet {
    fn drop(&mut self) {
        for font in [self.regular, self.title, self.badge] {
            if !font.is_null() {
                unsafe { DeleteObject(font) };
            }
        }
    }
}

pub struct Layout {
    pub timezone: HWND,
    pub locate: HWND,
    pub status: HWND,
    pub fonts: FontSet,
}

pub fn create_layout(
    parent: HWND,
    instance: HINSTANCE,
    zones: &[String],
    selected_zone: &str,
    client_text: &str,
    initial_status: &str,
) -> Result<Layout, String> {
    let fonts = FontSet::create()?;
    let factory = ControlFactory { parent, instance };
    let badge = factory.create(
        "STATIC",
        "TZ",
        WS_CHILD | WS_VISIBLE | SS_CENTER | SS_CENTERIMAGE,
        (34, 28, 42, 38),
        IDC_BADGE,
    )?;
    let title = factory.create(
        "STATIC",
        "Codex 时区启动器",
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
        (90, 27, 636, 38),
        IDC_TITLE,
    )?;
    let subtitle = factory.create(
        "STATIC",
        "只为本次启动的 Codex 设置时区，不更改 Windows 系统时区。",
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
        (34, 72, 692, 24),
        IDC_SUBTITLE,
    )?;
    let info = factory.create(
        "STATIC",
        "  时区环境变量仅在进程启动时读取。若 Codex 已运行，请先保存内容并自行退出，\r\n  再回来启动；本工具绝不会强制结束进程。",
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
        (34, 108, 692, 62),
        IDC_INFO,
    )?;
    let timezone_label = factory.create(
        "STATIC",
        "时区(&T)",
        WS_CHILD | WS_VISIBLE,
        (34, 198, 102, 26),
        0,
    )?;
    let timezone = factory.create(
        "COMBOBOX",
        "",
        WS_CHILD
            | WS_VISIBLE
            | WS_TABSTOP
            | WS_VSCROLL
            | CBS_DROPDOWN as u32
            | CBS_HASSTRINGS as u32,
        (150, 192, 418, 280),
        IDC_TIMEZONE,
    )?;
    unsafe { SendMessageW(timezone, CB_LIMITTEXT, 64, 0) };
    let locate = factory.create(
        "BUTTON",
        "自动定位(&A)",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
        (580, 191, 146, 34),
        IDC_LOCATE,
    )?;
    let help = factory.create(
        "STATIC",
        "可输入 shanghai、new_york 等关键字搜索完整 IANA 时区。自动定位只访问固定 HTTPS 服务，且不保存 IP。",
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
        (150, 233, 576, 40),
        IDC_HELP,
    )?;
    let divider = factory.create(
        "STATIC",
        "",
        WS_CHILD | WS_VISIBLE | WS_BORDER,
        (34, 282, 692, 1),
        0,
    )?;
    let client_label = factory.create(
        "STATIC",
        "客户端",
        WS_CHILD | WS_VISIBLE,
        (34, 306, 102, 26),
        0,
    )?;
    let client_path = factory.create(
        "STATIC",
        client_text,
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX | SS_PATHELLIPSIS,
        (150, 306, 576, 26),
        IDC_CLIENT_PATH,
    )?;
    let status = factory.create(
        "STATIC",
        initial_status,
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
        (34, 347, 692, 38),
        IDC_STATUS,
    )?;
    let footnote = factory.create(
        "STATIC",
        "适用于使用运行时默认时区的 ChatGPT/Electron 显示与逻辑；极少数直接调用 Windows 原生时区 API 的功能仍可能使用系统时区。",
        WS_CHILD | WS_VISIBLE | SS_NOPREFIX,
        (34, 393, 692, 42),
        IDC_FOOTNOTE,
    )?;
    let save = factory.create(
        "BUTTON",
        "仅保存设置(&S)",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
        (34, 454, 160, 40),
        IDC_SAVE,
    )?;
    let save_launch = factory.create(
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
        set_font(control, fonts.regular);
    }
    set_font(title, fonts.title);
    set_font(badge, fonts.badge);
    populate_timezones(timezone, zones, selected_zone);

    Ok(Layout {
        timezone,
        locate,
        status,
        fonts,
    })
}

fn populate_timezones(control: HWND, zones: &[String], selected_zone: &str) {
    for zone in zones {
        let zone = wide(zone);
        unsafe { SendMessageW(control, CB_ADDSTRING, 0, zone.as_ptr() as LPARAM) };
    }
    let selection = zones
        .iter()
        .position(|zone| zone == selected_zone)
        .unwrap_or(0);
    unsafe { SendMessageW(control, CB_SETCURSEL, selection, 0) };
}

struct ControlFactory {
    parent: HWND,
    instance: HINSTANCE,
}

impl ControlFactory {
    fn create(
        &self,
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
                self.parent,
                id as usize as *mut c_void,
                self.instance,
                std::ptr::null(),
            )
        };
        if control.is_null() {
            Err("无法创建界面控件。".into())
        } else {
            Ok(control)
        }
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

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
