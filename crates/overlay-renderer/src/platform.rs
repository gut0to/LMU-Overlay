use std::{error::Error, fmt};

use crate::config;
use crate::config::OverlayConfig;
#[cfg(not(windows))]
use telemetry_engine::TelemetrySnapshot;

#[derive(Debug)]
pub enum OverlayError {
    UnsupportedPlatform,
    WindowCreationFailed,
    Config(config::ConfigError),
}

impl fmt::Display for OverlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                f.write_str("overlay rendering is only supported on Windows")
            }
            Self::WindowCreationFailed => {
                f.write_str("could not create the telemetry overlay window")
            }
            Self::Config(error) => write!(f, "{error}"),
        }
    }
}

impl Error for OverlayError {}

impl From<config::ConfigError> for OverlayError {
    fn from(error: config::ConfigError) -> Self {
        Self::Config(error)
    }
}

#[cfg(windows)]
mod windows_overlay {
    use std::{
        ffi::c_void,
        fs,
        mem::zeroed,
        path::PathBuf,
        ptr,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread,
        time::{Duration, Instant, SystemTime},
    };

    use telemetry_engine::{RingBuffer, TelemetrySnapshot};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
            InvalidateRect, LineTo, MoveToEx, Rectangle, ScreenToClient, SelectObject, SetBkMode,
            SetTextColor, TextOutW, HDC, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::SetProcessDpiAwarenessContext,
            Input::KeyboardAndMouse::{
                RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, PostQuitMessage,
                RegisterClassW, SetLayeredWindowAttributes, ShowWindow, TranslateMessage,
                CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWL_EXSTYLE, HTBOTTOM, HTBOTTOMRIGHT,
                HTCAPTION, HTCLIENT, HTRIGHT, HWND_TOPMOST, LWA_ALPHA, LWA_COLORKEY, MSG,
                SWP_NOACTIVATE, SW_HIDE, SW_SHOW, WM_DESTROY, WM_HOTKEY, WM_LBUTTONDOWN,
                WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WNDCLASSW, WS_EX_LAYERED,
                WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    };

    use crate::config::{WidgetLayout, WidgetStyleConfig};

    use super::{config::parse_color, OverlayConfig, OverlayError};

    const CLASS_NAME: &[u16] = &[
        'H' as u16, 'a' as u16, 's' as u16, 'h' as u16, 'O' as u16, 'v' as u16, 'e' as u16,
        'r' as u16, 'l' as u16, 'a' as u16, 'y' as u16, 0,
    ];
    const COLOR_KEY: u32 = 0x000000;
    const HOTKEY_TOGGLE_OVERLAY: i32 = 1;
    const HOTKEY_EDIT_MODE: i32 = 2;
    const HOTKEY_TOGGLE_COACHING: i32 = 3;
    const HOTKEY_CYCLE_PRESET: i32 = 4;
    const EDIT_HIT_MARGIN: i32 = 16;

    #[derive(Clone, Copy)]
    struct Area {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    impl Area {
        fn right(self) -> i32 {
            self.x + self.width
        }

        fn bottom(self) -> i32 {
            self.y + self.height
        }

        fn contains(self, x: i32, y: i32) -> bool {
            x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum WidgetId {
        Telemetry,
        Inputs,
        LapTiming,
        Timing,
        Sectors,
        MiniSectors,
        Coaching,
        Performance,
        Extra(&'static str),
    }

    const EXTRA_WIDGET_IDS: &[&str] = &[
        "speed",
        "rpm",
        "lap_history",
        "position",
        "relative",
        "standings",
        "flags",
        "fuel",
        "tyres",
        "brakes",
        "electronics",
        "energy",
        "engine",
        "damage",
        "weather",
    ];

    #[derive(Clone, Copy)]
    struct DragState {
        widget: WidgetId,
        start_mouse_x: i32,
        start_mouse_y: i32,
        start_x: i32,
        start_y: i32,
        start_width: i32,
        start_height: i32,
        resize: bool,
    }

    #[derive(Clone, Copy)]
    struct BarStyle {
        fill: u32,
        label: u32,
        reference: u32,
        line_width: i32,
    }

    #[derive(Clone)]
    struct SharedState {
        latest: Arc<Mutex<Option<TelemetrySnapshot>>>,
        history: Arc<Mutex<RingBuffer<TelemetrySnapshot>>>,
        running: Arc<AtomicBool>,
        visible: Arc<AtomicBool>,
        edit_mode: Arc<AtomicBool>,
        stats: Arc<Mutex<PerfStats>>,
        config: Arc<Mutex<OverlayConfig>>,
        config_path: Option<Arc<PathBuf>>,
        selected_widget: Arc<Mutex<Option<WidgetId>>>,
        drag: Arc<Mutex<Option<DragState>>>,
    }

    #[derive(Debug, Default)]
    struct PerfStats {
        telemetry_samples: u64,
        render_frames: u64,
        telemetry_hz: u64,
        render_fps: u64,
        acquisition_micros: u64,
        render_micros: u64,
        acquisition_ms: f64,
        render_ms: f64,
        skipped_samples: u64,
        dropped_frames: u64,
    }

    impl PerfStats {
        fn refresh(&mut self) {
            self.telemetry_hz = self.telemetry_samples;
            self.render_fps = self.render_frames;
            self.acquisition_ms = if self.telemetry_samples == 0 {
                0.0
            } else {
                self.acquisition_micros as f64 / self.telemetry_samples as f64 / 1_000.0
            };
            self.render_ms = if self.render_frames == 0 {
                0.0
            } else {
                self.render_micros as f64 / self.render_frames as f64 / 1_000.0
            };
            self.telemetry_samples = 0;
            self.render_frames = 0;
            self.acquisition_micros = 0;
            self.render_micros = 0;
        }
    }

    pub struct TelemetryOverlay {
        state: SharedState,
    }

    impl TelemetryOverlay {
        pub fn new() -> Result<Self, OverlayError> {
            Self::with_config(OverlayConfig::default())
        }

        pub fn with_config(config: OverlayConfig) -> Result<Self, OverlayError> {
            Self::with_config_source(config, None)
        }

        pub fn with_config_path(
            config: OverlayConfig,
            config_path: impl Into<PathBuf>,
        ) -> Result<Self, OverlayError> {
            Self::with_config_source(config, Some(config_path.into()))
        }

        fn with_config_source(
            config: OverlayConfig,
            config_path: Option<PathBuf>,
        ) -> Result<Self, OverlayError> {
            let history_samples = config.window.history_samples;
            Ok(Self {
                state: SharedState {
                    latest: Arc::new(Mutex::new(None)),
                    history: Arc::new(Mutex::new(RingBuffer::new(history_samples))),
                    running: Arc::new(AtomicBool::new(true)),
                    visible: Arc::new(AtomicBool::new(true)),
                    edit_mode: Arc::new(AtomicBool::new(false)),
                    stats: Arc::new(Mutex::new(PerfStats::default())),
                    config: Arc::new(Mutex::new(config)),
                    config_path: config_path.map(Arc::new),
                    selected_widget: Arc::new(Mutex::new(None)),
                    drag: Arc::new(Mutex::new(None)),
                },
            })
        }

        pub fn run<F>(self, mut next_snapshot: F) -> Result<(), OverlayError>
        where
            F: FnMut() -> Option<TelemetrySnapshot> + Send + 'static,
        {
            let hwnd = create_window(self.state.clone())?;
            let telemetry_state = self.state.clone();
            let telemetry_worker = thread::spawn(move || {
                let mut next_sample = Instant::now();
                while telemetry_state.running.load(Ordering::Relaxed) {
                    let sample_interval = telemetry_state
                        .config
                        .lock()
                        .ok()
                        .map(|config| Duration::from_millis(config.window.sample_ms))
                        .unwrap_or_else(|| Duration::from_millis(10));

                    let now = Instant::now();
                    if now < next_sample {
                        thread::sleep(next_sample - now);
                        continue;
                    }

                    let acquisition_started = Instant::now();
                    if let Some(snapshot) = next_snapshot() {
                        if let Ok(mut latest) = telemetry_state.latest.lock() {
                            *latest = Some(snapshot);
                        }
                        if let Ok(mut history) = telemetry_state.history.lock() {
                            history.push(snapshot);
                        }
                        if let Ok(mut stats) = telemetry_state.stats.lock() {
                            stats.telemetry_samples += 1;
                            stats.acquisition_micros +=
                                acquisition_started.elapsed().as_micros() as u64;
                        }
                    }
                    next_sample += sample_interval;
                    if next_sample < Instant::now() {
                        if let Ok(mut stats) = telemetry_state.stats.lock() {
                            stats.skipped_samples += 1;
                        }
                        next_sample = Instant::now() + sample_interval;
                    }
                }
            });

            let repaint_running = self.state.running.clone();
            let repaint_config = self.state.config.clone();
            let repaint_visible = self.state.visible.clone();
            let repaint_edit_mode = self.state.edit_mode.clone();
            let repaint_hwnd = hwnd as isize;

            let repaint_stats = self.state.stats.clone();
            let repaint_worker = thread::spawn(move || {
                let hwnd = repaint_hwnd as HWND;
                let mut next_frame = Instant::now();
                while repaint_running.load(Ordering::Relaxed) {
                    if !repaint_visible.load(Ordering::Relaxed)
                        && !repaint_edit_mode.load(Ordering::Relaxed)
                    {
                        thread::sleep(Duration::from_millis(200));
                        next_frame = Instant::now();
                        continue;
                    }
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                    let refresh_hz = repaint_config
                        .lock()
                        .ok()
                        .map(|config| config.window.refresh_hz)
                        .unwrap_or(60);
                    let frame_duration =
                        Duration::from_micros((1_000_000 / refresh_hz.max(1)).max(1));
                    next_frame += frame_duration;
                    let now = Instant::now();
                    if now < next_frame {
                        thread::sleep(next_frame - now);
                    } else {
                        if let Ok(mut stats) = repaint_stats.lock() {
                            stats.dropped_frames += 1;
                        }
                        next_frame = now + frame_duration;
                    }
                }
            });

            let mut last_stats = Instant::now();
            let mut last_config_check = Instant::now();
            let mut config_mtime = self
                .state
                .config_path
                .as_ref()
                .and_then(|path| modified_time(path.as_ref()));
            let mut message: MSG = unsafe { zeroed() };

            'message_loop: loop {
                unsafe {
                    while windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(
                        &mut message,
                        ptr::null_mut(),
                        0,
                        0,
                        windows_sys::Win32::UI::WindowsAndMessaging::PM_REMOVE,
                    ) != 0
                    {
                        if message.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT {
                            self.state.running.store(false, Ordering::Relaxed);
                            break 'message_loop;
                        }
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }

                if last_stats.elapsed() >= Duration::from_secs(1) {
                    if let Ok(mut stats) = self.state.stats.lock() {
                        stats.refresh();
                    }
                    last_stats = Instant::now();
                }

                if last_config_check.elapsed() >= Duration::from_millis(500) {
                    reload_runtime_config(hwnd, &self.state, &mut config_mtime);
                    last_config_check = Instant::now();
                }

                thread::sleep(Duration::from_millis(1));
            }

            self.state.running.store(false, Ordering::Relaxed);
            let _ = telemetry_worker.join();
            let _ = repaint_worker.join();
            Ok(())
        }
    }

    fn modified_time(path: &PathBuf) -> Option<SystemTime> {
        fs::metadata(path).ok()?.modified().ok()
    }

    fn reload_runtime_config(hwnd: HWND, state: &SharedState, last_mtime: &mut Option<SystemTime>) {
        if state.edit_mode.load(Ordering::Relaxed) {
            return;
        }
        let Some(path) = &state.config_path else {
            return;
        };
        let Some(current_mtime) = modified_time(path.as_ref()) else {
            return;
        };
        if last_mtime.is_some_and(|mtime| mtime >= current_mtime) {
            return;
        }

        match OverlayConfig::load(path.as_ref()) {
            Ok(config) => {
                apply_window_config(hwnd, &config);
                unsafe {
                    reload_hotkeys(hwnd, &config);
                }
                resize_history_if_needed(state, config.window.history_samples);
                if let Ok(mut current) = state.config.lock() {
                    *current = config;
                }
                *last_mtime = Some(current_mtime);
                unsafe {
                    InvalidateRect(hwnd, ptr::null(), 0);
                }
            }
            Err(error) => log::warn!("Could not hot reload overlay config: {error}"),
        }
    }

    fn resize_history_if_needed(state: &SharedState, history_samples: usize) {
        if let Ok(mut history) = state.history.lock() {
            if history.capacity() != history_samples {
                *history = RingBuffer::new(history_samples);
            }
        }
    }

    fn apply_window_config(hwnd: HWND, config: &OverlayConfig) {
        unsafe {
            SetLayeredWindowAttributes(
                hwnd,
                COLOR_KEY,
                config.style.opacity,
                LWA_COLORKEY | LWA_ALPHA,
            );
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                config.window.x,
                config.window.y,
                config.window.width,
                config.window.height,
                SWP_NOACTIVATE,
            );
        }
    }

    unsafe fn reload_hotkeys(hwnd: HWND, config: &OverlayConfig) {
        UnregisterHotKey(hwnd, HOTKEY_TOGGLE_OVERLAY);
        UnregisterHotKey(hwnd, HOTKEY_EDIT_MODE);
        UnregisterHotKey(hwnd, HOTKEY_TOGGLE_COACHING);
        UnregisterHotKey(hwnd, HOTKEY_CYCLE_PRESET);
        register_runtime_hotkeys(hwnd, config);
    }

    unsafe fn register_runtime_hotkeys(hwnd: HWND, config: &OverlayConfig) {
        register_hotkey(
            hwnd,
            HOTKEY_TOGGLE_OVERLAY,
            "toggle overlay",
            &config.hotkeys.toggle_overlay,
        );
        register_hotkey(
            hwnd,
            HOTKEY_EDIT_MODE,
            "edit mode",
            &config.hotkeys.edit_mode,
        );
        register_hotkey(
            hwnd,
            HOTKEY_TOGGLE_COACHING,
            "toggle coaching",
            &config.hotkeys.toggle_coaching,
        );
        register_hotkey(
            hwnd,
            HOTKEY_CYCLE_PRESET,
            "cycle preset",
            &config.hotkeys.cycle_preset,
        );
    }

    unsafe fn register_hotkey(hwnd: HWND, id: i32, label: &str, value: &str) {
        let Some((modifiers, key)) = hotkey(value) else {
            log::warn!("Invalid {label} hotkey: {value}");
            return;
        };
        if RegisterHotKey(hwnd, id, modifiers, key) == 0 {
            log::warn!("Could not register {label} hotkey '{value}'; it may already be in use.");
        }
    }

    fn create_window(state: SharedState) -> Result<HWND, OverlayError> {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(
                windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );

            let instance = GetModuleHandleW(ptr::null());
            let window_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: CLASS_NAME.as_ptr(),
                ..zeroed()
            };
            RegisterClassW(&window_class);

            let config = state
                .config
                .lock()
                .ok()
                .map(|config| config.clone())
                .unwrap_or_default();
            let x = config.window.x;
            let y = config.window.y;
            let width = config.window.width;
            let height = config.window.height;
            let opacity = config.style.opacity;
            let state_ptr = Box::into_raw(Box::new(state));
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                CLASS_NAME.as_ptr(),
                CLASS_NAME.as_ptr(),
                WS_POPUP,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                state_ptr.cast::<c_void>(),
            );

            if hwnd.is_null() {
                drop(Box::from_raw(state_ptr));
                return Err(OverlayError::WindowCreationFailed);
            }

            SetLayeredWindowAttributes(hwnd, COLOR_KEY, opacity, LWA_COLORKEY | LWA_ALPHA);
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            );
            ShowWindow(hwnd, SW_SHOW);
            register_runtime_hotkeys(hwnd, &config);

            Ok(hwnd)
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            windows_sys::Win32::UI::WindowsAndMessaging::WM_NCCREATE => {
                let create =
                    lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
                let state = (*create).lpCreateParams;
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    state as isize,
                );
                1
            }
            WM_PAINT => {
                paint(hwnd);
                0
            }
            WM_HOTKEY => {
                handle_hotkey(hwnd, wparam as i32);
                0
            }
            WM_LBUTTONDOWN => {
                handle_mouse_down(hwnd, lparam);
                0
            }
            WM_MOUSEMOVE => {
                handle_mouse_move(hwnd, lparam);
                0
            }
            WM_LBUTTONUP => {
                handle_mouse_up(hwnd);
                0
            }
            WM_NCHITTEST => edit_mode_hit_test(hwnd, lparam)
                .unwrap_or_else(|| DefWindowProcW(hwnd, message, wparam, lparam)),
            WM_DESTROY => {
                UnregisterHotKey(hwnd, HOTKEY_TOGGLE_OVERLAY);
                UnregisterHotKey(hwnd, HOTKEY_EDIT_MODE);
                UnregisterHotKey(hwnd, HOTKEY_TOGGLE_COACHING);
                UnregisterHotKey(hwnd, HOTKEY_CYCLE_PRESET);
                let state_ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                ) as *mut SharedState;
                if !state_ptr.is_null() {
                    (*state_ptr).running.store(false, Ordering::Relaxed);
                    drop(Box::from_raw(state_ptr));
                }
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn handle_hotkey(hwnd: HWND, id: i32) {
        let Some(state) = shared_state(hwnd) else {
            return;
        };

        match id {
            HOTKEY_TOGGLE_OVERLAY => {
                let visible = !state.visible.load(Ordering::Relaxed);
                state.visible.store(visible, Ordering::Relaxed);
                ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
            }
            HOTKEY_EDIT_MODE => {
                let edit_mode = !state.edit_mode.load(Ordering::Relaxed);
                state.edit_mode.store(edit_mode, Ordering::Relaxed);
                state.visible.store(true, Ordering::Relaxed);
                ShowWindow(hwnd, SW_SHOW);
                set_click_through(hwnd, !edit_mode);
            }
            HOTKEY_TOGGLE_COACHING => {
                if let Ok(mut config) = state.config.lock() {
                    config.widgets.coaching = !config.widgets.coaching;
                    config.coaching.mode = if config.widgets.coaching {
                        "practice".to_string()
                    } else {
                        "off".to_string()
                    };
                }
                InvalidateRect(hwnd, ptr::null(), 0);
                save_runtime_config(state);
            }
            HOTKEY_CYCLE_PRESET => {
                if let Ok(mut config) = state.config.lock() {
                    cycle_runtime_preset(&mut config);
                }
                InvalidateRect(hwnd, ptr::null(), 0);
                save_runtime_config(state);
            }
            _ => {}
        }
    }

    unsafe fn handle_mouse_down(hwnd: HWND, lparam: LPARAM) {
        let Some(state) = shared_state(hwnd) else {
            return;
        };
        if !state.edit_mode.load(Ordering::Relaxed) {
            return;
        }

        let mouse_x = loword_signed(lparam);
        let mouse_y = hiword_signed(lparam);
        let Some((widget, area)) = state
            .config
            .lock()
            .ok()
            .and_then(|config| hit_widget(&config, mouse_x, mouse_y))
        else {
            if let Ok(mut selected) = state.selected_widget.lock() {
                *selected = None;
            }
            return;
        };

        if let Ok(mut selected) = state.selected_widget.lock() {
            *selected = Some(widget);
        }
        if let Ok(mut drag) = state.drag.lock() {
            *drag = Some(DragState {
                widget,
                start_mouse_x: mouse_x,
                start_mouse_y: mouse_y,
                start_x: area.x,
                start_y: area.y,
                start_width: area.width,
                start_height: area.height,
                resize: area.right() - mouse_x <= EDIT_HIT_MARGIN
                    && area.bottom() - mouse_y <= EDIT_HIT_MARGIN,
            });
        }
        InvalidateRect(hwnd, ptr::null(), 0);
    }

    unsafe fn handle_mouse_move(hwnd: HWND, lparam: LPARAM) {
        let Some(state) = shared_state(hwnd) else {
            return;
        };
        if !state.edit_mode.load(Ordering::Relaxed) {
            return;
        }

        let Some(drag) = state.drag.lock().ok().and_then(|value| *value) else {
            return;
        };

        let mouse_x = loword_signed(lparam);
        let mouse_y = hiword_signed(lparam);
        let delta_x = mouse_x - drag.start_mouse_x;
        let delta_y = mouse_y - drag.start_mouse_y;

        if let Ok(mut config) = state.config.lock() {
            let window_width = config.window.width;
            let window_height = config.window.height;
            let snap_to_edges = config.layout.snap_to_edges;
            let snap_to_grid = config.layout.snap_to_grid;
            let snap_to_widgets = config.layout.snap_to_widgets;
            let grid_size = config.layout.grid_size;
            let snap_distance = config.layout.snap_distance;
            let layout = widget_layout_mut(&mut config, drag.widget);
            if drag.resize {
                layout.width = (drag.start_width + delta_x).clamp(48, window_width.max(48));
                layout.height = (drag.start_height + delta_y).clamp(20, window_height.max(20));
            } else {
                layout.x = drag.start_x + delta_x;
                layout.y = drag.start_y + delta_y;
                if snap_to_edges {
                    snap_widget_to_edges(layout, window_width, window_height, snap_distance);
                }
            }
            if snap_to_grid {
                snap_widget_to_grid(layout, grid_size);
            }
            config.normalize();
            if snap_to_widgets {
                snap_widget_to_widgets(&mut config, drag.widget);
            }
        }

        InvalidateRect(hwnd, ptr::null(), 0);
    }

    unsafe fn handle_mouse_up(hwnd: HWND) {
        let Some(state) = shared_state(hwnd) else {
            return;
        };
        let had_drag = state
            .drag
            .lock()
            .ok()
            .and_then(|mut value| value.take())
            .is_some();
        if had_drag {
            save_runtime_config(state);
            InvalidateRect(hwnd, ptr::null(), 0);
        }
    }

    fn save_runtime_config(state: &SharedState) {
        let Some(path) = &state.config_path else {
            return;
        };
        if let Ok(config) = state.config.lock() {
            let mut config = config.clone();
            if let Err(error) = config.save(path.as_ref()) {
                log::warn!("Could not save overlay layout: {error}");
            }
        }
    }

    fn cycle_runtime_preset(config: &mut OverlayConfig) {
        let next = match (
            config.performance.mode.as_str(),
            config.timing.reference_mode.as_str(),
        ) {
            ("normal", "last_lap") => config.presets.qualifying.clone(),
            ("high_refresh", "personal_best") => config.presets.race.clone(),
            ("eco", "session_best") => config.presets.endurance.clone(),
            ("eco", _) => config.presets.minimal.clone(),
            _ => config.presets.practice.clone(),
        };
        config.performance.mode = next.performance_mode;
        config.timing.reference_mode = next.reference_mode;
        config.timing.mini_sectors = next.mini_sectors;
        config.style = next.style;
        config.units = next.units;
        config.coaching = next.coaching_config;
        config.layout = next.layout;
        config.extra_widgets = next.extra_widgets;
        config.widgets.title = next.title;
        config.widgets.speed_gear_rpm = next.speed_gear_rpm;
        config.widgets.pedals = next.pedals;
        config.widgets.steering = next.steering;
        config.widgets.lap_info = next.lap_info;
        config.widgets.lap_timing = next.lap_timing;
        config.widgets.sectors = next.sectors;
        config.widgets.mini_sector_widget = next.mini_sector_widget;
        config.widgets.input_history = next.input_history;
        config.widgets.delta_timing = next.delta_timing;
        config.widgets.ghost_inputs = next.ghost_inputs;
        config.widgets.coaching = next.coaching;
        config.widgets.performance_monitor = next.performance_monitor;
        config.normalize();
    }

    unsafe fn shared_state(hwnd: HWND) -> Option<&'static SharedState> {
        let state_ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        ) as *mut SharedState;

        (!state_ptr.is_null()).then_some(&*state_ptr)
    }

    unsafe fn set_click_through(hwnd: HWND, enabled: bool) {
        let mut ex_style =
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if enabled {
            ex_style |= WS_EX_TRANSPARENT as isize;
        } else {
            ex_style &= !(WS_EX_TRANSPARENT as isize);
        }
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
    }

    unsafe fn paint(hwnd: HWND) {
        let render_started = Instant::now();
        let mut paint: PAINTSTRUCT = zeroed();
        let hdc = BeginPaint(hwnd, &mut paint);
        let mut rect: RECT = zeroed();
        GetClientRect(hwnd, &mut rect);

        let black = CreateSolidBrush(COLOR_KEY);
        FillRect(hdc, &rect, black);
        DeleteObject(black);

        let state_ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        ) as *mut SharedState;

        if state_ptr.is_null() {
            EndPaint(hwnd, &paint);
            return;
        }

        let state = &*state_ptr;
        let config = state
            .config
            .lock()
            .ok()
            .map(|config| config.clone())
            .unwrap_or_default();
        if let Ok(mut stats) = state.stats.lock() {
            stats.render_frames += 1;
        }
        let latest = state.latest.lock().ok().and_then(|value| *value);

        draw_panel(hdc, &config);

        if let Some(snapshot) = latest {
            draw_snapshot(hdc, snapshot, &config);
            if config.widgets.input_history {
                if let Ok(history) = state.history.lock() {
                    draw_history(hdc, &history, &config);
                }
            }
        } else {
            draw_text(
                hdc,
                22,
                24,
                colors(&config).primary_text,
                "Waiting for LMU telemetry...",
            );
        }

        if config.widgets.performance_monitor {
            draw_performance_monitor(hdc, state, &config);
        }

        if state.edit_mode.load(Ordering::Relaxed) {
            let selected = state.selected_widget.lock().ok().and_then(|value| *value);
            draw_edit_handles(hdc, &config, selected);
        }

        if let Ok(mut stats) = state.stats.lock() {
            stats.render_micros += render_started.elapsed().as_micros() as u64;
        }

        EndPaint(hwnd, &paint);
    }

    unsafe fn draw_panel(hdc: HDC, config: &OverlayConfig) {
        let colors = colors(config);
        let bg = CreateSolidBrush(colors.background);
        let border = CreatePen(PS_SOLID, config.style.line_thickness, colors.border);
        let old_brush = SelectObject(hdc, bg);
        let old_pen = SelectObject(hdc, border);
        Rectangle(hdc, 0, 0, config.window.width, config.window.height);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(border);
        DeleteObject(bg);
    }

    unsafe fn draw_widget_panel(hdc: HDC, area: Area, config: &OverlayConfig) {
        let colors = colors(config);
        let border = CreatePen(PS_SOLID, config.style.line_thickness.max(1), colors.border);
        let old_pen = SelectObject(hdc, border);
        Rectangle(
            hdc,
            area.x,
            area.y,
            area.x + area.width,
            area.y + area.height,
        );
        SelectObject(hdc, old_pen);
        DeleteObject(border);
    }

    unsafe fn draw_snapshot(hdc: HDC, snapshot: TelemetrySnapshot, config: &OverlayConfig) {
        for (widget, area) in widget_areas(config) {
            match widget {
                WidgetId::Telemetry if config.widgets.title || config.widgets.speed_gear_rpm => {
                    draw_widget_panel(hdc, area, config);
                    draw_telemetry_widget(hdc, snapshot, config, area);
                }
                WidgetId::LapTiming if config.widgets.lap_info || config.widgets.lap_timing => {
                    draw_widget_panel(hdc, area, config);
                    draw_lap_timing_widget(hdc, snapshot, config, area);
                }
                WidgetId::Inputs
                    if config.widgets.pedals
                        || config.widgets.steering
                        || config.widgets.input_history =>
                {
                    draw_widget_panel(hdc, area, config);
                    draw_input_widget(hdc, snapshot, config, area);
                }
                WidgetId::Timing if config.widgets.delta_timing => {
                    draw_widget_panel(hdc, area, config);
                    draw_delta_widget(hdc, snapshot, config, area);
                }
                WidgetId::Sectors if config.widgets.sectors => {
                    draw_widget_panel(hdc, area, config);
                    draw_sectors_widget(hdc, snapshot, config, area);
                }
                WidgetId::MiniSectors if config.widgets.mini_sector_widget => {
                    draw_widget_panel(hdc, area, config);
                    draw_mini_sectors_widget(hdc, snapshot, config, area);
                }
                WidgetId::Coaching if config.widgets.coaching && config.coaching.mode != "off" => {
                    draw_widget_panel(hdc, area, config);
                    draw_coaching_widget(hdc, snapshot, config, area);
                }
                WidgetId::Extra(id) => {
                    draw_extra_widget(hdc, snapshot, config, area, id);
                }
                _ => {}
            }
        }
    }

    unsafe fn draw_telemetry_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        if config.widgets.title {
            draw_text(
                hdc,
                area.x + scale_px(config, 10),
                area.y + scale_px(config, 6),
                colors.primary_text,
                "HashOverlay LMU",
            );
        }
        if config.widgets.speed_gear_rpm {
            draw_text(
                hdc,
                area.x + scale_px(config, 10),
                area.y + scale_px(config, 26),
                colors.primary_text,
                &format!(
                    "{:.0} {}   gear {}   {:.0} rpm",
                    display_speed(snapshot.speed_kph, config),
                    speed_unit_label(config),
                    snapshot.gear,
                    snapshot.rpm
                ),
            );
        }
    }

    unsafe fn draw_extra_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
        id: &str,
    ) {
        let widget_style = config.extra_widgets.get(id).map(|widget| &widget.style);
        draw_extra_widget_panel(hdc, area, config, widget_style);
        let padding = widget_style.map_or(scale_px(config, 8), |style| style.padding);
        let value = match id {
            "speed" => format!(
                "{:.0} {}",
                display_speed(snapshot.speed_kph, config),
                speed_unit_label(config)
            ),
            "rpm" => snapshot.vehicle.max_rpm.map_or_else(
                || format!("{:.0} RPM", snapshot.rpm),
                |limit| {
                    format!(
                        "{:.0} RPM  {:.0}%",
                        snapshot.rpm,
                        snapshot.rpm / limit * 100.0
                    )
                },
            ),
            "position" => match (snapshot.session.position, snapshot.session.total_vehicles) {
                (Some(position), Some(total)) => format!("P{position}/{total}"),
                (Some(position), None) => format!("P{position}"),
                _ => "POSITION --".to_string(),
            },
            "flags" => snapshot
                .session
                .flag
                .map_or_else(|| "FLAG --".to_string(), |flag| format!("FLAG {flag}")),
            "fuel" => match (
                snapshot.vehicle.fuel_liters,
                snapshot.vehicle.fuel_capacity_liters,
            ) {
                (Some(fuel), Some(capacity)) if capacity > 0.0 => {
                    format!("FUEL {fuel:.1} L  {:.0}%", fuel / capacity * 100.0)
                }
                (Some(fuel), _) => format!("FUEL {fuel:.1} L"),
                _ => "FUEL --".to_string(),
            },
            "tyres" => wheel_summary(
                "TYRES",
                [
                    snapshot.wheels.front_left.pressure_kpa,
                    snapshot.wheels.front_right.pressure_kpa,
                    snapshot.wheels.rear_left.pressure_kpa,
                    snapshot.wheels.rear_right.pressure_kpa,
                ],
                "kPa",
            ),
            "brakes" => wheel_summary(
                "BRAKES",
                [
                    snapshot.wheels.front_left.brake_temp_c,
                    snapshot.wheels.front_right.brake_temp_c,
                    snapshot.wheels.rear_left.brake_temp_c,
                    snapshot.wheels.rear_right.brake_temp_c,
                ],
                "C",
            ),
            "electronics" => match (snapshot.vehicle.tc_setting, snapshot.vehicle.abs_setting) {
                (Some(tc), Some(abs)) => format!("TC {tc}  ABS {abs}"),
                _ => "TC / ABS --".to_string(),
            },
            "energy" => snapshot.vehicle.battery_charge_percent.map_or_else(
                || "ENERGY --".to_string(),
                |charge| format!("ENERGY {charge:.0}%"),
            ),
            "engine" => match (
                snapshot.vehicle.engine_water_temp_c,
                snapshot.vehicle.engine_oil_temp_c,
            ) {
                (Some(water), Some(oil)) => format!("W {water:.0}C  O {oil:.0}C"),
                _ => "ENGINE --".to_string(),
            },
            "weather" => match (
                snapshot.session.ambient_temp_c,
                snapshot.session.track_temp_c,
            ) {
                (Some(ambient), Some(track)) => format!("AIR {ambient:.0}C  TRACK {track:.0}C"),
                _ => "WEATHER --".to_string(),
            },
            "damage" => {
                if [
                    snapshot.wheels.front_left,
                    snapshot.wheels.front_right,
                    snapshot.wheels.rear_left,
                    snapshot.wheels.rear_right,
                ]
                .into_iter()
                .any(|wheel| wheel.flat == Some(true) || wheel.detached == Some(true))
                {
                    "DAMAGE WARNING".to_string()
                } else {
                    "DAMAGE --".to_string()
                }
            }
            "relative" | "standings" | "lap_history" => "WAITING FOR OFFICIAL SCORING".to_string(),
            _ => "--".to_string(),
        };
        let title_height = if widget_style.is_some_and(|style| style.show_title) {
            let title = widget_style
                .and_then(|style| {
                    (!style.title_text.trim().is_empty()).then_some(style.title_text.as_str())
                })
                .unwrap_or(id);
            draw_text(
                hdc,
                area.x + padding,
                area.y + padding,
                widget_secondary_color(config, widget_style),
                title,
            );
            scale_px(config, 18)
        } else {
            0
        };
        draw_text(
            hdc,
            area.x + padding,
            area.y + padding + title_height,
            widget_primary_color(config, widget_style),
            &value,
        );
    }

    unsafe fn draw_extra_widget_panel(
        hdc: HDC,
        area: Area,
        config: &OverlayConfig,
        widget_style: Option<&WidgetStyleConfig>,
    ) {
        let theme_colors = colors(config);
        let inherit_theme = widget_style.map_or(true, |style| style.inherit_theme);
        let show_background = widget_style.map_or(true, |style| style.show_background);
        let show_border = widget_style.map_or(true, |style| style.show_border);
        let background = if inherit_theme {
            theme_colors.background
        } else {
            parse_color(
                &widget_style.expect("style checked").background_color,
                theme_colors.background,
            )
        };
        let border = if inherit_theme {
            theme_colors.border
        } else {
            parse_color(
                &widget_style.expect("style checked").border_color,
                theme_colors.border,
            )
        };
        let rect = RECT {
            left: area.x,
            top: area.y,
            right: area.right(),
            bottom: area.bottom(),
        };
        if show_background {
            let brush = CreateSolidBrush(background);
            FillRect(hdc, &rect, brush);
            DeleteObject(brush);
        }
        if show_border {
            let width = widget_style
                .map_or(config.style.line_thickness, |style| style.border_width)
                .max(1);
            let pen = CreatePen(PS_SOLID, width, border);
            let old_pen = SelectObject(hdc, pen);
            Rectangle(hdc, area.x, area.y, area.right(), area.bottom());
            SelectObject(hdc, old_pen);
            DeleteObject(pen);
        }
    }

    fn widget_primary_color(
        config: &OverlayConfig,
        widget_style: Option<&WidgetStyleConfig>,
    ) -> u32 {
        let theme_colors = colors(config);
        widget_style
            .filter(|style| !style.inherit_theme)
            .map_or(theme_colors.primary_text, |style| {
                parse_color(&style.primary_color, theme_colors.primary_text)
            })
    }

    fn widget_secondary_color(
        config: &OverlayConfig,
        widget_style: Option<&WidgetStyleConfig>,
    ) -> u32 {
        let theme_colors = colors(config);
        widget_style
            .filter(|style| !style.inherit_theme)
            .map_or(theme_colors.secondary_text, |style| {
                parse_color(&style.secondary_color, theme_colors.secondary_text)
            })
    }

    fn wheel_summary(label: &str, values: [Option<f64>; 4], unit: &str) -> String {
        if values.iter().any(Option::is_none) {
            return format!("{label} --");
        }
        format!(
            "{label} {:.0} {:.0} / {:.0} {:.0} {unit}",
            values[0].unwrap_or_default(),
            values[1].unwrap_or_default(),
            values[2].unwrap_or_default(),
            values[3].unwrap_or_default(),
        )
    }

    unsafe fn draw_lap_timing_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        if config.widgets.lap_info {
            let lap_x = area.x + scale_px(config, 10);
            if let Some(progress) = snapshot.lap_progress {
                draw_text(
                    hdc,
                    lap_x,
                    area.y + scale_px(config, 8),
                    colors.secondary_text,
                    &format!("lap {:.1}%", progress * 100.0),
                );
            }
            draw_text(
                hdc,
                lap_x,
                area.y + scale_px(config, 30),
                colors.secondary_text,
                &format!("lap {} sector {}", snapshot.lap_number, snapshot.sector),
            );
        }
    }

    unsafe fn draw_input_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        if config.widgets.pedals {
            draw_bar(
                hdc,
                Area {
                    x: area.x + scale_px(config, 8),
                    y: area.y + scale_px(config, 8),
                    width: scale_size(config, 34),
                    height: (area.height - scale_size(config, 22)).max(scale_size(config, 42)),
                },
                snapshot.throttle,
                "THR",
                BarStyle {
                    fill: colors.throttle,
                    label: colors.secondary_text,
                    reference: colors.reference,
                    line_width: config.style.line_thickness,
                },
                config
                    .widgets
                    .ghost_inputs
                    .then_some(snapshot.reference_throttle)
                    .flatten(),
            );
            draw_bar(
                hdc,
                Area {
                    x: area.x + scale_px(config, 54),
                    y: area.y + scale_px(config, 8),
                    width: scale_size(config, 34),
                    height: (area.height - scale_size(config, 22)).max(scale_size(config, 42)),
                },
                snapshot.brake,
                "BRK",
                BarStyle {
                    fill: colors.brake,
                    label: colors.secondary_text,
                    reference: colors.reference,
                    line_width: config.style.line_thickness,
                },
                config
                    .widgets
                    .ghost_inputs
                    .then_some(snapshot.reference_brake)
                    .flatten(),
            );
            draw_bar(
                hdc,
                Area {
                    x: area.x + scale_px(config, 100),
                    y: area.y + scale_px(config, 8),
                    width: scale_size(config, 34),
                    height: (area.height - scale_size(config, 22)).max(scale_size(config, 42)),
                },
                snapshot.clutch,
                "CLT",
                BarStyle {
                    fill: colors.clutch,
                    label: colors.secondary_text,
                    reference: colors.reference,
                    line_width: config.style.line_thickness,
                },
                None,
            );
        }
        if config.widgets.steering {
            draw_center_bar(
                hdc,
                Area {
                    x: area.x + scale_px(config, 150),
                    y: area.y + scale_px(config, 24),
                    width: (area.width - scale_size(config, 160)).max(scale_size(config, 60)),
                    height: scale_size(config, 18),
                },
                snapshot.steering,
                colors.steering,
                colors.secondary_text,
            );
        }
    }

    unsafe fn draw_bar(
        hdc: HDC,
        area: Area,
        value: f64,
        label: &str,
        style: BarStyle,
        reference_value: Option<f64>,
    ) {
        let clamped = value.clamp(0.0, 1.0);
        let filled = (area.height as f64 * clamped).round() as i32;
        let outline = CreatePen(PS_SOLID, style.line_width.max(1), 0x00888888);
        let old_pen = SelectObject(hdc, outline);
        Rectangle(
            hdc,
            area.x,
            area.y,
            area.x + area.width,
            area.y + area.height,
        );
        SelectObject(hdc, old_pen);
        DeleteObject(outline);

        let brush = CreateSolidBrush(style.fill);
        let fill_rect = RECT {
            left: area.x + 2,
            top: area.y + area.height - filled + 2,
            right: area.x + area.width - 2,
            bottom: area.y + area.height - 2,
        };
        FillRect(hdc, &fill_rect, brush);
        DeleteObject(brush);
        draw_text(
            hdc,
            area.x - 1,
            area.y + area.height + 8,
            style.label,
            label,
        );

        if let Some(reference_value) = reference_value {
            let reference_y = area.y + area.height
                - (reference_value.clamp(0.0, 1.0) * area.height as f64).round() as i32;
            let reference_pen = CreatePen(PS_SOLID, style.line_width.max(1), style.reference);
            let old_pen = SelectObject(hdc, reference_pen);
            MoveToEx(hdc, area.x, reference_y, ptr::null_mut());
            LineTo(hdc, area.x + area.width, reference_y);
            SelectObject(hdc, old_pen);
            DeleteObject(reference_pen);
        }
    }

    unsafe fn draw_center_bar(hdc: HDC, area: Area, value: f64, color: u32, label_color: u32) {
        let center = area.x + area.width / 2;
        let end = center + (value.clamp(-1.0, 1.0) * (area.width / 2) as f64).round() as i32;
        let pen = CreatePen(PS_SOLID, area.height, color);
        let old_pen = SelectObject(hdc, pen);
        MoveToEx(hdc, center, area.y, ptr::null_mut());
        LineTo(hdc, end, area.y);
        SelectObject(hdc, old_pen);
        DeleteObject(pen);
        draw_text(hdc, area.x, area.y + 20, label_color, "STEERING");
    }

    unsafe fn draw_history(
        hdc: HDC,
        history: &RingBuffer<TelemetrySnapshot>,
        config: &OverlayConfig,
    ) {
        let colors = colors(config);
        let input_area = area_from_layout(&config.layout.inputs);
        let origin_x = input_area.x + scale_px(config, 150);
        let origin_y = input_area.y + scale_px(config, 52);
        let width = (input_area.width - scale_size(config, 160)).max(scale_size(config, 60));
        let height = scale_size(config, 34);
        draw_series(
            hdc,
            history,
            Area {
                x: origin_x,
                y: origin_y,
                width,
                height,
            },
            colors.throttle,
            config.style.line_thickness,
            |s| s.throttle,
        );
        draw_series(
            hdc,
            history,
            Area {
                x: origin_x,
                y: origin_y + scale_size(config, 42),
                width,
                height,
            },
            colors.brake,
            config.style.line_thickness,
            |s| s.brake,
        );
    }

    unsafe fn draw_delta_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        let x = area.x + scale_px(config, 10);
        let y = area.y + scale_px(config, 8);
        if let Some(delta) = snapshot.delta_seconds {
            let color = if delta <= 0.0 {
                colors.delta_gain
            } else if delta >= 0.01 {
                colors.delta_loss
            } else {
                colors.delta_neutral
            };
            draw_text(hdc, x, y, color, &format!("Delta {}", signed_time(delta)));
        } else {
            draw_text(hdc, x, y, colors.secondary_text, "Delta --");
        }

        if let Some(predicted) = snapshot.predicted_lap_seconds {
            draw_text(
                hdc,
                x,
                y + scale_size(config, 20),
                colors.secondary_text,
                &format!("Pred {}", lap_time(predicted)),
            );
        }

        if let Some(best) = snapshot.personal_best_seconds {
            draw_text(
                hdc,
                x + (area.width / 2).max(scale_size(config, 100)),
                y,
                colors.secondary_text,
                &format!("PB {}", lap_time(best)),
            );
        }

        if let Some(best) = snapshot.session_best_seconds {
            draw_text(
                hdc,
                x + (area.width / 2).max(scale_size(config, 100)),
                y + scale_size(config, 20),
                colors.secondary_text,
                &format!("SB {}", lap_time(best)),
            );
        }
    }

    unsafe fn draw_sectors_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        draw_text(
            hdc,
            area.x + scale_px(config, 10),
            area.y + scale_px(config, 8),
            colors.secondary_text,
            &format!("Sector {}", snapshot.sector),
        );
        let sectors = [
            (
                "S1",
                snapshot
                    .current_sector1_seconds
                    .or(snapshot.last_sector1_seconds),
                snapshot.best_sector1_seconds,
            ),
            (
                "S2",
                snapshot
                    .current_sector2_seconds
                    .or(snapshot.last_sector2_seconds),
                snapshot.best_sector2_seconds,
            ),
            (
                "S3",
                snapshot.last_sector3_seconds,
                snapshot.best_sector3_seconds,
            ),
        ];
        let mut x = area.x + scale_px(config, 10);
        for (label, current, best) in sectors {
            let delta = current.zip(best).map(|(current, best)| current - best);
            let color = delta.map_or(colors.secondary_text, |delta| {
                if delta < -0.01 {
                    colors.delta_gain
                } else if delta > 0.01 {
                    colors.delta_loss
                } else {
                    colors.delta_neutral
                }
            });
            draw_text(
                hdc,
                x,
                area.y + scale_px(config, 26),
                color,
                &format!("{label} {}", sector_time(current)),
            );
            x += (area.width / 3).max(scale_size(config, 52));
        }
    }

    unsafe fn draw_mini_sectors_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        if let Some(mini_sector) = snapshot.mini_sector_index {
            let delta = snapshot
                .mini_sector_delta_seconds
                .map(signed_time)
                .unwrap_or_else(|| "--".to_string());
            draw_text(
                hdc,
                area.x + scale_px(config, 10),
                area.y + scale_px(config, 8),
                colors.secondary_text,
                &format!("MS {}  {}", mini_sector + 1, delta),
            );
        } else {
            draw_text(
                hdc,
                area.x + scale_px(config, 10),
                area.y + scale_px(config, 8),
                colors.secondary_text,
                "MS --",
            );
        }
    }

    unsafe fn draw_coaching_widget(
        hdc: HDC,
        snapshot: TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
    ) {
        let colors = colors(config);
        let mut y = area.y + scale_px(config, 8);
        let x = area.x + scale_px(config, 10);
        let mut hints = 0_u8;
        let max_hints = config.coaching.max_hints.max(1);
        if config.coaching.brake_timing {
            if let Some(hint) = snapshot
                .brake_hint_meters
                .filter(|hint| hint.abs() >= config.coaching.timing_deadband_m)
            {
                if hints < max_hints {
                    draw_text(
                        hdc,
                        x,
                        y,
                        timing_color(hint, colors),
                        &format!("BRAKE {}", timing_hint(hint)),
                    );
                    hints += 1;
                    y += scale_size(config, 18);
                }
            }
        }
        if config.coaching.throttle_timing {
            if let Some(hint) = snapshot
                .throttle_hint_meters
                .filter(|hint| hint.abs() >= config.coaching.timing_deadband_m)
            {
                if hints < max_hints {
                    draw_text(
                        hdc,
                        x,
                        y,
                        timing_color(hint, colors),
                        &format!("THROTTLE {}", timing_hint(hint)),
                    );
                    hints += 1;
                    y += scale_size(config, 18);
                }
            }
        }
        if config.coaching.input_match && hints < max_hints {
            if let Some(message) = input_coaching_message(snapshot) {
                draw_text(hdc, x, y, colors.reference, message);
                hints += 1;
                y += scale_size(config, 18);
            }
        }
        if config.coaching.speed {
            if let Some(speed_gap) = snapshot
                .speed_hint_kph
                .filter(|gap| gap.abs() >= config.coaching.speed_threshold_kph)
            {
                if hints < max_hints {
                    draw_text(
                        hdc,
                        x,
                        y,
                        if speed_gap > 0.0 {
                            colors.coaching_positive
                        } else {
                            colors.coaching_warning
                        },
                        &format!("{speed_gap:+.0} km/h ENTRY"),
                    );
                    hints += 1;
                    y += scale_size(config, 18);
                }
            }
        }
        if config.coaching.gear && hints < max_hints {
            if let Some(reference_gear) = snapshot.reference_gear {
                let gear = gear_number(snapshot.gear);
                if reference_gear != gear {
                    draw_text(
                        hdc,
                        x,
                        y,
                        colors.secondary_text,
                        &format!("USE {reference_gear}{}", gear_suffix(reference_gear)),
                    );
                }
            }
        }
    }

    unsafe fn draw_performance_monitor(hdc: HDC, state: &SharedState, config: &OverlayConfig) {
        let colors = colors(config);
        let area = area_from_layout(&config.layout.performance);
        draw_widget_panel(hdc, area, config);
        let ring_usage = state
            .history
            .lock()
            .ok()
            .map(|history| format!("{}/{}", history.len(), history.capacity()))
            .unwrap_or_else(|| "--".to_string());
        if let Ok(stats) = state.stats.lock() {
            draw_text(
                hdc,
                area.x + scale_px(config, 8),
                area.y + scale_px(config, 2),
                colors.secondary_text,
                &format!(
                    "tel {} Hz  draw {} FPS  read {:.2} ms  draw {:.2} ms  skip {}  drop {}  ring {}",
                    stats.telemetry_hz,
                    stats.render_fps,
                    stats.acquisition_ms,
                    stats.render_ms,
                    stats.skipped_samples,
                    stats.dropped_frames,
                    ring_usage
                ),
            );
        }
    }

    unsafe fn edit_mode_hit_test(hwnd: HWND, lparam: LPARAM) -> Option<LRESULT> {
        let state = shared_state(hwnd)?;
        if !state.edit_mode.load(Ordering::Relaxed) {
            return None;
        }

        let mut point = POINT {
            x: loword_signed(lparam),
            y: hiword_signed(lparam),
        };
        ScreenToClient(hwnd, &mut point);

        let mut rect: RECT = zeroed();
        GetClientRect(hwnd, &mut rect);
        if state
            .config
            .lock()
            .ok()
            .and_then(|config| hit_widget(&config, point.x, point.y))
            .is_some()
        {
            return Some(HTCLIENT as LRESULT);
        }

        let near_right = point.x >= rect.right.saturating_sub(EDIT_HIT_MARGIN);
        let near_bottom = point.y >= rect.bottom.saturating_sub(EDIT_HIT_MARGIN);

        if near_right && near_bottom {
            Some(HTBOTTOMRIGHT as LRESULT)
        } else if near_right {
            Some(HTRIGHT as LRESULT)
        } else if near_bottom {
            Some(HTBOTTOM as LRESULT)
        } else {
            Some(HTCAPTION as LRESULT)
        }
    }

    fn loword_signed(value: LPARAM) -> i32 {
        (value as u32 & 0xffff) as i16 as i32
    }

    fn hiword_signed(value: LPARAM) -> i32 {
        ((value as u32 >> 16) & 0xffff) as i16 as i32
    }

    fn hit_widget(config: &OverlayConfig, x: i32, y: i32) -> Option<(WidgetId, Area)> {
        if config.layout.lock_all {
            return None;
        }

        widget_areas(config)
            .into_iter()
            .rev()
            .find(|(widget, area)| area.contains(x, y) && !widget_layout(config, *widget).locked)
    }

    fn widget_areas(config: &OverlayConfig) -> Vec<(WidgetId, Area)> {
        let mut areas: Vec<_> = [
            (WidgetId::Telemetry, &config.layout.telemetry),
            (WidgetId::Inputs, &config.layout.inputs),
            (WidgetId::LapTiming, &config.layout.lap_timing),
            (WidgetId::Timing, &config.layout.timing),
            (WidgetId::Sectors, &config.layout.sectors),
            (WidgetId::MiniSectors, &config.layout.mini_sectors),
            (WidgetId::Coaching, &config.layout.coaching),
            (WidgetId::Performance, &config.layout.performance),
        ]
        .into_iter()
        .map(|(id, layout)| (id, area_from_layout(layout)))
        .collect();
        for id in EXTRA_WIDGET_IDS {
            if let Some(widget) = config
                .extra_widgets
                .get(*id)
                .filter(|widget| widget.enabled)
            {
                areas.push((WidgetId::Extra(*id), area_from_layout(&widget.layout)));
            }
        }
        areas.sort_by_key(|(id, _)| widget_layout(config, *id).z_index);
        areas
    }

    fn area_from_layout(layout: &WidgetLayout) -> Area {
        Area {
            x: layout.x,
            y: layout.y,
            width: (f64::from(layout.width) * layout.scale).round() as i32,
            height: (f64::from(layout.height) * layout.scale).round() as i32,
        }
    }

    fn widget_layout(config: &OverlayConfig, widget: WidgetId) -> &WidgetLayout {
        match widget {
            WidgetId::Telemetry => &config.layout.telemetry,
            WidgetId::Inputs => &config.layout.inputs,
            WidgetId::LapTiming => &config.layout.lap_timing,
            WidgetId::Timing => &config.layout.timing,
            WidgetId::Sectors => &config.layout.sectors,
            WidgetId::MiniSectors => &config.layout.mini_sectors,
            WidgetId::Coaching => &config.layout.coaching,
            WidgetId::Performance => &config.layout.performance,
            WidgetId::Extra(id) => {
                &config
                    .extra_widgets
                    .get(id)
                    .expect("widget area only contains configured extra widgets")
                    .layout
            }
        }
    }

    fn widget_layout_mut(config: &mut OverlayConfig, widget: WidgetId) -> &mut WidgetLayout {
        match widget {
            WidgetId::Telemetry => &mut config.layout.telemetry,
            WidgetId::Inputs => &mut config.layout.inputs,
            WidgetId::LapTiming => &mut config.layout.lap_timing,
            WidgetId::Timing => &mut config.layout.timing,
            WidgetId::Sectors => &mut config.layout.sectors,
            WidgetId::MiniSectors => &mut config.layout.mini_sectors,
            WidgetId::Coaching => &mut config.layout.coaching,
            WidgetId::Performance => &mut config.layout.performance,
            WidgetId::Extra(id) => {
                &mut config
                    .extra_widgets
                    .get_mut(id)
                    .expect("widget area only contains configured extra widgets")
                    .layout
            }
        }
    }

    fn snap_widget_to_edges(
        layout: &mut WidgetLayout,
        window_width: i32,
        window_height: i32,
        snap_distance: i32,
    ) {
        if snap_distance <= 0 {
            return;
        }

        if layout.x.abs() <= snap_distance {
            layout.x = 0;
        }
        if layout.y.abs() <= snap_distance {
            layout.y = 0;
        }
        let right_gap = window_width - (layout.x + layout.width);
        if right_gap.abs() <= snap_distance {
            layout.x = window_width - layout.width;
        }
        let bottom_gap = window_height - (layout.y + layout.height);
        if bottom_gap.abs() <= snap_distance {
            layout.y = window_height - layout.height;
        }
    }

    fn snap_widget_to_grid(layout: &mut WidgetLayout, grid_size: i32) {
        let grid_size = if matches!(grid_size, 5 | 10 | 20) {
            grid_size
        } else {
            10
        };
        layout.x = snap_i32(layout.x, grid_size);
        layout.y = snap_i32(layout.y, grid_size);
        layout.width = snap_i32(layout.width, grid_size).max(48);
        layout.height = snap_i32(layout.height, grid_size).max(20);
    }

    fn snap_widget_to_widgets(config: &mut OverlayConfig, widget: WidgetId) {
        let snap_distance = config.layout.snap_distance;
        if snap_distance <= 0 {
            return;
        }
        let others = widget_areas(config)
            .into_iter()
            .filter(|(id, _)| *id != widget)
            .map(|(_, area)| area)
            .collect::<Vec<_>>();
        let layout = widget_layout_mut(config, widget);
        let mut area = area_from_layout(layout);

        for other in others {
            if (area.x - other.x).abs() <= snap_distance {
                layout.x = other.x;
            } else if (area.x - other.right()).abs() <= snap_distance {
                layout.x = other.right();
            } else if (area.right() - other.x).abs() <= snap_distance {
                layout.x = other.x - area.width;
            } else if (area.right() - other.right()).abs() <= snap_distance {
                layout.x = other.right() - area.width;
            }

            if (area.y - other.y).abs() <= snap_distance {
                layout.y = other.y;
            } else if (area.y - other.bottom()).abs() <= snap_distance {
                layout.y = other.bottom();
            } else if (area.bottom() - other.y).abs() <= snap_distance {
                layout.y = other.y - area.height;
            } else if (area.bottom() - other.bottom()).abs() <= snap_distance {
                layout.y = other.bottom() - area.height;
            }
            area = area_from_layout(layout);
        }
    }

    fn snap_i32(value: i32, grid_size: i32) -> i32 {
        ((value as f64 / f64::from(grid_size)).round() as i32) * grid_size
    }

    unsafe fn draw_edit_handles(hdc: HDC, config: &OverlayConfig, selected: Option<WidgetId>) {
        let colors = colors(config);
        let pen = CreatePen(
            PS_SOLID,
            config.style.line_thickness.max(2),
            colors.reference,
        );
        let old_pen = SelectObject(hdc, pen);

        for (widget, area) in widget_areas(config) {
            if widget_layout(config, widget).locked {
                continue;
            }
            Rectangle(hdc, area.x, area.y, area.right(), area.bottom());
            if selected == Some(widget) {
                let right = area.right().saturating_sub(scale_size(config, 6));
                let bottom = area.bottom().saturating_sub(scale_size(config, 6));
                let step = scale_size(config, 5);
                for index in 0..3 {
                    let inset = step * index;
                    MoveToEx(
                        hdc,
                        right - scale_size(config, 22) + inset,
                        bottom,
                        ptr::null_mut(),
                    );
                    LineTo(hdc, right, bottom - scale_size(config, 22) + inset);
                }
            }
        }

        SelectObject(hdc, old_pen);
        DeleteObject(pen);
    }

    unsafe fn draw_series(
        hdc: HDC,
        history: &RingBuffer<TelemetrySnapshot>,
        area: Area,
        color: u32,
        line_width: i32,
        value: impl Fn(TelemetrySnapshot) -> f64,
    ) {
        let pen = CreatePen(PS_SOLID, line_width.max(1), color);
        let old_pen = SelectObject(hdc, pen);
        let segment_count = history.len().saturating_sub(1).max(1) as f64;
        let mut previous = None;
        for (index, sample) in history.iter().copied().enumerate() {
            let Some(before) = previous else {
                previous = Some(sample);
                continue;
            };

            let x1 = area.x + (((index - 1) as f64 / segment_count) * area.width as f64) as i32;
            let x2 = area.x + ((index as f64 / segment_count) * area.width as f64) as i32;
            let y1 =
                area.y + area.height - (value(before).clamp(0.0, 1.0) * area.height as f64) as i32;
            let y2 =
                area.y + area.height - (value(sample).clamp(0.0, 1.0) * area.height as f64) as i32;
            MoveToEx(hdc, x1, y1, ptr::null_mut());
            LineTo(hdc, x2, y2);
            previous = Some(sample);
        }
        SelectObject(hdc, old_pen);
        DeleteObject(pen);
    }

    unsafe fn draw_text(hdc: HDC, x: i32, y: i32, color: u32, text: &str) {
        let wide: Vec<u16> = text.encode_utf16().collect();
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, color);
        TextOutW(hdc, x, y, wide.as_ptr(), wide.len() as i32);
    }

    #[derive(Clone, Copy)]
    struct Colors {
        background: u32,
        border: u32,
        primary_text: u32,
        secondary_text: u32,
        throttle: u32,
        brake: u32,
        clutch: u32,
        steering: u32,
        delta_gain: u32,
        delta_loss: u32,
        delta_neutral: u32,
        reference: u32,
        coaching_warning: u32,
        coaching_positive: u32,
    }

    fn colors(config: &OverlayConfig) -> Colors {
        Colors {
            background: parse_color(&config.style.background, 0x00202020),
            border: parse_color(&config.style.border, 0x00666666),
            primary_text: parse_color(&config.style.primary_text, 0x00FFFFFF),
            secondary_text: parse_color(&config.style.secondary_text, 0x00D0D0D0),
            throttle: parse_color(&config.style.throttle, 0x0022DD44),
            brake: parse_color(&config.style.brake, 0x002244EE),
            clutch: parse_color(&config.style.clutch, 0x00DDDD22),
            steering: parse_color(&config.style.steering, 0x00EEEEEE),
            delta_gain: parse_color(&config.style.delta_gain, 0x0022DD44),
            delta_loss: parse_color(&config.style.delta_loss, 0x002244EE),
            delta_neutral: parse_color(&config.style.delta_neutral, 0x0057BCF2),
            reference: parse_color(&config.style.reference, 0x00AAAAAA),
            coaching_warning: parse_color(&config.style.coaching_warning, 0x0057BCF2),
            coaching_positive: parse_color(&config.style.coaching_positive, 0x0022DD44),
        }
    }

    fn signed_time(seconds: f64) -> String {
        format!("{seconds:+.3}")
    }

    fn sector_time(seconds: Option<f64>) -> String {
        seconds
            .map(|seconds| format!("{seconds:.3}"))
            .unwrap_or_else(|| "--".to_string())
    }

    fn lap_time(seconds: f64) -> String {
        let minutes = (seconds / 60.0).floor() as u32;
        let seconds = seconds - f64::from(minutes) * 60.0;
        format!("{minutes}:{seconds:06.3}")
    }

    fn display_speed(speed_kph: f64, config: &OverlayConfig) -> f64 {
        if config.units.speed == "mph" {
            speed_kph * 0.621_371
        } else {
            speed_kph
        }
    }

    fn speed_unit_label(config: &OverlayConfig) -> &'static str {
        if config.units.speed == "mph" {
            "mph"
        } else {
            "km/h"
        }
    }

    fn timing_hint(meters: f64) -> String {
        if meters >= 0.0 {
            format!("{meters:.0}m LATE")
        } else {
            format!("{:.0}m EARLY", meters.abs())
        }
    }

    fn gear_suffix(gear: i32) -> &'static str {
        match gear {
            1 => "ST",
            2 => "ND",
            3 => "RD",
            _ => "TH",
        }
    }

    fn input_coaching_message(snapshot: TelemetrySnapshot) -> Option<&'static str> {
        if snapshot
            .reference_brake
            .is_some_and(|reference| snapshot.brake > reference + 0.12)
        {
            return Some("release brake");
        }

        if snapshot
            .reference_throttle
            .is_some_and(|reference| snapshot.throttle + 0.12 < reference)
            && snapshot.brake < 0.08
        {
            return Some("more throttle");
        }

        if snapshot
            .reference_speed_kph
            .is_some_and(|reference| snapshot.speed_kph + 8.0 < reference)
        {
            return Some("carry speed");
        }

        None
    }

    fn gear_number(gear: telemetry_engine::Gear) -> i32 {
        match gear {
            telemetry_engine::Gear::Reverse => -1,
            telemetry_engine::Gear::Neutral => 0,
            telemetry_engine::Gear::Forward(value) => i32::from(value),
            telemetry_engine::Gear::Unknown(value) => value,
        }
    }

    fn timing_color(delta_meters: f64, colors: Colors) -> u32 {
        if delta_meters <= 0.0 {
            colors.delta_gain
        } else {
            colors.delta_loss
        }
    }

    fn scale_px(config: &OverlayConfig, value: i32) -> i32 {
        (f64::from(value) * config.style.scale).round() as i32
    }

    fn scale_size(config: &OverlayConfig, value: i32) -> i32 {
        scale_px(config, value).max(1)
    }

    fn hotkey(value: &str) -> Option<(u32, u32)> {
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
                _ => key = virtual_key(&part),
            }
        }
        key.map(|key| (modifiers, key))
    }

    fn virtual_key(value: &str) -> Option<u32> {
        let key = value.trim().to_ascii_uppercase();
        match key.as_str() {
            "F1" => Some(0x70),
            "F2" => Some(0x71),
            "F3" => Some(0x72),
            "F4" => Some(0x73),
            "F5" => Some(0x74),
            "F6" => Some(0x75),
            "F7" => Some(0x76),
            "F8" => Some(0x77),
            "F9" => Some(0x78),
            "F10" => Some(0x79),
            "F11" => Some(0x7A),
            "F12" => Some(0x7B),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use telemetry_engine::{GamePhase, Gear, SessionKind};

        #[test]
        fn formats_direct_timing_hints() {
            assert_eq!(timing_hint(25.0), "25m LATE");
            assert_eq!(timing_hint(-12.0), "12m EARLY");
            assert_eq!(gear_suffix(1), "ST");
            assert_eq!(gear_suffix(3), "RD");
            assert_eq!(sector_time(Some(31.4567)), "31.457");
            assert_eq!(sector_time(None), "--");
        }

        #[test]
        fn parses_hotkey_modifiers() {
            assert_eq!(
                hotkey("Ctrl+Shift+F9"),
                Some((MOD_CONTROL | MOD_SHIFT, 0x78))
            );
            assert_eq!(hotkey("Alt+F10"), Some((MOD_ALT, 0x79)));
            assert_eq!(hotkey("Nope"), None);
        }

        #[test]
        fn picks_input_coaching_message_from_reference_gap() {
            let mut braking = snapshot();
            braking.brake = 0.4;
            braking.reference_brake = Some(0.1);
            assert_eq!(input_coaching_message(braking), Some("release brake"));

            let mut throttle = snapshot();
            throttle.throttle = 0.4;
            throttle.reference_throttle = Some(0.8);
            assert_eq!(input_coaching_message(throttle), Some("more throttle"));

            let mut speed = snapshot();
            speed.speed_kph = 180.0;
            speed.reference_speed_kph = Some(200.0);
            assert_eq!(input_coaching_message(speed), Some("carry speed"));
        }

        fn snapshot() -> TelemetrySnapshot {
            TelemetrySnapshot {
                throttle: 1.0,
                brake: 0.0,
                clutch: 0.0,
                steering: 0.0,
                rpm: 7_000.0,
                gear: Gear::Forward(4),
                speed_kph: 200.0,
                lap_distance_m: Some(1_000.0),
                track_length_m: Some(5_000.0),
                lap_time_seconds: Some(10.0),
                lap_progress: Some(0.2),
                lap_number: 1,
                sector: 1,
                current_sector1_seconds: None,
                current_sector2_seconds: None,
                last_sector1_seconds: None,
                last_sector2_seconds: None,
                last_sector3_seconds: None,
                best_sector1_seconds: None,
                best_sector2_seconds: None,
                best_sector3_seconds: None,
                vehicle: lmu_telemetry::VehicleSystems::default(),
                wheels: lmu_telemetry::Wheels::default(),
                session: lmu_telemetry::SessionData::default(),
                session_elapsed_seconds: 10.0,
                session_kind: SessionKind::Practice,
                game_phase: GamePhase::GreenFlag,
                in_pits: false,
                in_garage: false,
                lap_invalidated: None,
                player_slot_id: 42,
                delta_seconds: None,
                predicted_lap_seconds: None,
                session_best_seconds: None,
                personal_best_seconds: None,
                reference_lap_seconds: None,
                mini_sector_index: None,
                mini_sector_delta_seconds: None,
                brake_hint_meters: None,
                throttle_hint_meters: None,
                speed_hint_kph: None,
                reference_gear: None,
                reference_steering: None,
                reference_throttle: None,
                reference_brake: None,
                reference_speed_kph: None,
            }
        }
    }
}

#[cfg(windows)]
pub use windows_overlay::TelemetryOverlay;

#[cfg(not(windows))]
pub struct TelemetryOverlay;

#[cfg(not(windows))]
impl TelemetryOverlay {
    pub fn new() -> Result<Self, OverlayError> {
        Err(OverlayError::UnsupportedPlatform)
    }

    pub fn with_config(_config: OverlayConfig) -> Result<Self, OverlayError> {
        Err(OverlayError::UnsupportedPlatform)
    }

    pub fn with_config_path(
        _config: OverlayConfig,
        _config_path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, OverlayError> {
        Err(OverlayError::UnsupportedPlatform)
    }

    pub fn run<F>(self, _next_snapshot: F) -> Result<(), OverlayError>
    where
        F: FnMut() -> Option<TelemetrySnapshot> + Send + 'static,
    {
        Err(OverlayError::UnsupportedPlatform)
    }
}
