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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply { Running, Stopped, Stopping, Reloaded, Shown, Hidden, EditOn, EditOff, Invalid, Error(String) }
impl Reply {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value { "running" => Self::Running, "stopped" => Self::Stopped, "stopping" => Self::Stopping, "reloaded" => Self::Reloaded, "shown" => Self::Shown, "hidden" => Self::Hidden, "edit_on" => Self::EditOn, "edit_off" => Self::EditOff, "invalid" => Self::Invalid,
            other => Self::Error(other.strip_prefix("error:").filter(|detail| !detail.trim().is_empty())?.to_string()) })
    }
    pub fn visibility(&self) -> Option<bool> { match self { Self::Shown => Some(true), Self::Hidden => Some(false), _ => None } }
    pub fn edit_mode(&self) -> Option<bool> { match self { Self::EditOn => Some(true), Self::EditOff => Some(false), _ => None } }
}

/// Keep error replies within one pipe frame, including multibyte messages.
pub fn error_response(detail: &str) -> String {
    let clean: String = detail.chars().map(|ch| if ch.is_control() { ' ' } else { ch }).collect();
    let clean = if clean.trim().is_empty() { "host error" } else { clean.trim() };
    let limit = MAX_FRAME_BYTES - "error:".len();
    if clean.len() <= limit { return format!("error:{clean}"); }
    let mut end = limit - 3;
    while !clean.is_char_boundary(end) { end -= 1; }
    format!("error:{}...", &clean[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_replies_fit_the_wire_limit_and_remain_valid_utf8() {
        let reply = error_response(&"é".repeat(200));
        assert!(reply.len() <= MAX_FRAME_BYTES);
        assert!(decode_frame(reply.as_bytes()).is_ok());
        assert!(reply.ends_with("..."));
        assert_eq!(error_response("line\nbreak\0"), "error:line break");
    }

    #[test]
    fn reply_states_are_explicit_not_truthy_strings() {
        assert_eq!(Reply::parse("shown").unwrap().visibility(), Some(true));
        assert_eq!(Reply::parse("hidden").unwrap().visibility(), Some(false));
        assert_eq!(Reply::parse("running").unwrap().edit_mode(), None);
        assert_eq!(Reply::parse("error:disk full"), Some(Reply::Error("disk full".into())));
        assert_eq!(Reply::parse("error:"), None);
        assert_eq!(Reply::parse("running now"), None);
    }

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
