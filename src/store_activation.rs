use crate::store_launch::{
    RESUME_EVENT_PREFIX, build_debugger_command_line, build_timezone_environment,
};
use std::ffi::c_void;
use std::time::{SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Com::{
    CLSCTX_ALL, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows_sys::core::GUID;

const CLSID_APPLICATION_ACTIVATION_MANAGER: GUID =
    GUID::from_u128(0x45ba127d_10a8_46ea_8ab7_56ea9078943c);
const IID_APPLICATION_ACTIVATION_MANAGER: GUID =
    GUID::from_u128(0x2e941141_7f97_4756_ba1d_9decde894a3d);
const CLSID_PACKAGE_DEBUG_SETTINGS: GUID = GUID::from_u128(0xb1aec16f_2383_4852_b0e9_8f0b1dc66b4d);
const IID_PACKAGE_DEBUG_SETTINGS: GUID = GUID::from_u128(0xf27c3930_8029_4ad1_94e3_3dba417810c1);

pub fn activate(
    package_full_name: &str,
    app_user_model_id: &str,
    timezone: &str,
) -> Result<(), String> {
    let environment = build_timezone_environment(timezone)?;
    let executable = std::env::current_exe().map_err(|_| "无法确定线程恢复器路径。".to_string())?;
    let event_name = unique_event_name()?;
    let debugger_command = build_debugger_command_line(&executable, &event_name)?;
    let event_name_wide = wide(&event_name);
    let event =
        OwnedHandle(unsafe { CreateEventW(std::ptr::null(), 0, 0, event_name_wide.as_ptr()) });
    if event.0.is_null() {
        return Err("无法创建 Store 启动握手事件。".into());
    }

    let _apartment = ComApartment::initialize()?;
    let package_settings = ComObject::create(
        &CLSID_PACKAGE_DEBUG_SETTINGS,
        &IID_PACKAGE_DEBUG_SETTINGS,
        CLSCTX_ALL,
    )?;
    let activation_manager = ComObject::create(
        &CLSID_APPLICATION_ACTIVATION_MANAGER,
        &IID_APPLICATION_ACTIVATION_MANAGER,
        CLSCTX_LOCAL_SERVER,
    )?;

    let package = wide(package_full_name);
    let debugger = wide(&debugger_command);
    unsafe { package_settings.disable_debugging(package.as_ptr()) };
    let enable_result = unsafe {
        package_settings.enable_debugging(package.as_ptr(), debugger.as_ptr(), environment.as_ptr())
    };
    if enable_result < 0 {
        return Err(hresult_message(
            "无法准备 Store 应用时区环境",
            enable_result,
        ));
    }
    let mut debug_guard = DebugSettingsGuard {
        settings: &package_settings,
        package: package.as_ptr(),
        active: true,
    };

    let app_id = wide(app_user_model_id);
    let activation_result = unsafe { activation_manager.activate(app_id.as_ptr()) };
    if activation_result < 0 {
        return Err(hresult_message(
            "无法激活 Codex Store 应用",
            activation_result,
        ));
    }
    if unsafe { WaitForSingleObject(event.0, 10_000) } != WAIT_OBJECT_0 {
        return Err("Store 应用已创建，但启动线程未在 10 秒内恢复。".into());
    }
    debug_guard.disable()?;
    Ok(())
}

fn unique_event_name() -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常，无法创建启动事件。".to_string())?
        .as_nanos();
    Ok(format!(
        "{RESUME_EVENT_PREFIX}{}-{timestamp}",
        std::process::id()
    ))
}

struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, String> {
        let result = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        if result < 0 {
            Err(hresult_message("无法初始化 Store 应用激活", result))
        } else {
            Ok(Self)
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

struct ComObject {
    pointer: *mut c_void,
}

impl ComObject {
    fn create(class_id: &GUID, interface_id: &GUID, context: u32) -> Result<Self, String> {
        let mut pointer = std::ptr::null_mut();
        let result = unsafe {
            CoCreateInstance(
                class_id,
                std::ptr::null_mut(),
                context,
                interface_id,
                &mut pointer,
            )
        };
        if result < 0 || pointer.is_null() {
            Err(hresult_message("无法创建 Store 激活组件", result))
        } else {
            Ok(Self { pointer })
        }
    }

    unsafe fn enable_debugging(
        &self,
        package: *const u16,
        debugger: *const u16,
        environment: *const u16,
    ) -> i32 {
        let table = unsafe { &**(self.pointer as *const *const PackageDebugSettingsVTable) };
        unsafe { (table.enable_debugging)(self.pointer, package, debugger, environment) }
    }

    unsafe fn disable_debugging(&self, package: *const u16) -> i32 {
        let table = unsafe { &**(self.pointer as *const *const PackageDebugSettingsVTable) };
        unsafe { (table.disable_debugging)(self.pointer, package) }
    }

    unsafe fn activate(&self, app_id: *const u16) -> i32 {
        let table = unsafe { &**(self.pointer as *const *const ActivationManagerVTable) };
        let mut process_id = 0u32;
        unsafe {
            (table.activate_application)(self.pointer, app_id, std::ptr::null(), 0, &mut process_id)
        }
    }
}

impl Drop for ComObject {
    fn drop(&mut self) {
        let table = unsafe { &**(self.pointer as *const *const UnknownVTable) };
        unsafe { (table.release)(self.pointer) };
    }
}

struct DebugSettingsGuard<'a> {
    settings: &'a ComObject,
    package: *const u16,
    active: bool,
}

impl DebugSettingsGuard<'_> {
    fn disable(&mut self) -> Result<(), String> {
        let result = unsafe { self.settings.disable_debugging(self.package) };
        if result < 0 {
            Err(hresult_message(
                "Codex 已启动，但无法撤销临时包设置",
                result,
            ))
        } else {
            self.active = false;
            Ok(())
        }
    }
}

impl Drop for DebugSettingsGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            unsafe { self.settings.disable_debugging(self.package) };
        }
    }
}

#[repr(C)]
struct UnknownVTable {
    query_interface: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct PackageDebugSettingsVTable {
    unknown: UnknownVTable,
    enable_debugging:
        unsafe extern "system" fn(*mut c_void, *const u16, *const u16, *const u16) -> i32,
    disable_debugging: unsafe extern "system" fn(*mut c_void, *const u16) -> i32,
}

#[repr(C)]
struct ActivationManagerVTable {
    unknown: UnknownVTable,
    activate_application:
        unsafe extern "system" fn(*mut c_void, *const u16, *const u16, i32, *mut u32) -> i32,
}

fn hresult_message(action: &str, result: i32) -> String {
    format!("{action}（HRESULT 0x{:08X}）。", result as u32)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
