use std::ffi::c_void;
use std::io;
use std::os::windows::ffi::OsStrExt as _;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, GENERIC_ALL, GENERIC_READ, HANDLE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, GetAce,
    GetSecurityDescriptorDacl, GetTokenInformation, PROTECTED_DACL_SECURITY_INFORMATION, PSID,
    TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::FILE_READ_DATA;
use windows_sys::Win32::System::SystemServices::ACCESS_ALLOWED_ACE_TYPE;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: these buffers are allocated by Windows APIs using LocalAlloc, or null.
        unsafe { LocalFree(self.0) };
    }
}

struct Token(HANDLE);

impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: this is the owned handle returned by OpenProcessToken.
        unsafe { CloseHandle(self.0) };
    }
}

fn sid_string(sid: PSID) -> io::Result<String> {
    let mut text = null_mut();
    // SAFETY: callers provide a SID in a live token or security descriptor buffer.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let _text = LocalAllocation(text.cast());
    // SAFETY: conversion returned a live, null-terminated UTF-16 string.
    let units = unsafe {
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(text, len)
    };
    String::from_utf16(units).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
}

pub(super) fn current_user_sid() -> io::Result<String> {
    let mut token = null_mut();
    // SAFETY: the process pseudo-handle is valid and token is an output pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Token(token);
    let mut size = 0;
    // SAFETY: a null, zero-sized buffer queries the required length.
    unsafe { GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut size) };
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32) {
        return Err(error);
    }
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    // SAFETY: buffer has TOKEN_USER alignment and at least size writable bytes.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the successful query initialized a TOKEN_USER and its SID in buffer.
    sid_string(unsafe { (*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid })
}

fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    wide.push(0);
    Ok(wide)
}

pub(super) fn set_dacl(path: &Path, sddl: &str) -> io::Result<()> {
    let path = wide_path(path)?;
    let sddl: Vec<_> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = LocalAllocation(null_mut());
    // SAFETY: SDDL is null-terminated and descriptor is a writable output pointer.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor.0,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl = null_mut();
    // SAFETY: descriptor owns a valid security descriptor; all outputs are writable.
    if unsafe { GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut dacl, &mut defaulted) }
        == 0
    {
        return Err(io::Error::last_os_error());
    }
    if present == 0 {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    // SAFETY: path is null-terminated and dacl remains live inside descriptor.
    let status = unsafe {
        SetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            dacl,
            null(),
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    Ok(())
}

pub(crate) fn restrict_to_owner(path: &Path, dir: bool) -> io::Result<()> {
    let sid = current_user_sid()?;
    let flags = if dir { "OICI" } else { "" };
    set_dacl(path, &format!("D:P(A;{flags};FA;;;{sid})"))
}

fn read_dacl(path: &Path) -> io::Result<(LocalAllocation, *mut ACL)> {
    let path = wide_path(path)?;
    let mut descriptor = LocalAllocation(null_mut());
    let mut dacl = null_mut();
    // SAFETY: path is null-terminated and all requested outputs are writable.
    let status = unsafe {
        GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor.0,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    Ok((descriptor, dacl))
}

pub(crate) fn others_can_read(path: &Path) -> io::Result<bool> {
    let owner = current_user_sid()?;
    let (_descriptor, dacl) = read_dacl(path)?;
    if dacl.is_null() {
        return Ok(true);
    }
    // SAFETY: dacl is live inside the security descriptor returned by Windows.
    for index in 0..unsafe { (*dacl).AceCount } {
        let mut ace = null_mut();
        // SAFETY: index is within the ACL's ACE count; ace is a writable output.
        if unsafe { GetAce(dacl, u32::from(index), &mut ace) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: GetAce returned a live ACE with an ACE_HEADER.
        if unsafe { (*ace.cast::<ACE_HEADER>()).AceType } != ACCESS_ALLOWED_ACE_TYPE as u8 {
            continue;
        }
        // SAFETY: the ACE type identifies an ACCESS_ALLOWED_ACE with an inline SID.
        let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
        if allowed.Mask & (FILE_READ_DATA | GENERIC_READ | GENERIC_ALL) == 0 {
            continue;
        }
        let sid = sid_string(std::ptr::addr_of!(allowed.SidStart).cast_mut().cast())?;
        if sid != owner && sid != "S-1-5-18" && sid != "S-1-5-32-544" {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
pub(super) fn dacl_sddl(path: &Path) -> io::Result<String> {
    use windows_sys::Win32::Security::Authorization::ConvertSecurityDescriptorToStringSecurityDescriptorW;

    let (descriptor, _) = read_dacl(path)?;
    let mut text = null_mut();
    // SAFETY: descriptor is live and text is a writable output pointer.
    if unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor.0,
            1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let _text = LocalAllocation(text.cast());
    // SAFETY: conversion returned a live, null-terminated UTF-16 string.
    let units = unsafe {
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(text, len)
    };
    String::from_utf16(units).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
}

#[cfg(test)]
pub(super) fn dacl_is_protected(path: &Path) -> io::Result<bool> {
    use windows_sys::Win32::Security::{GetSecurityDescriptorControl, SE_DACL_PROTECTED};

    let (descriptor, _) = read_dacl(path)?;
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: descriptor is live and the control/revision outputs are writable.
    if unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(control & SE_DACL_PROTECTED != 0)
}
