use crate::ui_layout::{
    FontSet, IDC_BADGE, IDC_FOOTNOTE, IDC_HELP, IDC_INFO, IDC_STATUS, IDC_SUBTITLE, IDC_TITLE,
    create_layout,
};
use windows_sys::Win32::Foundation::{COLORREF, HINSTANCE, HWND};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_WINDOW, CreateSolidBrush, DeleteObject, GetSysColorBrush, HBRUSH, HDC, SetBkColor,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{COMBOBOXINFO, GetComboBoxInfo};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CB_ADDSTRING, CB_RESETCONTENT, CB_SETCURSEL, CB_SETEDITSEL, CB_SHOWDROPDOWN, GetDlgCtrlID,
    GetWindowTextLengthW, GetWindowTextW, SendMessageW, SetWindowTextW,
};

const COLOR_TEXT: COLORREF = rgb(28, 38, 53);
const COLOR_MUTED: COLORREF = rgb(94, 105, 120);
const COLOR_BLUE: COLORREF = rgb(31, 111, 235);
const COLOR_BLUE_TEXT: COLORREF = rgb(31, 78, 121);
const COLOR_INFO_BG: COLORREF = rgb(237, 245, 255);

pub const STATUS_INFO: COLORREF = rgb(48, 94, 139);
pub const STATUS_SUCCESS: COLORREF = rgb(24, 120, 76);
pub const STATUS_WARNING: COLORREF = rgb(162, 91, 0);
pub const STATUS_ERROR: COLORREF = rgb(185, 45, 45);

pub struct UiControls {
    pub timezone: HWND,
    pub locate: HWND,
    timezone_edit: HWND,
    status: HWND,
    _fonts: FontSet,
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
        let layout = create_layout(
            parent,
            instance,
            zones,
            selected_zone,
            client_text,
            initial_status,
        )?;
        let mut combo_info = COMBOBOXINFO {
            cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetComboBoxInfo(layout.timezone, &mut combo_info) } == 0
            || combo_info.hwndItem.is_null()
        {
            return Err("无法初始化时区搜索框。".into());
        }
        let info_brush = unsafe { CreateSolidBrush(COLOR_INFO_BG) };
        let badge_brush = unsafe { CreateSolidBrush(COLOR_BLUE) };
        if info_brush.is_null() || badge_brush.is_null() {
            for brush in [info_brush, badge_brush] {
                if !brush.is_null() {
                    unsafe { DeleteObject(brush) };
                }
            }
            return Err("无法创建界面画刷。".into());
        }
        Ok(Self {
            timezone: layout.timezone,
            locate: layout.locate,
            timezone_edit: combo_info.hwndItem,
            status: layout.status,
            _fonts: layout.fonts,
            info_brush,
            badge_brush,
            status_color: COLOR_MUTED,
        })
    }

    pub fn timezone_text(&self) -> Result<String, String> {
        let length = unsafe { GetWindowTextLengthW(self.timezone_edit) };
        if !(1..=64).contains(&length) {
            return Err("请输入或选择一个有效 IANA 时区。".into());
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let copied =
            unsafe { GetWindowTextW(self.timezone_edit, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            Err("无法读取时区输入。".into())
        } else {
            Ok(String::from_utf16_lossy(&buffer[..copied as usize]))
        }
    }

    pub fn filter_timezones(&self, matches: &[&str], query: &str) {
        unsafe { SendMessageW(self.timezone, CB_RESETCONTENT, 0, 0) };
        for timezone in matches {
            let timezone = wide(timezone);
            unsafe { SendMessageW(self.timezone, CB_ADDSTRING, 0, timezone.as_ptr() as isize) };
        }
        let query_text = wide(query);
        unsafe { SetWindowTextW(self.timezone_edit, query_text.as_ptr()) };
        let cursor = query.encode_utf16().count().min(64);
        let selection = ((cursor as isize) << 16) | cursor as isize;
        unsafe {
            SendMessageW(self.timezone, CB_SETEDITSEL, 0, selection);
            SendMessageW(self.timezone, CB_SHOWDROPDOWN, 1, 0);
        }
    }

    pub fn select_timezone(&self, zones: &[String], selected_index: usize) {
        unsafe { SendMessageW(self.timezone, CB_RESETCONTENT, 0, 0) };
        for timezone in zones {
            let timezone = wide(timezone);
            unsafe { SendMessageW(self.timezone, CB_ADDSTRING, 0, timezone.as_ptr() as isize) };
        }
        unsafe { SendMessageW(self.timezone, CB_SETCURSEL, selected_index, 0) };
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
            IDC_TITLE => self.system_brush_with_text(device, COLOR_TEXT),
            IDC_STATUS => self.system_brush_with_text(device, self.status_color),
            IDC_SUBTITLE | IDC_HELP | IDC_FOOTNOTE => {
                self.system_brush_with_text(device, COLOR_MUTED)
            }
            _ => self.system_brush_with_text(device, COLOR_TEXT),
        }
    }

    fn system_brush_with_text(&self, device: HDC, color: COLORREF) -> HBRUSH {
        unsafe {
            SetTextColor(device, color);
            GetSysColorBrush(COLOR_WINDOW)
        }
    }
}

impl Drop for UiControls {
    fn drop(&mut self) {
        for brush in [self.info_brush, self.badge_brush] {
            unsafe { DeleteObject(brush) };
        }
    }
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
