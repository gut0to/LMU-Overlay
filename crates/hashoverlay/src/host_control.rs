#[cfg(not(windows))]
use std::sync::atomic::AtomicBool;
#[cfg(not(windows))]
use std::sync::Arc;

pub const PIPE_NAME: &str = r"\\.\pipe\HashOverlay.Host";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlCommand {
    Status,
    Reload,
    Show,
    Hide,
    Stop,
    Shutdown,
}

impl ControlCommand {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "status" => Some(Self::Status),
            "reload" => Some(Self::Reload),
            "show" => Some(Self::Show),
            "hide" => Some(Self::Hide),
            "stop" => Some(Self::Stop),
            "shutdown" => Some(Self::Shutdown),
            _ => None,
        }
    }
}

#[cfg(windows)]
mod windows_control {
    use super::{ControlCommand, PIPE_NAME};
    use std::{
        ptr,
        sync::atomic::{AtomicBool, Ordering},
        sync::Arc,
        thread::{self, JoinHandle},
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_PIPE_CONNECTED, HANDLE},
        Storage::FileSystem::{FlushFileBuffers, ReadFile, WriteFile, PIPE_ACCESS_DUPLEX},
        System::Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
            PIPE_TYPE_MESSAGE, PIPE_WAIT,
        },
    };

    const BUFFER_SIZE: u32 = 128;

    pub struct HostControl {
        running: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl HostControl {
        pub fn start(
            running: Arc<AtomicBool>,
            visible: Arc<AtomicBool>,
            reload_requested: Arc<AtomicBool>,
        ) -> std::io::Result<Self> {
            let server_running = running.clone();
            let server_visible = visible.clone();
            let handle = thread::Builder::new()
                .name("hashoverlay-host-control".to_string())
                .spawn(move || server_loop(server_running, server_visible, reload_requested))?;
            Ok(Self {
                running,
                handle: Some(handle),
            })
        }

        pub fn shutdown(mut self) {
            self.running.store(false, Ordering::Relaxed);
            let _ = send_command("shutdown");
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    impl Drop for HostControl {
        fn drop(&mut self) {
            self.running.store(false, Ordering::Relaxed);
            let _ = send_command("shutdown");
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn server_loop(
        running: Arc<AtomicBool>,
        visible: Arc<AtomicBool>,
        reload_requested: Arc<AtomicBool>,
    ) {
        while running.load(Ordering::Relaxed) {
            let pipe = unsafe {
                CreateNamedPipeW(
                    wide_null(PIPE_NAME).as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                    1,
                    BUFFER_SIZE,
                    BUFFER_SIZE,
                    0,
                    ptr::null_mut(),
                )
            };
            if pipe.is_null() {
                break;
            }

            let connected = unsafe { ConnectNamedPipe(pipe, ptr::null_mut()) } != 0
                || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
            if !connected {
                unsafe { CloseHandle(pipe) };
                continue;
            }

            let command = read_command(pipe);
            let response = match command.as_deref().and_then(ControlCommand::parse) {
                Some(ControlCommand::Status) => {
                    if running.load(Ordering::Relaxed) {
                        "running"
                    } else {
                        "stopped"
                    }
                }
                Some(ControlCommand::Reload) => {
                    reload_requested.store(true, Ordering::Relaxed);
                    "reloaded"
                }
                Some(ControlCommand::Show) => {
                    visible.store(true, Ordering::Relaxed);
                    "shown"
                }
                Some(ControlCommand::Hide) => {
                    visible.store(false, Ordering::Relaxed);
                    "hidden"
                }
                Some(ControlCommand::Stop) | Some(ControlCommand::Shutdown) => {
                    running.store(false, Ordering::Relaxed);
                    "stopping"
                }
                _ => "invalid",
            };
            let _ = write_response(pipe, response);
            unsafe {
                FlushFileBuffers(pipe);
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
            }
        }
    }

    fn read_command(pipe: HANDLE) -> Option<String> {
        let mut buffer = [0_u8; BUFFER_SIZE as usize];
        let mut read = 0_u32;
        let success = unsafe {
            ReadFile(
                pipe,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
                ptr::null_mut(),
            )
        } != 0;
        success.then(|| {
            String::from_utf8_lossy(&buffer[..read as usize])
                .trim()
                .to_string()
        })
    }

    fn write_response(pipe: HANDLE, response: &str) -> bool {
        let bytes = response.as_bytes();
        let mut written = 0_u32;
        unsafe {
            WriteFile(
                pipe,
                bytes.as_ptr().cast(),
                bytes.len() as u32,
                &mut written,
                ptr::null_mut(),
            ) != 0
                && written == bytes.len() as u32
        }
    }

    pub fn send_command(command: &str) -> std::io::Result<String> {
        use std::io::{Error, ErrorKind};
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING},
        };

        let pipe = unsafe {
            CreateFileW(
                wide_null(PIPE_NAME).as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                ptr::null_mut(),
            )
        };
        if pipe.is_null() || pipe == INVALID_HANDLE_VALUE {
            return Err(Error::new(
                ErrorKind::NotFound,
                "host control pipe is unavailable",
            ));
        }
        let command_result = (|| {
            let bytes = command.as_bytes();
            let mut written = 0_u32;
            if unsafe {
                WriteFile(
                    pipe,
                    bytes.as_ptr().cast(),
                    bytes.len() as u32,
                    &mut written,
                    ptr::null_mut(),
                )
            } == 0
            {
                return Err(Error::last_os_error());
            }
            let mut buffer = [0_u8; BUFFER_SIZE as usize];
            let mut read = 0_u32;
            if unsafe {
                ReadFile(
                    pipe,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut read,
                    ptr::null_mut(),
                )
            } == 0
            {
                return Err(Error::last_os_error());
            }
            Ok(String::from_utf8_lossy(&buffer[..read as usize])
                .trim()
                .to_string())
        })();
        unsafe { CloseHandle(pipe) };
        command_result
    }

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
pub use windows_control::HostControl;

#[cfg(not(windows))]
pub struct HostControl;

#[cfg(not(windows))]
impl HostControl {
    pub fn start(
        _running: Arc<AtomicBool>,
        _visible: Arc<AtomicBool>,
        _reload_requested: Arc<AtomicBool>,
    ) -> std::io::Result<Self> {
        Ok(Self)
    }

    pub fn shutdown(self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_public_control_command() {
        assert_eq!(
            ControlCommand::parse("status"),
            Some(ControlCommand::Status)
        );
        assert_eq!(
            ControlCommand::parse("reload"),
            Some(ControlCommand::Reload)
        );
        assert_eq!(ControlCommand::parse("show"), Some(ControlCommand::Show));
        assert_eq!(ControlCommand::parse("hide"), Some(ControlCommand::Hide));
        assert_eq!(ControlCommand::parse("stop"), Some(ControlCommand::Stop));
        assert_eq!(ControlCommand::parse("invalid"), None);
    }
}
