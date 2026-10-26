use std::{io, ptr, time::Instant};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, ERROR_IO_PENDING, WAIT_TIMEOUT},
    System::{IO::{CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED}, Threading::CreateEventW},
};

struct Handle(HANDLE);
impl Handle {
    fn new(raw: HANDLE) -> io::Result<Self> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE { Err(io::Error::last_os_error()) } else { Ok(Self(raw)) }
    }
}
impl Drop for Handle { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }

struct Operation<'a> {
    pipe: &'a Handle,
    overlapped: OVERLAPPED,
    _event: Handle,
    pending: bool,
}
impl<'a> Operation<'a> {
    fn new(pipe: &'a Handle) -> io::Result<Self> {
        let event = Handle::new(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })?;
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlapped.hEvent = event.0;
        Ok(Self { pipe, overlapped, _event: event, pending: false })
    }

    fn finish(&mut self, succeeded: i32, deadline: Instant) -> io::Result<u32> {
        if succeeded == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) { return Err(error); }
            self.pending = true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let milliseconds = remaining.as_millis().clamp(1, u128::from(u32::MAX - 1)) as u32;
        let mut transferred = 0;
        let success = unsafe { GetOverlappedResultEx(self.pipe.0, &self.overlapped, &mut transferred, milliseconds, 0) };
        if success == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(WAIT_TIMEOUT as i32) {
                self.cancel();
                return Err(io::Error::new(io::ErrorKind::TimedOut, "overlay host did not respond within the transaction deadline"));
            }
            self.pending = false;
            return Err(error);
        }
        self.pending = false;
        Ok(transferred)
    }

    fn cancel(&mut self) {
        if self.pending {
            // Cancellation alone is not completion: retain the OVERLAPPED and IO
            // buffer until Windows has finished the canceled request.
            unsafe {
                CancelIoEx(self.pipe.0, &self.overlapped);
                let mut transferred = 0;
                GetOverlappedResult(self.pipe.0, &self.overlapped, &mut transferred, 1);
            }
            self.pending = false;
        }
    }
}
impl Drop for Operation<'_> { fn drop(&mut self) { self.cancel(); } }

fn open_pipe(name: &str, deadline: Instant) -> io::Result<Handle> {
    use windows_sys::Win32::{
        Foundation::{GENERIC_READ, GENERIC_WRITE, ERROR_PIPE_BUSY},
        Storage::FileSystem::{CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING},
        System::Pipes::WaitNamedPipeW,
    };
    if name.contains('\0') { return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid pipe name")); }
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    loop {
        if Instant::now() >= deadline { return Err(io::Error::new(io::ErrorKind::TimedOut, "overlay host pipe remained busy")); }
        let raw = unsafe { CreateFileW(wide.as_ptr(), GENERIC_READ | GENERIC_WRITE, 0, ptr::null(), OPEN_EXISTING, FILE_FLAG_OVERLAPPED, ptr::null_mut()) };
        if raw != INVALID_HANDLE_VALUE && !raw.is_null() { return Ok(Handle(raw)); }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) { return Err(error); }
        let remaining = deadline.saturating_duration_since(Instant::now()).as_millis().clamp(1, 50) as u32;
        unsafe { WaitNamedPipeW(wide.as_ptr(), remaining); }
    }
}

fn read_reply(pipe: &Handle, deadline: Instant) -> io::Result<String> {
    use windows_sys::Win32::Storage::FileSystem::ReadFile;
    let mut bytes = [0_u8; host_protocol::MAX_FRAME_BYTES + 1];
    let mut operation = Operation::new(pipe)?;
    let succeeded = unsafe { ReadFile(pipe.0, bytes.as_mut_ptr().cast(), bytes.len() as u32, ptr::null_mut(), &mut operation.overlapped) };
    let count = operation.finish(succeeded, deadline)? as usize;
    host_protocol::decode_frame(&bytes[..count]).map(str::to_owned).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_command(pipe: &Handle, command: host_protocol::Command, deadline: Instant) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::WriteFile;
    let bytes = command.as_str().as_bytes();
    let mut operation = Operation::new(pipe)?;
    let succeeded = unsafe { WriteFile(pipe.0, bytes.as_ptr().cast(), bytes.len() as u32, ptr::null_mut(), &mut operation.overlapped) };
    let count = operation.finish(succeeded, deadline)? as usize;
    if count != bytes.len() { return Err(io::Error::new(io::ErrorKind::WriteZero, "partial host command write")); }
    Ok(())
}

pub(super) fn send_to(name: &str, command: host_protocol::Command, timeout: std::time::Duration) -> io::Result<String> {
    use windows_sys::Win32::System::Pipes::{SetNamedPipeHandleState, PIPE_READMODE_MESSAGE};
    let deadline = super::deadline(timeout)?;
    let pipe = open_pipe(name, deadline)?;
    let mode = PIPE_READMODE_MESSAGE;
    if unsafe { SetNamedPipeHandleState(pipe.0, &mode, ptr::null(), ptr::null()) } == 0 { return Err(io::Error::last_os_error()); }
    write_command(&pipe, command, deadline)?;
    read_reply(&pipe, deadline)
}
