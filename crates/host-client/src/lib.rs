//! Bounded client transactions for the local host control pipe.
use std::{io, time::Duration};

#[cfg(any(windows, test))]
use std::time::Instant;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

#[cfg(windows)]
mod windows;

pub fn send(command: host_protocol::Command) -> io::Result<String> {
    #[cfg(windows)]
    {
        windows::send_to(host_protocol::PIPE_NAME, command, DEFAULT_TIMEOUT)
    }
    #[cfg(not(windows))]
    {
        let _ = command;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "host control requires Windows",
        ))
    }
}

#[cfg(any(windows, test))]
fn deadline(timeout: Duration) -> io::Result<Instant> {
    if timeout.is_zero() || timeout > Duration::from_secs(30) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "host timeout must be between zero and 30 seconds",
        ));
    }
    Ok(Instant::now() + timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transactions_require_a_finite_positive_budget() {
        assert!(deadline(Duration::ZERO).is_err());
        assert!(deadline(Duration::from_secs(31)).is_err());
        assert!(deadline(DEFAULT_TIMEOUT).is_ok());
    }
}
