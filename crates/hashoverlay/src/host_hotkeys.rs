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
            mpsc::{sync_channel, Sender},
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
            WindowsAndMessaging::{
                GetMessageW, PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, WM_HOTKEY, WM_QUIT,
            },
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
        pub fn start(config: HotkeyConfig, actions: Sender<HostAction>) -> std::io::Result<Self> {
            let thread_id = Arc::new(AtomicU32::new(0));
            let worker_id = thread_id.clone();
            let (ready_sender, ready_receiver) = sync_channel(1);
            let join = thread::Builder::new()
                .name("hashoverlay-global-hotkeys".to_string())
                .spawn(move || unsafe {
                    let mut message: MSG = std::mem::zeroed();
                    // A thread queue must exist before shutdown can safely post WM_QUIT.
                    PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                    worker_id.store(GetCurrentThreadId(), Ordering::Relaxed);
                    let registrations = [
                        (TOGGLE_VISIBILITY, config.toggle_overlay.as_str()),
                        (TOGGLE_EDIT, config.edit_mode.as_str()),
                        (TOGGLE_COACHING, config.toggle_coaching.as_str()),
                        (CYCLE_PRESET, config.cycle_preset.as_str()),
                    ];
                    let mut registered = Vec::new();
                    let registration_result = registrations.iter().try_for_each(|(id, binding)| {
                        register(*id, binding).map(|()| registered.push(*id))
                    });
                    if let Err(error) = registration_result {
                        for id in registered {
                            UnregisterHotKey(std::ptr::null_mut(), id);
                        }
                        let _ = ready_sender.send(Err(error));
                        return;
                    }
                    let _ = ready_sender.send(Ok(()));
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
                    for id in registered {
                        UnregisterHotKey(std::ptr::null_mut(), id);
                    }
                })?;
            match ready_receiver.recv() {
                Ok(Ok(())) => Ok(Self {
                    thread_id,
                    join: Some(join),
                }),
                Ok(Err(error)) => {
                    let _ = join.join();
                    Err(error)
                }
                Err(_) => {
                    let _ = join.join();
                    Err(std::io::Error::other(
                        "host hotkey worker exited before initialization completed",
                    ))
                }
            }
        }

        pub fn shutdown(mut self) {
            self.finish();
        }

        fn finish(&mut self) {
            let thread_id = self.thread_id.load(Ordering::Relaxed);
            if thread_id != 0 {
                unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) };
            }
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    impl Drop for HostHotkeys {
        fn drop(&mut self) {
            self.finish();
        }
    }

    unsafe fn register(id: i32, value: &str) -> std::io::Result<()> {
        let (modifiers, key) = parse_hotkey(value).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid host hotkey: {value}"),
            )
        })?;
        if RegisterHotKey(std::ptr::null_mut(), id, modifiers, key) == 0 {
            let error = std::io::Error::last_os_error();
            return Err(std::io::Error::new(
                error.kind(),
                format!("could not register host hotkey '{value}': {error}"),
            ));
        }
        Ok(())
    }

    fn parse_hotkey(value: &str) -> Option<(u32, u32)> {
        let mut modifiers = 0;
        let mut key = None;
        for part in value
            .split('+')
            .map(|part| part.trim().to_ascii_uppercase())
        {
            match part.as_str() {
                "CTRL" | "CONTROL" if modifiers & MOD_CONTROL == 0 => modifiers |= MOD_CONTROL,
                "SHIFT" if modifiers & MOD_SHIFT == 0 => modifiers |= MOD_SHIFT,
                "ALT" if modifiers & MOD_ALT == 0 => modifiers |= MOD_ALT,
                value
                    if value
                        .strip_prefix('F')
                        .and_then(|number| number.parse::<u32>().ok())
                        .is_some_and(|number| (1..=12).contains(&number))
                        && key.is_none() =>
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
    pub fn start(_config: HotkeyConfig, _actions: Sender<HostAction>) -> std::io::Result<Self> {
        Ok(Self)
    }
    pub fn shutdown(self) {}
}
