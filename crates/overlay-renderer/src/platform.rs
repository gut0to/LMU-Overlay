use std::{error::Error, fmt};

use crate::config;
use crate::config::OverlayConfig;
#[cfg(not(windows))]
use telemetry_engine::TelemetrySnapshot;

#[cfg(windows)]
#[path = "d2d_backend.rs"]
mod d2d_backend;

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
        cell::{Cell, RefCell},
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
            InvalidateRect, LineTo, MoveToEx, Rectangle, RoundRect, ScreenToClient, SelectObject,
            SetBkMode, SetTextColor, TextOutW, HDC, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
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

    use super::d2d_backend;
    use crate::config::{WidgetLayout, WidgetOptions, WidgetStyleConfig};

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

    thread_local! {
        static WIDGET_RENDER_STATE: Cell<(f64, u32)> = const { Cell::new((1.0, COLOR_KEY)) };
        static NATIVE_TEXT_ENABLED: Cell<bool> = const { Cell::new(false) };
        static NATIVE_SHAPE_COMMANDS: RefCell<Vec<d2d_backend::ShapeCommand>> = const { RefCell::new(Vec::new()) };
        static NATIVE_TEXT_COMMANDS: RefCell<Vec<d2d_backend::TextCommand>> = const { RefCell::new(Vec::new()) };
    }

    struct WidgetOpacityScope {
        previous: (f64, u32),
    }

    impl WidgetOpacityScope {
        fn new(opacity: f64) -> Self {
            let previous = WIDGET_RENDER_STATE.with(|state| {
                let previous = state.get();
                state.set((opacity.clamp(0.1, 1.0), previous.1));
                previous
            });
            Self { previous }
        }
    }

    impl Drop for WidgetOpacityScope {
        fn drop(&mut self) {
            WIDGET_RENDER_STATE.with(|state| state.set(self.previous));
        }
    }

    fn set_render_background(color: u32) {
        WIDGET_RENDER_STATE.with(|state| {
            let (opacity, _) = state.get();
            state.set((opacity, color));
        });
    }

    fn widget_color(color: u32) -> u32 {
        WIDGET_RENDER_STATE.with(|state| {
            let (opacity, background) = state.get();
            blend_color(color, background, opacity)
        })
    }

    fn blend_color(color: u32, background: u32, opacity: f64) -> u32 {
        let opacity = opacity.clamp(0.0, 1.0);
        let channel = |shift: u32| {
            let foreground = ((color >> shift) & 0xff) as f64;
            let background = ((background >> shift) & 0xff) as f64;
            (foreground * opacity + background * (1.0 - opacity)).round() as u32
        };
        channel(0) | (channel(8) << 8) | (channel(16) << 16)
    }

    fn begin_native_text(enabled: bool) {
        NATIVE_TEXT_ENABLED.with(|state| state.set(enabled));
        NATIVE_SHAPE_COMMANDS.with(|commands| commands.borrow_mut().clear());
        NATIVE_TEXT_COMMANDS.with(|commands| commands.borrow_mut().clear());
    }

    fn queue_shape(command: d2d_backend::ShapeCommand) {
        NATIVE_SHAPE_COMMANDS.with(|commands| commands.borrow_mut().push(command));
    }

    fn take_native_shape_commands() -> Vec<d2d_backend::ShapeCommand> {
        NATIVE_SHAPE_COMMANDS.with(|commands| std::mem::take(&mut *commands.borrow_mut()))
    }

    fn take_native_text_commands() -> Vec<d2d_backend::TextCommand> {
        NATIVE_TEXT_ENABLED.with(|state| state.set(false));
        NATIVE_TEXT_COMMANDS.with(|commands| std::mem::take(&mut *commands.borrow_mut()))
    }

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
        d2d: Option<Arc<d2d_backend::D2dBackend>>,
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
                    d2d: None,
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
                            *latest = Some(snapshot.clone());
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
                if let Some(backend) = state.d2d.as_ref() {
                    unsafe {
                        if let Err(error) =
                            backend.resize(config.window.width as u32, config.window.height as u32)
                        {
                            log::warn!("Could not resize Direct2D render target: {error}");
                        }
                    }
                }
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

            match d2d_backend::D2dBackend::new(
                windows::Win32::Foundation::HWND(hwnd),
                width as u32,
                height as u32,
            ) {
                Ok(backend) => {
                    // The D2D device is created and consumed on the overlay UI thread.
                    #[allow(clippy::arc_with_non_send_sync)]
                    {
                        (*state_ptr).d2d = Some(Arc::new(backend));
                    }
                }
                Err(error) => {
                    log::warn!("Direct2D initialization failed; using GDI fallback: {error}")
                }
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
        let state_ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        ) as *mut SharedState;

        if state_ptr.is_null() {
            return;
        }

        let state = &*state_ptr;
        let d2d_hdc = state
            .d2d
            .as_ref()
            .and_then(|backend| backend.begin_gdi().ok());
        let using_d2d = d2d_hdc.is_some();
        begin_native_text(using_d2d);
        let hdc = d2d_hdc.unwrap_or_else(|| BeginPaint(hwnd, &mut paint));
        let mut rect: RECT = zeroed();
        GetClientRect(hwnd, &mut rect);

        if using_d2d {
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: 0.0,
                top: 0.0,
                right: rect.right as f32,
                bottom: rect.bottom as f32,
                fill: Some(COLOR_KEY),
                stroke: None,
                stroke_width: 0.0,
                radius: 0.0,
            });
        } else {
            let black = CreateSolidBrush(COLOR_KEY);
            FillRect(hdc, &rect, black);
            DeleteObject(black);
        }

        let config = state
            .config
            .lock()
            .ok()
            .map(|config| config.clone())
            .unwrap_or_default();
        set_render_background(colors(&config).background);
        if let Ok(mut stats) = state.stats.lock() {
            stats.render_frames += 1;
        }
        let latest = state.latest.lock().ok().and_then(|value| value.clone());

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

        if let Some(backend) = state.d2d.as_ref() {
            if using_d2d {
                let texts = take_native_text_commands();
                let shapes = take_native_shape_commands();
                if let Err(error) = backend.end_gdi(
                    &shapes,
                    &texts,
                    &config.style.font_family,
                    config.style.font_size as f32 * config.style.scale as f32,
                    config.style.font_weight,
                    config.window.width,
                    config.window.height,
                ) {
                    log::warn!("Direct2D frame submission failed: {error}");
                }
            } else {
                EndPaint(hwnd, &paint);
            }
        } else {
            EndPaint(hwnd, &paint);
        }
    }

    unsafe fn draw_panel(hdc: HDC, config: &OverlayConfig) {
        let colors = colors(config);
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: 0.0,
                top: 0.0,
                right: config.window.width as f32,
                bottom: config.window.height as f32,
                fill: Some(colors.background),
                stroke: Some(colors.border),
                stroke_width: config.style.line_thickness.max(1) as f32,
                radius: config.style.border_radius.max(0) as f32,
            });
            return;
        }
        let bg = CreateSolidBrush(colors.background);
        let border = CreatePen(PS_SOLID, config.style.line_thickness, colors.border);
        let old_brush = SelectObject(hdc, bg);
        let old_pen = SelectObject(hdc, border);
        if config.style.border_radius > 0 {
            RoundRect(
                hdc,
                0,
                0,
                config.window.width,
                config.window.height,
                config.style.border_radius * 2,
                config.style.border_radius * 2,
            );
        } else {
            Rectangle(hdc, 0, 0, config.window.width, config.window.height);
        }
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(border);
        DeleteObject(bg);
    }

    unsafe fn draw_widget_panel(hdc: HDC, area: Area, config: &OverlayConfig) {
        let colors = colors(config);
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: area.x as f32,
                top: area.y as f32,
                right: (area.x + area.width) as f32,
                bottom: (area.y + area.height) as f32,
                fill: None,
                stroke: Some(widget_color(colors.border)),
                stroke_width: config.style.line_thickness.max(1) as f32,
                radius: config.style.border_radius.max(0) as f32,
            });
            return;
        }
        let border = CreatePen(
            PS_SOLID,
            config.style.line_thickness.max(1),
            widget_color(colors.border),
        );
        let old_pen = SelectObject(hdc, border);
        if config.style.border_radius > 0 {
            RoundRect(
                hdc,
                area.x,
                area.y,
                area.x + area.width,
                area.y + area.height,
                config.style.border_radius * 2,
                config.style.border_radius * 2,
            );
        } else {
            Rectangle(
                hdc,
                area.x,
                area.y,
                area.x + area.width,
                area.y + area.height,
            );
        }
        SelectObject(hdc, old_pen);
        DeleteObject(border);
    }

    unsafe fn draw_snapshot(hdc: HDC, snapshot: TelemetrySnapshot, config: &OverlayConfig) {
        for (widget, area) in widget_areas(config) {
            let _opacity = WidgetOpacityScope::new(widget_layout(config, widget).opacity);
            match widget {
                WidgetId::Telemetry if config.widgets.title || config.widgets.speed_gear_rpm => {
                    draw_widget_panel(hdc, area, config);
                    draw_telemetry_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::LapTiming if config.widgets.lap_info || config.widgets.lap_timing => {
                    draw_widget_panel(hdc, area, config);
                    draw_lap_timing_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::Inputs
                    if config.widgets.pedals
                        || config.widgets.steering
                        || config.widgets.input_history =>
                {
                    draw_widget_panel(hdc, area, config);
                    draw_input_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::Timing if config.widgets.delta_timing => {
                    draw_widget_panel(hdc, area, config);
                    draw_delta_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::Sectors if config.widgets.sectors => {
                    draw_widget_panel(hdc, area, config);
                    draw_sectors_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::MiniSectors if config.widgets.mini_sector_widget => {
                    draw_widget_panel(hdc, area, config);
                    draw_mini_sectors_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::Coaching if config.widgets.coaching && config.coaching.mode != "off" => {
                    draw_widget_panel(hdc, area, config);
                    draw_coaching_widget(hdc, snapshot.clone(), config, area);
                }
                WidgetId::Extra(id) => {
                    draw_extra_widget(hdc, snapshot.clone(), config, area, id);
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
        let widget_options = config.extra_widgets.get(id).map(|widget| &widget.options);
        let options = widget_options.cloned().unwrap_or_default();
        draw_extra_widget_panel(hdc, area, config, widget_style);
        let padding = widget_style.map_or(scale_px(config, 8), |style| style.padding);
        if id == "relative" {
            draw_relative_widget(
                hdc,
                &snapshot,
                config,
                area,
                padding,
                widget_style,
                widget_options,
            );
            return;
        }
        if id == "standings" {
            draw_standings_widget(
                hdc,
                &snapshot,
                config,
                area,
                padding,
                widget_style,
                widget_options,
            );
            return;
        }
        if id == "lap_history" {
            draw_lap_history_widget(
                hdc,
                &snapshot,
                config,
                area,
                padding,
                widget_style,
                widget_options,
            );
            return;
        }
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
                .map_or_else(|| "FLAG --".to_string(), semantic_flag),
            "fuel" => match (snapshot.fuel_current_liters, snapshot.fuel_capacity_liters) {
                (Some(fuel), Some(capacity)) if capacity > 0.0 => {
                    format!(
                        "{}  {:.0}%",
                        display_fuel(fuel, config),
                        fuel / capacity * 100.0
                    )
                }
                (Some(fuel), _) => display_fuel(fuel, config),
                _ => "FUEL --".to_string(),
            },
            "tyres" => wheel_summary(
                "TYRES",
                [
                    display_pressure_value(snapshot.wheels.front_left.pressure_kpa, config),
                    display_pressure_value(snapshot.wheels.front_right.pressure_kpa, config),
                    display_pressure_value(snapshot.wheels.rear_left.pressure_kpa, config),
                    display_pressure_value(snapshot.wheels.rear_right.pressure_kpa, config),
                ],
                pressure_unit_label(config),
            ),
            "brakes" => wheel_summary(
                "BRAKES",
                [
                    display_temperature_value(snapshot.wheels.front_left.brake_temp_c, config),
                    display_temperature_value(snapshot.wheels.front_right.brake_temp_c, config),
                    display_temperature_value(snapshot.wheels.rear_left.brake_temp_c, config),
                    display_temperature_value(snapshot.wheels.rear_right.brake_temp_c, config),
                ],
                temperature_unit_label(config),
            ),
            "electronics" => match (snapshot.vehicle.tc_setting, snapshot.vehicle.abs_setting) {
                (Some(tc), Some(abs)) => format!("TC {tc}  ABS {abs}"),
                _ => "TC / ABS --".to_string(),
            },
            "energy" => match (
                snapshot.vehicle.battery_charge_percent,
                snapshot.vehicle.virtual_energy_percent,
            ) {
                (Some(charge), Some(energy)) => {
                    format!("SOC {charge:.0}%  VE {energy:.0}%")
                }
                (Some(charge), None) => format!("SOC {charge:.0}%"),
                _ => "ENERGY --".to_string(),
            },
            "engine" => match (
                snapshot.vehicle.engine_water_temp_c,
                snapshot.vehicle.engine_oil_temp_c,
            ) {
                (Some(water), Some(oil)) => format!(
                    "W {}{}  O {}{}",
                    display_temperature_value(Some(water), config)
                        .unwrap_or_default()
                        .round(),
                    temperature_unit_label(config),
                    display_temperature_value(Some(oil), config)
                        .unwrap_or_default()
                        .round(),
                    temperature_unit_label(config)
                ),
                _ => "ENGINE --".to_string(),
            },
            "weather" => match (
                snapshot.session.ambient_temp_c,
                snapshot.session.track_temp_c,
            ) {
                (Some(ambient), Some(track)) => format!(
                    "AIR {}{}  TRACK {}{}",
                    display_temperature_value(Some(ambient), config)
                        .unwrap_or_default()
                        .round(),
                    temperature_unit_label(config),
                    display_temperature_value(Some(track), config)
                        .unwrap_or_default()
                        .round(),
                    temperature_unit_label(config)
                ),
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
            "lap_history" => "LAP HISTORY".to_string(),
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
        let detail_y = area.y + padding + title_height + scale_px(config, 22);
        let detail_color = widget_secondary_color(config, widget_style);
        match id {
            "fuel" => {
                if let (Some(fuel), Some(capacity)) =
                    (snapshot.fuel_current_liters, snapshot.fuel_capacity_liters)
                {
                    draw_horizontal_meter(
                        hdc,
                        Area {
                            x: area.x + padding,
                            y: detail_y,
                            width: (area.width - padding * 2).max(scale_size(config, 40)),
                            height: scale_size(config, 8),
                        },
                        fuel / capacity,
                        colors(config).throttle,
                        detail_color,
                    );
                }
                let stats_y = detail_y + scale_size(config, 14);
                if options.show_average {
                    if let Some(average) = snapshot.fuel_average_lap_used {
                        draw_text(
                            hdc,
                            area.x + padding,
                            stats_y,
                            detail_color,
                            &format!("AVG {} /lap", display_fuel(average, config)),
                        );
                    }
                }
                if options.show_last_lap {
                    if let Some(last) = snapshot.fuel_last_lap_used {
                        draw_text(
                            hdc,
                            area.x + padding,
                            stats_y + scale_px(config, 18),
                            detail_color,
                            &format!("LAST {} /lap", display_fuel(last, config)),
                        );
                    }
                }
                if options.show_estimated_laps {
                    if let Some(remaining) = snapshot.fuel_estimated_laps_remaining {
                        draw_text(
                            hdc,
                            area.x + padding,
                            stats_y + scale_px(config, 36),
                            detail_color,
                            &format!("REMAIN {remaining:.1} laps"),
                        );
                    }
                }
            }
            "rpm" => {
                if let Some(max_rpm) = snapshot.vehicle.max_rpm {
                    draw_rpm_segments(
                        hdc,
                        Area {
                            x: area.x + padding,
                            y: detail_y,
                            width: (area.width - padding * 2).max(scale_size(config, 60)),
                            height: scale_size(config, 8),
                        },
                        snapshot.rpm,
                        max_rpm,
                        colors(config).reference,
                        detail_color,
                        options.shift_start_percent,
                        options.shift_warning_percent,
                        options.limiter_percent,
                        options.shift_segments,
                    );
                }
            }
            "tyres" => draw_four_wheel_detail(
                hdc,
                area,
                config,
                detail_y,
                detail_color,
                [
                    snapshot.wheels.front_left,
                    snapshot.wheels.front_right,
                    snapshot.wheels.rear_left,
                    snapshot.wheels.rear_right,
                ],
                false,
                options.show_wear,
                &options.tyre_temperature_mode,
            ),
            "brakes" => draw_four_wheel_detail(
                hdc,
                area,
                config,
                detail_y,
                detail_color,
                [
                    snapshot.wheels.front_left,
                    snapshot.wheels.front_right,
                    snapshot.wheels.rear_left,
                    snapshot.wheels.rear_right,
                ],
                true,
                false,
                "surface_average",
            ),
            "electronics" => {
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y,
                    detail_color,
                    &format!(
                        "TC SLIP {}/{}  CUT {}/{}",
                        option_number(snapshot.vehicle.tc_slip),
                        option_number(snapshot.vehicle.tc_slip_max),
                        option_number(snapshot.vehicle.tc_cut),
                        option_number(snapshot.vehicle.tc_cut_max)
                    ),
                );
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y + scale_px(config, 18),
                    detail_color,
                    &format!(
                        "MIG {} / {}",
                        option_number(snapshot.vehicle.migration),
                        option_number(snapshot.vehicle.migration_max)
                    ),
                );
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y + scale_px(config, 36),
                    detail_color,
                    &format!(
                        "WIPER {}  LIMITS {}",
                        option_number(snapshot.vehicle.wiper_state),
                        option_number(snapshot.vehicle.track_limit_steps)
                    ),
                );
            }
            "engine" => {
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y,
                    detail_color,
                    &format!(
                        "BOOST {} kPa",
                        option_decimal(snapshot.vehicle.turbo_boost_kpa)
                    ),
                );
                if snapshot.vehicle.overheating == Some(true) {
                    draw_text(
                        hdc,
                        area.x + padding,
                        detail_y + scale_px(config, 18),
                        colors(config).delta_loss,
                        "OVERHEAT",
                    );
                }
            }
            "energy" => {
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y,
                    detail_color,
                    &format!(
                        "REGEN {}  MTR RPM {}",
                        if snapshot.vehicle.hybrid_regen_active == Some(true) {
                            "ON"
                        } else if snapshot.vehicle.hybrid_regen_active == Some(false) {
                            "OFF"
                        } else {
                            "--"
                        },
                        option_decimal(snapshot.vehicle.electric_motor_rpm)
                    ),
                );
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y + scale_px(config, 18),
                    detail_color,
                    &format!(
                        "MTR TEMP {} C  STATE {}",
                        option_decimal(snapshot.vehicle.electric_motor_temp_c),
                        option_number(snapshot.vehicle.electric_motor_state)
                    ),
                );
            }
            "weather" => draw_text(
                hdc,
                area.x + padding,
                detail_y,
                detail_color,
                &format!(
                    "RAIN {}%  WET {}%",
                    option_percent(snapshot.session.rain_density),
                    option_percent(snapshot.session.track_wetness)
                ),
            ),
            "damage" => {
                let wheels = [
                    ("FL", snapshot.wheels.front_left),
                    ("FR", snapshot.wheels.front_right),
                    ("RL", snapshot.wheels.rear_left),
                    ("RR", snapshot.wheels.rear_right),
                ];
                let damaged = wheels
                    .into_iter()
                    .filter_map(|(label, wheel)| {
                        (wheel.flat == Some(true) || wheel.detached == Some(true)).then_some(label)
                    })
                    .collect::<Vec<_>>();
                let message = if damaged.is_empty() {
                    "WHEELS OK".to_string()
                } else {
                    format!("WHEEL DAMAGE {}", damaged.join(" "))
                };
                draw_text(
                    hdc,
                    area.x + padding,
                    detail_y,
                    if damaged.is_empty() {
                        detail_color
                    } else {
                        colors(config).delta_loss
                    },
                    &message,
                );
            }
            _ => {}
        }
    }

    unsafe fn draw_relative_widget(
        hdc: HDC,
        snapshot: &TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
        padding: i32,
        style: Option<&crate::config::WidgetStyleConfig>,
        options: Option<&WidgetOptions>,
    ) {
        let title_color = widget_secondary_color(config, style);
        let text_color = widget_primary_color(config, style);
        let options = options.cloned().unwrap_or_default();
        draw_text(
            hdc,
            area.x + padding,
            area.y + padding,
            title_color,
            "RELATIVE",
        );
        let player_index = snapshot
            .field
            .iter()
            .position(|car| car.is_player || car.slot_id == snapshot.player_slot_id);
        let Some(player_index) = player_index else {
            draw_text(
                hdc,
                area.x + padding,
                area.y + padding + scale_px(config, 20),
                text_color,
                "SCORING DATA --",
            );
            return;
        };
        let player_class = snapshot.field[player_index].vehicle_class.as_deref();
        let mut cars: Vec<&_> = snapshot
            .field
            .iter()
            .filter(|car| !options.same_class_only || car.vehicle_class.as_deref() == player_class)
            .collect();
        cars.sort_by_key(|car| car.place.unwrap_or(i32::MAX));
        let Some(player_position) = cars
            .iter()
            .position(|car| car.slot_id == snapshot.field[player_index].slot_id)
        else {
            return;
        };
        let start = player_position.saturating_sub(options.cars_ahead as usize);
        let end = (player_position + options.cars_behind as usize + 1).min(cars.len());
        for (row, car) in cars[start..end].iter().enumerate() {
            let y = area.y + padding + scale_px(config, 20 + (row as i32 * 18));
            let marker = if car.is_player || car.slot_id == snapshot.player_slot_id {
                ">"
            } else {
                " "
            };
            let name = if options.show_driver {
                car.driver_name.as_deref().unwrap_or("UNKNOWN")
            } else {
                "CAR"
            };
            let gap = if !options.show_gap {
                String::new()
            } else if car.is_player || car.slot_id == snapshot.player_slot_id {
                "0.000".to_string()
            } else {
                relative_gap(car, &snapshot.field[player_index])
            };
            draw_text(
                hdc,
                area.x + padding,
                y,
                text_color,
                &format!(
                    "{marker} {}{} {}{}{}{}",
                    if options.show_position {
                        format!("P{}", car.place.unwrap_or(0))
                    } else {
                        String::new()
                    },
                    name,
                    if options.show_class {
                        format!(" {}", car.vehicle_class.as_deref().unwrap_or("--"))
                    } else {
                        String::new()
                    },
                    gap,
                    if options.show_gap { "s" } else { "" },
                    if options.show_pit && car.in_pits {
                        " PIT"
                    } else {
                        ""
                    }
                ),
            );
        }
    }

    unsafe fn draw_lap_history_widget(
        hdc: HDC,
        snapshot: &TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
        padding: i32,
        style: Option<&crate::config::WidgetStyleConfig>,
        options: Option<&WidgetOptions>,
    ) {
        let title_color = widget_secondary_color(config, style);
        let text_color = widget_primary_color(config, style);
        let rows = options.map_or(8, |value| value.rows.clamp(1, 20) as usize);
        draw_text(
            hdc,
            area.x + padding,
            area.y + padding,
            title_color,
            "LAP HISTORY",
        );
        if snapshot.lap_history.is_empty() {
            draw_text(
                hdc,
                area.x + padding,
                area.y + padding + scale_px(config, 20),
                text_color,
                "NO COMPLETED LAPS",
            );
            return;
        }
        for (row, entry) in snapshot.lap_history.iter().rev().take(rows).enumerate() {
            let y = area.y + padding + scale_px(config, 20 + row as i32 * 18);
            let time = entry
                .time_seconds
                .map(lap_time)
                .unwrap_or_else(|| "--:--.---".to_string());
            let status = if !entry.valid {
                " INVALID"
            } else if entry
                .delta_to_best
                .is_some_and(|delta| delta.abs() < 0.0005)
            {
                " PB"
            } else {
                ""
            };
            let delta = entry
                .delta_to_best
                .map(|value| format!(" {value:+.3}"))
                .unwrap_or_default();
            draw_text(
                hdc,
                area.x + padding,
                y,
                text_color,
                &format!("L{:>3}  {time}{delta}{status}", entry.lap),
            );
        }
    }

    fn relative_gap(
        car: &lmu_telemetry::VehicleScoringSnapshot,
        player: &lmu_telemetry::VehicleScoringSnapshot,
    ) -> String {
        let lap_delta = car.lap_number - player.lap_number;
        if lap_delta != 0 {
            return format!(
                "{}{}L",
                if lap_delta > 0 { "+" } else { "-" },
                lap_delta.abs()
            );
        }
        car.gap_to_leader_seconds
            .zip(player.gap_to_leader_seconds)
            .map(|(other, own)| format!("{:+.3}", other - own))
            .unwrap_or_else(|| "--".to_string())
    }

    unsafe fn draw_standings_widget(
        hdc: HDC,
        snapshot: &TelemetrySnapshot,
        config: &OverlayConfig,
        area: Area,
        padding: i32,
        style: Option<&crate::config::WidgetStyleConfig>,
        options: Option<&WidgetOptions>,
    ) {
        let title_color = widget_secondary_color(config, style);
        let text_color = widget_primary_color(config, style);
        let options = options.cloned().unwrap_or_default();
        draw_text(
            hdc,
            area.x + padding,
            area.y + padding,
            title_color,
            "STANDINGS",
        );
        let player_class = snapshot
            .field
            .iter()
            .find(|car| car.is_player || car.slot_id == snapshot.player_slot_id)
            .and_then(|car| car.vehicle_class.as_deref());
        let mut cars: Vec<&_> = snapshot
            .field
            .iter()
            .filter(|car| !options.same_class_only || car.vehicle_class.as_deref() == player_class)
            .collect();
        cars.sort_by_key(|car| car.place.unwrap_or(i32::MAX));
        if cars.is_empty() {
            draw_text(
                hdc,
                area.x + padding,
                area.y + padding + scale_px(config, 20),
                text_color,
                "SCORING DATA --",
            );
            return;
        }
        draw_text(
            hdc,
            area.x + padding,
            area.y + padding + scale_px(config, 18),
            title_color,
            "POS DRIVER           LAPS       GAP      LAST      BEST",
        );
        for (row, car) in cars
            .iter()
            .take(options.rows.clamp(1, 20) as usize)
            .enumerate()
        {
            let y = area.y + padding + scale_px(config, 36 + (row as i32 * 18));
            let marker = if car.is_player || car.slot_id == snapshot.player_slot_id {
                ">"
            } else {
                " "
            };
            let name = if options.show_driver {
                car.driver_name.as_deref().unwrap_or("UNKNOWN")
            } else {
                "CAR"
            };
            let lap = if options.show_laps && car.lap_number > 0 {
                format!("L{}", car.lap_number)
            } else if options.show_laps {
                "--".to_string()
            } else {
                String::new()
            };
            let gap = if options.show_gap {
                standings_gap(car)
            } else {
                String::new()
            };
            let last = car
                .last_lap_seconds
                .map(lap_time)
                .unwrap_or_else(|| "--:--.---".to_string());
            let best = car
                .best_lap_seconds
                .map(lap_time)
                .unwrap_or_else(|| "--:--.---".to_string());
            draw_text(
                hdc,
                area.x + padding,
                y,
                text_color,
                &format!(
                    "{marker} {:>2} {:<16} {:>4} {:>8} {:>9} {:>9}{}{}",
                    if options.show_position {
                        format!("{:>2}", car.place.unwrap_or(0))
                    } else {
                        String::new()
                    },
                    name,
                    lap,
                    gap,
                    last,
                    best,
                    if options.show_class {
                        format!(" {}", car.vehicle_class.as_deref().unwrap_or("--"))
                    } else {
                        String::new()
                    },
                    if options.show_pit && car.in_pits {
                        " PIT"
                    } else {
                        ""
                    }
                ),
            );
        }
    }

    fn standings_gap(car: &lmu_telemetry::VehicleScoringSnapshot) -> String {
        if let Some(laps) = car.laps_behind_leader.filter(|laps| *laps != 0) {
            return format!("{}{}L", if laps > 0 { "+" } else { "-" }, laps.abs());
        }
        car.gap_to_leader_seconds
            .map_or_else(|| "--".to_string(), |gap| format!("+{gap:.3}"))
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn draw_four_wheel_detail(
        hdc: HDC,
        area: Area,
        config: &OverlayConfig,
        y: i32,
        color: u32,
        wheels: [lmu_telemetry::WheelData; 4],
        brake: bool,
        show_wear: bool,
        temperature_mode: &str,
    ) {
        let labels = ["FL", "FR", "RL", "RR"];
        for (index, wheel) in wheels.into_iter().enumerate() {
            let card_width = (area.width / 2 - scale_size(config, 12)).max(scale_size(config, 54));
            let x = area.x + scale_px(config, 6) + (index as i32 % 2) * (area.width / 2);
            let row = index as i32 / 2;
            let card = Area {
                x,
                y: y + row * scale_size(config, 30),
                width: card_width,
                height: scale_size(config, 26),
            };
            draw_widget_panel(hdc, card, config);
            let value = if brake {
                format!(
                    "{}  {} {} / {} {}",
                    labels[index],
                    option_decimal(display_temperature_value(wheel.brake_temp_c, config)),
                    temperature_unit_label(config),
                    option_decimal(display_pressure_value(wheel.brake_pressure_kpa, config)),
                    pressure_unit_label(config)
                )
            } else if show_wear {
                format!(
                    "{}  {} {}  W{}%",
                    labels[index],
                    option_decimal(display_pressure_value(wheel.pressure_kpa, config)),
                    pressure_unit_label(config),
                    wheel
                        .wear_percent
                        .map_or_else(|| "--".to_string(), |value| format!("{value:.0}"))
                )
            } else {
                format!(
                    "{}  {} {}  T{} {}",
                    labels[index],
                    option_decimal(display_pressure_value(wheel.pressure_kpa, config)),
                    pressure_unit_label(config),
                    wheel_temperature_text(wheel, config, temperature_mode),
                    temperature_unit_label(config)
                )
            };
            draw_text(
                hdc,
                card.x + scale_px(config, 4),
                card.y + scale_px(config, 5),
                color,
                &value,
            );
        }
    }

    fn wheel_surface_temperature(wheel: lmu_telemetry::WheelData) -> Option<f64> {
        let values = [
            wheel.surface_temp_left_c,
            wheel.surface_temp_center_c,
            wheel.surface_temp_right_c,
        ];
        let mut total = 0.0;
        let mut count = 0;
        for value in values.into_iter().flatten() {
            total += value;
            count += 1;
        }
        (count > 0).then_some(total / f64::from(count))
    }

    fn wheel_temperature_text(
        wheel: lmu_telemetry::WheelData,
        config: &OverlayConfig,
        mode: &str,
    ) -> String {
        let value = match mode {
            "carcass" => display_temperature_value(wheel.carcass_temp_c, config)
                .map(|value| format!("{value:.0}")),
            "inner_layer" => display_temperature_value(wheel.inner_temp_c, config)
                .map(|value| format!("{value:.0}")),
            "surface_lcr" => {
                let values = [
                    wheel.surface_temp_left_c,
                    wheel.surface_temp_center_c,
                    wheel.surface_temp_right_c,
                ];
                values.into_iter().flatten().next().is_some().then(|| {
                    values
                        .into_iter()
                        .map(|value| {
                            display_temperature_value(value, config)
                                .map_or_else(|| "--".to_string(), |value| format!("{value:.0}"))
                        })
                        .collect::<Vec<_>>()
                        .join("/")
                })
            }
            _ => wheel_surface_temperature(wheel)
                .and_then(|value| display_temperature_value(Some(value), config))
                .map(|value| format!("{value:.0}")),
        };
        value.unwrap_or_else(|| "--".to_string())
    }

    fn option_number(value: Option<u8>) -> String {
        value.map_or_else(|| "--".to_string(), |value| value.to_string())
    }
    fn option_decimal(value: Option<f64>) -> String {
        value.map_or_else(|| "--".to_string(), |value| format!("{value:.1}"))
    }
    fn option_percent(value: Option<f64>) -> String {
        value.map_or_else(|| "--".to_string(), |value| format!("{:.0}", value * 100.0))
    }

    fn display_fuel(value_liters: f64, config: &OverlayConfig) -> String {
        if config.units.fuel == "gallons" {
            format!("{:.2} gal", value_liters * 0.2641720524)
        } else {
            format!("{value_liters:.1} L")
        }
    }

    fn display_pressure_value(value_kpa: Option<f64>, config: &OverlayConfig) -> Option<f64> {
        value_kpa.map(|value| {
            if config.units.pressure == "psi" {
                value * 0.1450377377
            } else {
                value
            }
        })
    }

    fn pressure_unit_label(config: &OverlayConfig) -> &'static str {
        if config.units.pressure == "psi" {
            "psi"
        } else {
            "kPa"
        }
    }

    fn display_temperature_value(value_c: Option<f64>, config: &OverlayConfig) -> Option<f64> {
        value_c.map(|value| {
            if config.units.temperature == "fahrenheit" {
                value * 9.0 / 5.0 + 32.0
            } else {
                value
            }
        })
    }

    fn temperature_unit_label(config: &OverlayConfig) -> &'static str {
        if config.units.temperature == "fahrenheit" {
            "F"
        } else {
            "C"
        }
    }

    fn semantic_flag(flag: i32) -> String {
        let label = match flag {
            0 => "GREEN",
            1 => "BLUE",
            2 => "YELLOW",
            3 => "RED",
            4 => "BLACK",
            5 => "WHITE",
            6 => "BLUE",
            other => return format!("FLAG UNKNOWN ({other})"),
        };
        format!("FLAG {label}")
    }

    unsafe fn draw_extra_widget_panel(
        hdc: HDC,
        area: Area,
        config: &OverlayConfig,
        widget_style: Option<&WidgetStyleConfig>,
    ) {
        let theme_colors = colors(config);
        let inherit_theme = widget_style.is_none_or(|style| style.inherit_theme);
        let show_background = widget_style.is_none_or(|style| style.show_background);
        let show_border = widget_style.is_none_or(|style| style.show_border);
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
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: area.x as f32,
                top: area.y as f32,
                right: area.right() as f32,
                bottom: area.bottom() as f32,
                fill: show_background.then(|| widget_color(background)),
                stroke: show_border.then(|| widget_color(border)),
                stroke_width: widget_style
                    .map_or(config.style.line_thickness, |style| style.border_width)
                    .max(1) as f32,
                radius: widget_style
                    .map_or(config.style.border_radius, |style| style.border_radius)
                    .max(0) as f32,
            });
            return;
        }
        let rect = RECT {
            left: area.x,
            top: area.y,
            right: area.right(),
            bottom: area.bottom(),
        };
        if show_background {
            let brush = CreateSolidBrush(widget_color(background));
            FillRect(hdc, &rect, brush);
            DeleteObject(brush);
        }
        if show_border {
            let width = widget_style
                .map_or(config.style.line_thickness, |style| style.border_width)
                .max(1);
            let pen = CreatePen(PS_SOLID, width, widget_color(border));
            let old_pen = SelectObject(hdc, pen);
            let radius = widget_style
                .map_or(config.style.border_radius, |style| style.border_radius)
                .max(0);
            if radius > 0 {
                RoundRect(
                    hdc,
                    area.x,
                    area.y,
                    area.right(),
                    area.bottom(),
                    radius * 2,
                    radius * 2,
                );
            } else {
                Rectangle(hdc, area.x, area.y, area.right(), area.bottom());
            }
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
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: area.x as f32,
                top: area.y as f32,
                right: (area.x + area.width) as f32,
                bottom: (area.y + area.height) as f32,
                fill: None,
                stroke: Some(widget_color(0x00888888)),
                stroke_width: style.line_width.max(1) as f32,
                radius: 0.0,
            });
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: (area.x + 2) as f32,
                top: (area.y + area.height - filled + 2) as f32,
                right: (area.x + area.width - 2) as f32,
                bottom: (area.y + area.height - 2) as f32,
                fill: Some(widget_color(style.fill)),
                stroke: None,
                stroke_width: 0.0,
                radius: 0.0,
            });
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
                queue_shape(d2d_backend::ShapeCommand::Line {
                    x1: area.x as f32,
                    y1: reference_y as f32,
                    x2: (area.x + area.width) as f32,
                    y2: reference_y as f32,
                    color: widget_color(style.reference),
                    width: style.line_width.max(1) as f32,
                });
            }
            return;
        }
        let outline = CreatePen(PS_SOLID, style.line_width.max(1), widget_color(0x00888888));
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

        let brush = CreateSolidBrush(widget_color(style.fill));
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
            let reference_pen = CreatePen(
                PS_SOLID,
                style.line_width.max(1),
                widget_color(style.reference),
            );
            let old_pen = SelectObject(hdc, reference_pen);
            MoveToEx(hdc, area.x, reference_y, ptr::null_mut());
            LineTo(hdc, area.x + area.width, reference_y);
            SelectObject(hdc, old_pen);
            DeleteObject(reference_pen);
        }
    }

    unsafe fn draw_horizontal_meter(hdc: HDC, area: Area, value: f64, fill: u32, background: u32) {
        let ratio = value.clamp(0.0, 1.0);
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: area.x as f32,
                top: area.y as f32,
                right: area.right() as f32,
                bottom: area.bottom() as f32,
                fill: Some(widget_color(background)),
                stroke: None,
                stroke_width: 0.0,
                radius: (area.height / 2) as f32,
            });
            queue_shape(d2d_backend::ShapeCommand::Rectangle {
                left: area.x as f32,
                top: area.y as f32,
                right: (area.x + (area.width as f64 * ratio) as i32) as f32,
                bottom: area.bottom() as f32,
                fill: Some(widget_color(fill)),
                stroke: None,
                stroke_width: 0.0,
                radius: (area.height / 2) as f32,
            });
            return;
        }
        let background_brush = CreateSolidBrush(widget_color(background));
        let rect = RECT {
            left: area.x,
            top: area.y,
            right: area.right(),
            bottom: area.bottom(),
        };
        FillRect(hdc, &rect, background_brush);
        DeleteObject(background_brush);
        let filled = RECT {
            right: area.x + (area.width as f64 * ratio) as i32,
            ..rect
        };
        let fill_brush = CreateSolidBrush(widget_color(fill));
        FillRect(hdc, &filled, fill_brush);
        DeleteObject(fill_brush);
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn draw_rpm_segments(
        hdc: HDC,
        area: Area,
        rpm: f64,
        max_rpm: f64,
        active_color: u32,
        inactive_color: u32,
        shift_start_percent: u8,
        shift_warning_percent: u8,
        limiter_percent: u8,
        segment_count: u8,
    ) {
        let ratio = if max_rpm > 0.0 {
            (rpm / max_rpm).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let segments = i32::from(segment_count.clamp(4, 20));
        let gap = 2;
        let segment_width = ((area.width - gap * (segments - 1)) / segments).max(2);
        let active = (ratio * segments as f64).ceil() as i32;
        for index in 0..segments {
            let left = area.x + index * (segment_width + gap);
            let right = (left + segment_width).min(area.right());
            let segment_ratio = (index + 1) as f64 / segments as f64;
            let start_ratio = f64::from(shift_start_percent) / 100.0;
            let warning_ratio = f64::from(shift_warning_percent) / 100.0;
            let limiter_ratio = f64::from(limiter_percent) / 100.0;
            let color = if index < active && segment_ratio >= start_ratio {
                if segment_ratio >= limiter_ratio {
                    0x000000FF
                } else if segment_ratio >= warning_ratio {
                    0x0000FFFF
                } else {
                    active_color
                }
            } else {
                inactive_color
            };
            if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
                queue_shape(d2d_backend::ShapeCommand::Rectangle {
                    left: left as f32,
                    top: area.y as f32,
                    right: right as f32,
                    bottom: area.bottom() as f32,
                    fill: Some(widget_color(color)),
                    stroke: None,
                    stroke_width: 0.0,
                    radius: 1.0,
                });
            } else {
                let brush = CreateSolidBrush(widget_color(color));
                let rect = RECT {
                    left,
                    top: area.y,
                    right,
                    bottom: area.bottom(),
                };
                FillRect(hdc, &rect, brush);
                DeleteObject(brush);
            }
        }
    }

    unsafe fn draw_center_bar(hdc: HDC, area: Area, value: f64, color: u32, label_color: u32) {
        let center = area.x + area.width / 2;
        let end = center + (value.clamp(-1.0, 1.0) * (area.width / 2) as f64).round() as i32;
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            queue_shape(d2d_backend::ShapeCommand::Line {
                x1: center as f32,
                y1: area.y as f32,
                x2: end as f32,
                y2: area.y as f32,
                color: widget_color(color),
                width: area.height.max(1) as f32,
            });
            draw_text(hdc, area.x, area.y + 20, label_color, "STEERING");
            return;
        }
        let pen = CreatePen(PS_SOLID, area.height, widget_color(color));
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
            if let Some(message) = input_coaching_message(snapshot.clone()) {
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
                areas.push((WidgetId::Extra(id), area_from_layout(&widget.layout)));
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
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            for (widget, area) in widget_areas(config) {
                if widget_layout(config, widget).locked {
                    continue;
                }
                queue_shape(d2d_backend::ShapeCommand::Rectangle {
                    left: area.x as f32,
                    top: area.y as f32,
                    right: area.right() as f32,
                    bottom: area.bottom() as f32,
                    fill: None,
                    stroke: Some(widget_color(colors.reference)),
                    stroke_width: config.style.line_thickness.max(2) as f32,
                    radius: 0.0,
                });
                if selected == Some(widget) {
                    let right = area.right().saturating_sub(scale_size(config, 6));
                    let bottom = area.bottom().saturating_sub(scale_size(config, 6));
                    let step = scale_size(config, 5);
                    for index in 0..3 {
                        let inset = step * index;
                        queue_shape(d2d_backend::ShapeCommand::Line {
                            x1: (right - scale_size(config, 22) + inset) as f32,
                            y1: bottom as f32,
                            x2: right as f32,
                            y2: (bottom - scale_size(config, 22) + inset) as f32,
                            color: widget_color(colors.reference),
                            width: config.style.line_thickness.max(2) as f32,
                        });
                    }
                }
            }
            return;
        }
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
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            let segment_count = history.len().saturating_sub(1).max(1) as f64;
            let mut previous = None;
            for (index, sample) in history.iter().cloned().enumerate() {
                let Some(before) = previous else {
                    previous = Some(sample);
                    continue;
                };
                let x1 = area.x + (((index - 1) as f64 / segment_count) * area.width as f64) as i32;
                let x2 = area.x + ((index as f64 / segment_count) * area.width as f64) as i32;
                let y1 = area.y + area.height
                    - (value(before).clamp(0.0, 1.0) * area.height as f64) as i32;
                let y2 = area.y + area.height
                    - (value(sample.clone()).clamp(0.0, 1.0) * area.height as f64) as i32;
                queue_shape(d2d_backend::ShapeCommand::Line {
                    x1: x1 as f32,
                    y1: y1 as f32,
                    x2: x2 as f32,
                    y2: y2 as f32,
                    color: widget_color(color),
                    width: line_width.max(1) as f32,
                });
                previous = Some(sample);
            }
            return;
        }
        let pen = CreatePen(PS_SOLID, line_width.max(1), widget_color(color));
        let old_pen = SelectObject(hdc, pen);
        let segment_count = history.len().saturating_sub(1).max(1) as f64;
        let mut previous = None;
        for (index, sample) in history.iter().cloned().enumerate() {
            let Some(before) = previous else {
                previous = Some(sample);
                continue;
            };

            let x1 = area.x + (((index - 1) as f64 / segment_count) * area.width as f64) as i32;
            let x2 = area.x + ((index as f64 / segment_count) * area.width as f64) as i32;
            let y1 =
                area.y + area.height - (value(before).clamp(0.0, 1.0) * area.height as f64) as i32;
            let y2 = area.y + area.height
                - (value(sample.clone()).clamp(0.0, 1.0) * area.height as f64) as i32;
            MoveToEx(hdc, x1, y1, ptr::null_mut());
            LineTo(hdc, x2, y2);
            previous = Some(sample);
        }
        SelectObject(hdc, old_pen);
        DeleteObject(pen);
    }

    unsafe fn draw_text(hdc: HDC, x: i32, y: i32, color: u32, text: &str) {
        if NATIVE_TEXT_ENABLED.with(|state| state.get()) {
            NATIVE_TEXT_COMMANDS.with(|commands| {
                commands.borrow_mut().push(d2d_backend::TextCommand {
                    x,
                    y,
                    color: widget_color(color),
                    text: text.to_string(),
                });
            });
            return;
        }
        let wide: Vec<u16> = text.encode_utf16().collect();
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, widget_color(color));
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

        #[test]
        fn blends_widget_colors_with_configured_opacity() {
            assert_eq!(blend_color(0x00FFFFFF, 0x00000000, 1.0), 0x00FFFFFF);
            assert_eq!(blend_color(0x00FFFFFF, 0x00000000, 0.5), 0x00808080);
            assert_eq!(blend_color(0x00112233, 0x00445566, 0.0), 0x00445566);
        }

        #[test]
        fn converts_configured_display_units_from_normalized_si() {
            let mut config = OverlayConfig::default();
            assert_eq!(display_fuel(10.0, &config), "10.0 L");
            assert_eq!(display_pressure_value(Some(100.0), &config), Some(100.0));
            assert_eq!(display_temperature_value(Some(100.0), &config), Some(100.0));

            config.units.fuel = "gallons".to_string();
            config.units.pressure = "psi".to_string();
            config.units.temperature = "fahrenheit".to_string();
            assert_eq!(display_fuel(10.0, &config), "2.64 gal");
            assert!(
                (display_pressure_value(Some(100.0), &config).unwrap() - 14.5038).abs() < 0.001
            );
            assert_eq!(display_temperature_value(Some(100.0), &config), Some(212.0));
        }

        #[test]
        fn formats_standings_lap_gaps_without_double_signs() {
            let mut car = lmu_telemetry::VehicleScoringSnapshot {
                slot_id: 1,
                driver_name: None,
                vehicle_name: None,
                vehicle_class: None,
                place: None,
                lap_number: 1,
                lap_distance_m: None,
                current_sector: None,
                last_lap_seconds: None,
                best_lap_seconds: None,
                gap_to_next_seconds: None,
                gap_to_leader_seconds: None,
                laps_behind_next: None,
                laps_behind_leader: Some(-1),
                in_pits: false,
                in_garage: false,
                pit_state: None,
                finish_status: None,
                flag: None,
                is_player: false,
                world_position: None,
            };
            assert_eq!(standings_gap(&car), "-1L");
            car.laps_behind_leader = Some(2);
            assert_eq!(standings_gap(&car), "+2L");
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
                field: std::sync::Arc::from(Vec::new()),
                lap_history: std::sync::Arc::from(Vec::new()),
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
                fuel_current_liters: None,
                fuel_capacity_liters: None,
                fuel_last_lap_used: None,
                fuel_average_lap_used: None,
                fuel_estimated_laps_remaining: None,
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
