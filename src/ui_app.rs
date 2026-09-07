use crate::config::{Settings, load_settings, save_settings};
use crate::discovery::discover_client;
use crate::geo::lookup_timezone;
use crate::timezone::{DEFAULT_TIMEZONE, all_timezones};
use crate::ui_controls::{STATUS_ERROR, STATUS_INFO, STATUS_SUCCESS, STATUS_WARNING, UiControls};
use crate::ui_layout::{IDC_LOCATE, IDC_SAVE, IDC_SAVE_LAUNCH};
use crate::workflow::{SaveLaunchOutcome, save_and_launch};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::Graphics::Gdi::{HBRUSH, HDC};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MessageBoxW, PostMessageW,
    WM_APP,
};

pub const WM_GEO_RESULT: u32 = WM_APP + 1;
static GEO_RESULT: Mutex<Option<Result<String, String>>> = Mutex::new(None);

pub struct AppState {
    zones: Vec<String>,
    controls: UiControls,
}

impl AppState {
    pub fn create(window: HWND, instance: HINSTANCE) -> Result<Self, String> {
        let zones = all_timezones();
        let (selected, initial_status) = match load_settings() {
            Ok(Some(settings)) => (settings.timezone, "已载入上次保存的时区。".to_string()),
            Ok(None) => (
                DEFAULT_TIMEZONE.to_string(),
                "首次使用：请选择时区，或根据公网 IP 自动定位。".to_string(),
            ),
            Err(message) => (
                DEFAULT_TIMEZONE.to_string(),
                format!("{message} 已使用安全默认值 Etc/UTC。"),
            ),
        };
        let client_text = match discover_client() {
            Ok(path) => format!("已检测：{}", path.display()),
            Err(message) => message,
        };
        let controls = UiControls::create(
            window,
            instance,
            &zones,
            &selected,
            &client_text,
            &initial_status,
        )?;
        Ok(Self { zones, controls })
    }

    pub fn handle_command(&mut self, window: HWND, control_id: i32, notification: u32) {
        if notification != BN_CLICKED {
            return;
        }
        match control_id {
            IDC_LOCATE => self.start_geo_lookup(window),
            IDC_SAVE => self.save_selected(window),
            IDC_SAVE_LAUNCH => self.save_and_launch_selected(window),
            _ => {}
        }
    }

    pub fn handle_geo_result(&mut self, window: HWND) {
        unsafe { EnableWindow(self.controls.locate, 1) };
        let result = GEO_RESULT.lock().ok().and_then(|mut slot| slot.take());
        match result {
            Some(Ok(timezone)) => {
                if let Some(index) = self.zones.iter().position(|zone| zone == &timezone) {
                    self.controls.select_index(index);
                    self.controls.set_status(
                        &format!("已定位为 {timezone}。IP 未保存；请保存或直接启动。"),
                        STATUS_SUCCESS,
                    );
                } else {
                    self.report_error(window, "定位结果不在内置 IANA 时区表中。");
                }
            }
            Some(Err(message)) => self.report_error(window, &message),
            None => self.report_error(window, "未能取得定位结果，请重试。"),
        }
    }

    pub fn static_brush(&self, control: HWND, device: HDC) -> HBRUSH {
        self.controls.static_brush(control, device)
    }

    fn selected_timezone(&self) -> Result<&str, String> {
        self.controls
            .selected_index()
            .and_then(|index| self.zones.get(index))
            .map(String::as_str)
            .ok_or_else(|| "请选择一个有效时区。".into())
    }

    fn save_selected(&mut self, window: HWND) {
        let result = self
            .selected_timezone()
            .and_then(Settings::new)
            .and_then(|settings| save_settings(&settings).map(|_| settings.timezone));
        match result {
            Ok(timezone) => self.controls.set_status(
                &format!("已保存 {timezone}，下次打开会自动选中。"),
                STATUS_SUCCESS,
            ),
            Err(message) => self.report_error(window, &message),
        }
    }

    fn save_and_launch_selected(&mut self, window: HWND) {
        let timezone = match self.selected_timezone() {
            Ok(timezone) => timezone.to_string(),
            Err(message) => {
                self.report_error(window, &message);
                return;
            }
        };
        match save_and_launch(&timezone) {
            Ok(SaveLaunchOutcome::AlreadyRunning(_)) => {
                let message = concat!(
                    "检测到 Codex/ChatGPT 客户端正在运行。\r\n\r\n",
                    "时区只会在进程启动时读取。请先在客户端中保存未完成内容，",
                    "然后自行退出客户端，再回到这里点击“保存并启动 Codex”。\r\n\r\n",
                    "为避免数据丢失，本启动器绝不会强制结束任何进程。"
                );
                self.controls.set_status(
                    "设置已保存；请保存客户端内容并自行退出后再启动。",
                    STATUS_WARNING,
                );
                show_message(window, message, MB_ICONWARNING);
            }
            Ok(SaveLaunchOutcome::Launched(_)) => {
                self.controls
                    .set_status(&format!("已使用 {timezone} 启动 Codex。"), STATUS_SUCCESS);
                show_message(
                    window,
                    "Codex 已启动；Windows 系统时区没有改变。",
                    MB_ICONINFORMATION,
                );
            }
            Err(message) => self.report_error(window, &message),
        }
    }

    fn start_geo_lookup(&mut self, window: HWND) {
        if let Ok(mut slot) = GEO_RESULT.lock() {
            *slot = None;
        }
        unsafe { EnableWindow(self.controls.locate, 0) };
        self.controls
            .set_status("正在通过固定 HTTPS 服务定位时区…", STATUS_INFO);
        let target = window as usize;
        let spawned = std::thread::Builder::new()
            .name("timezone-ip-lookup".into())
            .spawn(move || {
                let result = lookup_timezone();
                if let Ok(mut slot) = GEO_RESULT.lock() {
                    *slot = Some(result);
                }
                unsafe { PostMessageW(target as HWND, WM_GEO_RESULT, 0, 0) };
            });
        if spawned.is_err() {
            unsafe { EnableWindow(self.controls.locate, 1) };
            self.report_error(window, "无法启动定位任务，请稍后重试。");
        }
    }

    fn report_error(&mut self, window: HWND, message: &str) {
        self.controls.set_status(message, STATUS_ERROR);
        show_message(window, message, MB_ICONERROR);
    }
}

fn show_message(window: HWND, message: &str, icon: u32) {
    let message = wide(message);
    let title = wide("Codex 时区启动器");
    unsafe { MessageBoxW(window, message.as_ptr(), title.as_ptr(), MB_OK | icon) };
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
