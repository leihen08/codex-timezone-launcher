use crate::config::{Settings, load_settings, save_settings};
use crate::discovery::discover_client;
use crate::geo::lookup_timezone;
use crate::timezone::{DEFAULT_TIMEZONE, all_timezones, matching_timezones};
use crate::ui_controls::{STATUS_ERROR, STATUS_INFO, STATUS_SUCCESS, STATUS_WARNING, UiControls};
use crate::ui_layout::{IDC_LOCATE, IDC_SAVE, IDC_SAVE_LAUNCH, IDC_TIMEZONE};
use crate::workflow::{SaveLaunchOutcome, save_and_launch};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::Graphics::Gdi::{HBRUSH, HDC};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, CBN_EDITCHANGE, KillTimer, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK,
    MessageBoxW, PostMessageW, SetTimer, WM_APP,
};

pub const WM_GEO_RESULT: u32 = WM_APP + 1;
pub const WM_LAUNCH_RESULT: u32 = WM_APP + 2;
pub const TIMEZONE_SEARCH_TIMER: usize = 1;
static GEO_RESULT: Mutex<Option<Result<String, String>>> = Mutex::new(None);
static LAUNCH_RESULT: Mutex<Option<Result<SaveLaunchOutcome, String>>> = Mutex::new(None);

pub struct AppState {
    zones: Vec<String>,
    controls: UiControls,
    filtering_timezones: bool,
    launch_in_progress: bool,
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
        Ok(Self {
            zones,
            controls,
            filtering_timezones: false,
            launch_in_progress: false,
        })
    }

    pub fn handle_command(&mut self, window: HWND, control_id: i32, notification: u32) {
        if control_id == IDC_TIMEZONE && notification == CBN_EDITCHANGE {
            if !self.filtering_timezones {
                unsafe {
                    KillTimer(window, TIMEZONE_SEARCH_TIMER);
                    SetTimer(window, TIMEZONE_SEARCH_TIMER, 250, None);
                }
            }
            return;
        }
        if notification != BN_CLICKED {
            return;
        }
        match control_id {
            IDC_LOCATE => self.start_geo_lookup(window),
            IDC_SAVE => self.save_selected(window),
            IDC_SAVE_LAUNCH => self.start_launch(window),
            _ => {}
        }
    }

    pub fn handle_geo_result(&mut self, window: HWND) {
        unsafe { EnableWindow(self.controls.locate, 1) };
        let result = GEO_RESULT.lock().ok().and_then(|mut slot| slot.take());
        match result {
            Some(Ok(timezone)) => {
                if let Some(index) = self.zones.iter().position(|zone| zone == &timezone) {
                    self.filtering_timezones = true;
                    self.controls.select_timezone(&self.zones, index);
                    self.filtering_timezones = false;
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

    pub fn handle_timer(&mut self, window: HWND, timer_id: usize) {
        if timer_id == TIMEZONE_SEARCH_TIMER {
            unsafe { KillTimer(window, TIMEZONE_SEARCH_TIMER) };
            self.filter_timezone_list();
        }
    }

    pub fn handle_launch_result(&mut self, window: HWND) {
        self.launch_in_progress = false;
        unsafe { EnableWindow(self.controls.save_launch, 1) };
        let result = LAUNCH_RESULT.lock().ok().and_then(|mut slot| slot.take());
        match result {
            Some(Ok(SaveLaunchOutcome::AlreadyRunning(_))) => {
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
            Some(Ok(SaveLaunchOutcome::Launched(_))) => {
                self.controls
                    .set_status("Codex 已使用所选时区启动。", STATUS_SUCCESS);
                show_message(
                    window,
                    "Codex 已启动；Windows 系统时区没有改变。",
                    MB_ICONINFORMATION,
                );
            }
            Some(Err(message)) => self.report_error(window, &message),
            None => self.report_error(window, "未能取得启动结果，请重试。"),
        }
    }

    pub fn can_close(&self, window: HWND) -> bool {
        if self.launch_in_progress {
            show_message(
                window,
                "正在完成 Store 应用启动与临时设置清理，请稍候。",
                MB_ICONINFORMATION,
            );
            false
        } else {
            true
        }
    }

    fn selected_timezone(&self) -> Result<String, String> {
        self.controls.timezone_text()
    }

    fn save_selected(&mut self, window: HWND) {
        let result = self
            .selected_timezone()
            .and_then(|timezone| Settings::new(&timezone))
            .and_then(|settings| save_settings(&settings).map(|_| settings.timezone));
        match result {
            Ok(timezone) => self.controls.set_status(
                &format!("已保存 {timezone}，下次打开会自动选中。"),
                STATUS_SUCCESS,
            ),
            Err(message) => self.report_error(window, &message),
        }
    }

    fn start_launch(&mut self, window: HWND) {
        let timezone = match self.selected_timezone() {
            Ok(timezone) => timezone,
            Err(message) => {
                self.report_error(window, &message);
                return;
            }
        };
        if let Ok(mut slot) = LAUNCH_RESULT.lock() {
            *slot = None;
        }
        self.launch_in_progress = true;
        unsafe { EnableWindow(self.controls.save_launch, 0) };
        self.controls
            .set_status("正在保存设置并启动 Codex…", STATUS_INFO);
        let target = window as usize;
        let spawned = std::thread::Builder::new()
            .name("codex-store-launch".into())
            .spawn(move || {
                let result = save_and_launch(&timezone);
                if let Ok(mut slot) = LAUNCH_RESULT.lock() {
                    *slot = Some(result);
                }
                unsafe { PostMessageW(target as HWND, WM_LAUNCH_RESULT, 0, 0) };
            });
        if spawned.is_err() {
            self.launch_in_progress = false;
            unsafe { EnableWindow(self.controls.save_launch, 1) };
            self.report_error(window, "无法启动后台启动任务，请重试。");
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

    fn filter_timezone_list(&mut self) {
        if self.filtering_timezones {
            return;
        }
        let Ok(query) = self.controls.timezone_text() else {
            return;
        };
        let matches = matching_timezones(&self.zones, &query);
        self.filtering_timezones = true;
        self.controls.filter_timezones(&matches, &query);
        self.filtering_timezones = false;
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
