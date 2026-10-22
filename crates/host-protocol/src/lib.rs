//! Shared wire vocabulary for the local overlay control pipe.
pub const PIPE_NAME: &str = r"\\.\pipe\HashOverlay.Host";
pub const MAX_FRAME_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command { Status, Reload, Show, Hide, ToggleEdit, Stop, EditStatus, ToggleVisibility, VisibilityStatus, Shutdown }

impl Command {
    pub const ALL: [Self; 10] = [Self::Status, Self::Reload, Self::Show, Self::Hide, Self::ToggleEdit, Self::Stop, Self::EditStatus, Self::ToggleVisibility, Self::VisibilityStatus, Self::Shutdown];
    pub fn parse(value: &str) -> Option<Self> { Self::ALL.into_iter().find(|command| command.as_str() == value) }
    pub const fn as_str(self) -> &'static str {
        match self { Self::Status => "status", Self::Reload => "reload", Self::Show => "show", Self::Hide => "hide", Self::ToggleEdit => "edit", Self::Stop => "stop", Self::EditStatus => "edit_status", Self::ToggleVisibility => "toggle_visibility", Self::VisibilityStatus => "visibility_status", Self::Shutdown => "shutdown" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError { Empty, TooLong, InvalidUtf8, ControlCharacter }
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Empty => "empty host frame", Self::TooLong => "host frame exceeds the wire limit", Self::InvalidUtf8 => "host frame is not UTF-8", Self::ControlCharacter => "host frame contains control characters" })
    }
}
impl std::error::Error for FrameError {}

pub fn decode_frame(bytes: &[u8]) -> Result<&str, FrameError> {
    if bytes.len() > MAX_FRAME_BYTES { return Err(FrameError::TooLong); }
    let text = std::str::from_utf8(bytes).map_err(|_| FrameError::InvalidUtf8)?.trim();
    if text.is_empty() { return Err(FrameError::Empty); }
    if text.chars().any(char::is_control) { return Err(FrameError::ControlCharacter); }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_require_bounded_single_line_utf8() {
        assert_eq!(decode_frame(b" status\r\n"), Ok("status"));
        assert_eq!(decode_frame(b""), Err(FrameError::Empty));
        assert_eq!(decode_frame(&[0xff]), Err(FrameError::InvalidUtf8));
        assert_eq!(decode_frame(b"status\0"), Err(FrameError::ControlCharacter));
        assert_eq!(decode_frame(b"status\nstop"), Err(FrameError::ControlCharacter));
        assert_eq!(decode_frame(&[b'x'; 129]), Err(FrameError::TooLong));
    }

    #[test]
    fn every_command_round_trips() {
        for command in Command::ALL { assert_eq!(Command::parse(command.as_str()), Some(command)); }
        assert_eq!(Command::parse("unknown"), None);
    }
}
