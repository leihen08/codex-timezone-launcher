use crate::timezone::validate_timezone;
use serde::Deserialize;
use std::ffi::c_void;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Networking::WinHttp::{
    ERROR_WINHTTP_CANNOT_CONNECT, ERROR_WINHTTP_NAME_NOT_RESOLVED, ERROR_WINHTTP_TIMEOUT,
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_NEVER,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpQueryHeaders,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetOption,
    WinHttpSetTimeouts,
};

pub const IP_SERVICE_URL: &str = "https://ipapi.co/json/";
const IP_SERVICE_HOST: &str = "ipapi.co";
const IP_SERVICE_PATH: &str = "/json/";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
struct IpApiResponse {
    timezone: Option<String>,
    #[serde(default)]
    error: bool,
}

pub fn parse_timezone_response(body: &[u8]) -> Result<String, String> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err("定位服务返回的数据异常过大。".into());
    }
    let response: IpApiResponse =
        serde_json::from_slice(body).map_err(|_| "定位服务返回了无法解析的数据。".to_string())?;
    if response.error {
        return Err("定位服务暂时不可用，请稍后重试。".into());
    }
    let timezone = response
        .timezone
        .ok_or_else(|| "定位服务没有返回时区。".to_string())?;
    validate_timezone(&timezone).map_err(|_| "定位服务返回了无效时区。".into())
}

pub fn timezone_from_http_response(status: u32, body: &[u8]) -> Result<String, String> {
    if !(200..300).contains(&status) {
        return Err(format!("定位服务返回 HTTP {status}，请稍后重试。"));
    }
    parse_timezone_response(body)
}

pub fn lookup_timezone() -> Result<String, String> {
    let body_and_status = fetch_fixed_endpoint()?;
    timezone_from_http_response(body_and_status.0, &body_and_status.1)
}

fn fetch_fixed_endpoint() -> Result<(u32, Vec<u8>), String> {
    let agent = wide("ChatGPTTimeZoneLauncher/1.0");
    let session = InternetHandle::new(unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    })
    .ok_or_else(|| network_error("初始化定位请求"))?;
    if unsafe { WinHttpSetTimeouts(session.0, 4_000, 5_000, 5_000, 8_000) } == 0 {
        return Err(network_error("设置定位超时"));
    }

    let host = wide(IP_SERVICE_HOST);
    let connection = InternetHandle::new(unsafe {
        WinHttpConnect(session.0, host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0)
    })
    .ok_or_else(|| network_error("连接定位服务"))?;

    let method = wide("GET");
    let path = wide(IP_SERVICE_PATH);
    let request = InternetHandle::new(unsafe {
        WinHttpOpenRequest(
            connection.0,
            method.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    })
    .ok_or_else(|| network_error("创建定位请求"))?;

    let redirect_policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
    if unsafe {
        WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_REDIRECT_POLICY,
            (&redirect_policy as *const u32).cast::<c_void>(),
            std::mem::size_of::<u32>() as u32,
        )
    } == 0
    {
        return Err(network_error("限制定位请求"));
    }

    if unsafe { WinHttpSendRequest(request.0, std::ptr::null(), 0, std::ptr::null(), 0, 0, 0) } == 0
    {
        return Err(network_error("发送定位请求"));
    }
    if unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) } == 0 {
        return Err(network_error("接收定位响应"));
    }

    let status = query_status(request.0)?;
    let body = read_response(request.0)?;
    Ok((status, body))
}

fn query_status(request: *mut c_void) -> Result<u32, String> {
    let mut status = 0u32;
    let mut status_size = std::mem::size_of::<u32>() as u32;
    let mut index = 0u32;
    if unsafe {
        WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            std::ptr::null(),
            (&mut status as *mut u32).cast::<c_void>(),
            &mut status_size,
            &mut index,
        )
    } == 0
    {
        Err(network_error("读取定位状态"))
    } else {
        Ok(status)
    }
}

fn read_response(request: *mut c_void) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    loop {
        let mut available = 0u32;
        if unsafe { WinHttpQueryDataAvailable(request, &mut available) } == 0 {
            return Err(network_error("读取定位数据"));
        }
        if available == 0 {
            return Ok(body);
        }
        if body.len().saturating_add(available as usize) > MAX_RESPONSE_BYTES {
            return Err("定位服务返回的数据异常过大。".into());
        }

        let start = body.len();
        body.resize(start + available as usize, 0);
        let mut read = 0u32;
        if unsafe {
            WinHttpReadData(
                request,
                body[start..].as_mut_ptr().cast::<c_void>(),
                available,
                &mut read,
            )
        } == 0
        {
            return Err(network_error("读取定位数据"));
        }
        body.truncate(start + read as usize);
    }
}

fn network_error(action: &str) -> String {
    let code = unsafe { GetLastError() };
    if code == ERROR_WINHTTP_TIMEOUT {
        "定位请求超时，请检查网络后重试。".into()
    } else if code == ERROR_WINHTTP_CANNOT_CONNECT || code == ERROR_WINHTTP_NAME_NOT_RESOLVED {
        "无法连接定位服务，请检查网络后重试。".into()
    } else {
        format!("{action}失败，请稍后重试。")
    }
}

struct InternetHandle(*mut c_void);

impl InternetHandle {
    fn new(handle: *mut c_void) -> Option<Self> {
        (!handle.is_null()).then_some(Self(handle))
    }
}

impl Drop for InternetHandle {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
