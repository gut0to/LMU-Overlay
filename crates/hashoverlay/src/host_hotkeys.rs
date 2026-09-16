#[cfg(not(windows))]
use std::sync::mpsc::Sender;

use overlay_renderer::config::HotkeyConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostAction {
    ToggleVisibility,
    ToggleEditMode,
    ToggleCoaching,
    CyclePreset,
}

#[cfg(windows)]
mod windows_hotkeys {
    use super::{HostAction, HotkeyConfig};
    use std::{
        sync::{
            atomic::{AtomicU32, Ordering},
            mpsc::Sender,
            Arc,
        },
        thread::{self, JoinHandle},
    };
    use windows_sys::Win32::{
        System::Threading::GetCurrentThreadId,
        UI::{
            Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT,
            },
            WindowsAndMessaging::{GetMessageW, PostThreadMessageW, MSG, WM_HOTKEY, WM_QUIT},
        },
    };

    const TOGGLE_VISIBILITY: i32 = 1;
    const TOGGLE_EDIT: i32 = 2;
    const TOGGLE_COACHING: i32 = 3;
    const CYCLE_PRESET: i32 = 4;

    pub struct HostHotkeys {
        thread_id: Arc<AtomicU32>,
        join: Option<JoinHandle<()>>,
    }

    impl HostHotkeys {
        pub fn start(config: HotkeyConfig, actions: Sender<HostAction>) -> Self {
            let thread_id = Arc::new(AtomicU32::new(0));
            let worker_id = thread_id.clone();
            let join = thread::spawn(move || unsafe {
                worker_id.store(GetCurrentThreadId(), Ordering::Relaxed);
                register(TOGGLE_VISIBILITY, &config.toggle_overlay);
                register(TOGGLE_EDIT, &config.edit_mode);
                register(TOGGLE_COACHING, &config.toggle_coaching);
                register(CYCLE_PRESET, &config.cycle_preset);
                let mut message: MSG = std::mem::zeroed();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    if message.message == WM_HOTKEY {
                        let action = match message.wParam as i32 {
                            TOGGLE_VISIBILITY => Some(HostAction::ToggleVisibility),
                            TOGGLE_EDIT => Some(HostAction::ToggleEditMode),
                            TOGGLE_COACHING => Some(HostAction::ToggleCoaching),
                            CYCLE_PRESET => Some(HostAction::CyclePreset),
                            _ => None,
                        };
                        if let Some(action) = action {
                            let _ = actions.send(action);
                        }
                    }
                }
                for id in [
                    TOGGLE_VISIBILITY,
                    TOGGLE_EDIT,
                    TOGGLE_COACHING,
                    CYCLE_PRESET,
                ] {
                    UnregisterHotKey(std::ptr::null_mut(), id);
                }
            });
            Self {
                thread_id,
                join: Some(join),
            }
        }

        pub fn shutdown(mut self) {
            let thread_id = self.thread_id.load(Ordering::Relaxed);
            if thread_id != 0 {
                unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) };
            }
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    unsafe fn register(id: i32, value: &str) {
        let Some((modifiers, key)) = parse_hotkey(value) else {
            log::warn!("Invalid host hotkey: {value}");
            return;
        };
        if RegisterHotKey(std::ptr::null_mut(), id, modifiers, key) == 0 {
            log::warn!("Could not register host hotkey '{value}'");
        }
    }

    fn parse_hotkey(value: &str) -> Option<(u32, u32)> {
        let mut modifiers = 0;
        let mut key = None;
        for part in value
            .split('+')
            .map(|part| part.trim().to_ascii_uppercase())
        {
            match part.as_str() {
                "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL,
                "SHIFT" => modifiers |= MOD_SHIFT,
                "ALT" => modifiers |= MOD_ALT,
                value
                    if value
                        .strip_prefix('F')
                        .and_then(|number| number.parse::<u32>().ok())
                        .is_some_and(|number| (1..=12).contains(&number)) =>
                {
                    key = value[1..].parse::<u32>().ok().map(|number| 0x6f + number);
                }
                _ => return None,
            }
        }
        key.map(|key| (modifiers, key))
    }
}

#[cfg(windows)]
pub use windows_hotkeys::HostHotkeys;

#[cfg(not(windows))]
pub struct HostHotkeys;

#[cfg(not(windows))]
impl HostHotkeys {
    pub fn start(_config: HotkeyConfig, _actions: Sender<HostAction>) -> Self {
        Self
    }
    pub fn shutdown(self) {}
}
