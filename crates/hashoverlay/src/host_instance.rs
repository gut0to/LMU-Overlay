use anyhow::Result;

#[cfg(windows)]
use std::ptr::null_mut;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
    System::Threading::CreateMutexW,
};

/// Process-wide guard for the normal overlay host. A second Settings launch
/// can safely ask the existing host for status instead of starting another
/// telemetry pipeline.
pub struct HostInstance {
    #[cfg(windows)]
    handle: HANDLE,
}

impl HostInstance {
    pub fn acquire() -> Result<Option<Self>> {
        #[cfg(windows)]
        unsafe {
            let name: Vec<u16> = "Local\\HashOverlay.Host"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let handle = CreateMutexW(null_mut(), 0, name.as_ptr());
            if handle.is_null() {
                anyhow::bail!("could not create HashOverlay host mutex");
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return Ok(None);
            }
            return Ok(Some(Self { handle }));
        }

        #[cfg(not(windows))]
        Ok(Some(Self {}))
    }
}

#[cfg(windows)]
impl Drop for HostInstance {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}
