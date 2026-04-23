use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "MKAC";

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

fn open_run_key(samdesired: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
    let subkey = to_wide(RUN_KEY);
    let mut hkey = HKEY::default();
    let err = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            samdesired,
            None,
            &mut hkey,
            None,
        )
    };
    if err.is_ok() { Some(hkey) } else { None }
}

pub fn is_enabled() -> bool {
    let Some(hkey) = open_run_key(KEY_READ) else {
        return false;
    };
    let valname = to_wide(VALUE_NAME);
    let mut data_len: u32 = 0;
    let result = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(valname.as_ptr()),
            None,
            None,
            None,
            Some(&mut data_len),
        )
    };
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    result.is_ok() && data_len > 0
}

pub fn enable() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy().into_owned();
    let value = format!("\"{}\" --minimized", exe_str);
    let value_wide = to_wide(&value);

    let hkey = open_run_key(KEY_SET_VALUE)
        .ok_or_else(|| anyhow::anyhow!("could not open Run key"))?;
    let valname = to_wide(VALUE_NAME);

    let byte_len = value_wide.len() * 2;
    let data = unsafe { std::slice::from_raw_parts(value_wide.as_ptr() as *const u8, byte_len) };

    let err = unsafe {
        RegSetValueExW(
            hkey,
            PCWSTR(valname.as_ptr()),
            None,
            REG_SZ,
            Some(data),
        )
    };
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    err.ok()?;
    Ok(())
}

pub fn disable() -> anyhow::Result<()> {
    let hkey = open_run_key(KEY_SET_VALUE)
        .ok_or_else(|| anyhow::anyhow!("could not open Run key"))?;
    let valname = to_wide(VALUE_NAME);
    unsafe {
        let _ = RegDeleteValueW(hkey, PCWSTR(valname.as_ptr()));
        let _ = RegCloseKey(hkey);
    }
    Ok(())
}

pub fn set(enabled: bool) -> anyhow::Result<()> {
    if enabled {
        enable()
    } else {
        disable()
    }
}
