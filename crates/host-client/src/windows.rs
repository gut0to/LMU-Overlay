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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::{atomic::{AtomicU64, Ordering}, mpsc}, thread, time::Duration};
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_PIPE_CONNECTED},
        Storage::FileSystem::{ReadFile, WriteFile, FlushFileBuffers, PIPE_ACCESS_DUPLEX},
        System::Pipes::{CreateNamedPipeW, ConnectNamedPipe, PIPE_TYPE_MESSAGE, PIPE_READMODE_MESSAGE, PIPE_WAIT},
    };
    static NEXT_PIPE: AtomicU64 = AtomicU64::new(0);

    fn serve(reply: Option<Vec<u8>>, delay: Duration) -> (String, thread::JoinHandle<()>) {
        let name = format!(r"\\.\pipe\HashOverlay.Test.{}.{}", std::process::id(), NEXT_PIPE.fetch_add(1, Ordering::Relaxed));
        let server_name = name.clone();
        let (ready, wait) = mpsc::channel();
        let worker = thread::spawn(move || {
            let wide: Vec<_> = server_name.encode_utf16().chain(Some(0)).collect();
            let pipe = Handle::new(unsafe { CreateNamedPipeW(wide.as_ptr(), PIPE_ACCESS_DUPLEX, PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT, 1, 512, 512, 0, ptr::null()) }).unwrap();
            ready.send(()).unwrap();
            let connected = unsafe { ConnectNamedPipe(pipe.0, ptr::null_mut()) } != 0 || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
            assert!(connected);
            let mut command = [0_u8; 128];
            let mut count = 0;
            assert_ne!(unsafe { ReadFile(pipe.0, command.as_mut_ptr().cast(), 128, &mut count, ptr::null_mut()) }, 0);
            assert!(host_protocol::Command::parse(std::str::from_utf8(&command[..count as usize]).unwrap()).is_some());
            thread::sleep(delay);
            if let Some(reply) = reply {
                let mut written = 0;
                unsafe { WriteFile(pipe.0, reply.as_ptr().cast(), reply.len() as u32, &mut written, ptr::null_mut()); FlushFileBuffers(pipe.0); }
            }
        });
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        (name, worker)
    }

    #[test]
    fn real_pipe_round_trips_complete_before_client_handles_are_released() {
        for _ in 0..10 {
            let (name, server) = serve(Some(b"running".to_vec()), Duration::ZERO);
            assert_eq!(send_to(&name, host_protocol::Command::Status, Duration::from_secs(2)).unwrap(), "running");
            server.join().unwrap();
        }
    }
}
