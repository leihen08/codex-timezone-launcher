use crate::process::running_processes;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, FILETIME};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
};

const PACKAGE_REGISTRY_PATH: &str = concat!(
    "Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\",
    "AppModel\\Repository\\Packages"
);

pub fn select_first_existing(
    candidates: &[PathBuf],
    exists: impl Fn(&std::path::Path) -> bool,
) -> Option<PathBuf> {
    candidates.iter().find(|path| exists(path)).cloned()
}

pub fn candidate_paths_from_roots(local_app_data: &Path, program_files: &Path) -> Vec<PathBuf> {
    [
        local_app_data.join(r"Programs\ChatGPT\ChatGPT.exe"),
        local_app_data.join(r"Programs\Codex\Codex.exe"),
        local_app_data.join(r"OpenAI\ChatGPT\ChatGPT.exe"),
        local_app_data.join(r"OpenAI\Codex\Codex.exe"),
        program_files.join(r"ChatGPT\ChatGPT.exe"),
        program_files.join(r"Codex\Codex.exe"),
    ]
    .into()
}

pub fn package_executable_candidates(package_root: &Path) -> Vec<PathBuf> {
    [
        package_root.join(r"app\ChatGPT.exe"),
        package_root.join("ChatGPT.exe"),
        package_root.join(r"app\Codex.exe"),
        package_root.join("Codex.exe"),
    ]
    .into()
}

pub fn discover_client() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Ok(processes) = running_processes() {
        candidates.extend(processes.into_iter().filter_map(|process| process.path));
    }
    for package_root in app_model_package_roots() {
        candidates.extend(package_executable_candidates(&package_root));
    }
    candidates.extend(app_paths_candidates());

    if let (Some(local), Some(programs)) = (
        std::env::var_os("LOCALAPPDATA"),
        std::env::var_os("ProgramFiles"),
    ) {
        candidates.extend(candidate_paths_from_roots(
            Path::new(&local),
            Path::new(&programs),
        ));
    }

    select_first_existing(&candidates, |path| {
        path.is_file()
            && path.file_name().is_some_and(|name| {
                name.eq_ignore_ascii_case("ChatGPT.exe") || name.eq_ignore_ascii_case("Codex.exe")
            })
    })
    .ok_or_else(|| "未找到已安装的 Codex/ChatGPT Windows 客户端。".into())
}

fn app_model_package_roots() -> Vec<PathBuf> {
    let mut package_key: HKEY = std::ptr::null_mut();
    let registry_path = wide(PACKAGE_REGISTRY_PATH);
    if unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            registry_path.as_ptr(),
            0,
            KEY_READ,
            &mut package_key,
        )
    } != ERROR_SUCCESS
    {
        return Vec::new();
    }

    let mut roots = Vec::new();
    for index in 0..4096 {
        let mut name = [0u16; 512];
        let mut name_length = name.len() as u32;
        let mut modified = FILETIME::default();
        let status = unsafe {
            RegEnumKeyExW(
                package_key,
                index,
                name.as_mut_ptr(),
                &mut name_length,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut modified,
            )
        };
        if status != ERROR_SUCCESS {
            break;
        }
        let package_name = String::from_utf16_lossy(&name[..name_length as usize]);
        let lower = package_name.to_ascii_lowercase();
        if (lower.starts_with("openai.codex_") || lower.starts_with("openai.chatgpt_"))
            && let Some(root) =
                read_registry_string(package_key, Some(&package_name), "PackageRootFolder")
        {
            roots.push(PathBuf::from(root));
        }
    }
    unsafe { RegCloseKey(package_key) };
    roots.reverse();
    roots
}

fn app_paths_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for executable in ["ChatGPT.exe", "Codex.exe"] {
            let key =
                format!("Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{executable}");
            if let Some(path) = read_registry_string(root, Some(&key), "") {
                paths.push(PathBuf::from(path));
            }
        }
    }
    paths
}

fn read_registry_string(root: HKEY, subkey: Option<&str>, value: &str) -> Option<String> {
    let subkey = subkey.map(wide);
    let value = wide(value);
    let subkey_pointer = subkey
        .as_ref()
        .map_or(std::ptr::null(), |text| text.as_ptr());
    let value_pointer = if value == [0] {
        std::ptr::null()
    } else {
        value.as_ptr()
    };
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
    let mut bytes = 0u32;
    if unsafe {
        RegGetValueW(
            root,
            subkey_pointer,
            value_pointer,
            flags,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        )
    } != ERROR_SUCCESS
        || !(2..=32_768).contains(&bytes)
    {
        return None;
    }

    let mut buffer = vec![0u16; bytes.div_ceil(2) as usize];
    if unsafe {
        RegGetValueW(
            root,
            subkey_pointer,
            value_pointer,
            flags,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast::<c_void>(),
            &mut bytes,
        )
    } != ERROR_SUCCESS
    {
        return None;
    }
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..length]))
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}
