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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_command_round_trips() {
        for command in Command::ALL { assert_eq!(Command::parse(command.as_str()), Some(command)); }
        assert_eq!(Command::parse("unknown"), None);
    }
}
