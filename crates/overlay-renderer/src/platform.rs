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
        mem::zeroed,
        ptr,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread,
        time::{Duration, Instant},
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
            Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, PostQuitMessage,
                RegisterClassW, SetLayeredWindowAttributes, ShowWindow, TranslateMessage,
                CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWL_EXSTYLE, HTBOTTOM, HTBOTTOMRIGHT,
                HTCAPTION, HTRIGHT, HWND_TOPMOST, LWA_ALPHA, LWA_COLORKEY, MSG, SWP_NOACTIVATE,
                SW_HIDE, SW_SHOW, WM_DESTROY, WM_HOTKEY, WM_NCHITTEST, WM_PAINT, WNDCLASSW,
                WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
                WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    };

    use super::{config::parse_color, OverlayConfig, OverlayError};

    const CLASS_NAME: &[u16] = &[
        'H' as u16, 'a' as u16, 's' as u16, 'h' as u16, 'O' as u16, 'v' as u16, 'e' as u16,
        'r' as u16, 'l' as u16, 'a' as u16, 'y' as u16, 0,
    ];
    const COLOR_KEY: u32 = 0x000000;
    const HOTKEY_TOGGLE_OVERLAY: i32 = 1;
    const HOTKEY_EDIT_MODE: i32 = 2;
    const EDIT_HIT_MARGIN: i32 = 16;

    #[derive(Clone, Copy)]
    struct Area {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
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
        config: Arc<OverlayConfig>,
    }

    #[derive(Debug, Default)]
    struct PerfStats {
        telemetry_samples: u64,
        render_frames: u64,
        telemetry_hz: u64,
        render_fps: u64,
    }

    impl PerfStats {
        fn refresh(&mut self) {
            self.telemetry_hz = self.telemetry_samples;
            self.render_fps = self.render_frames;
            self.telemetry_samples = 0;
            self.render_frames = 0;
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
            let history_samples = config.window.history_samples;
            Ok(Self {
                state: SharedState {
                    latest: Arc::new(Mutex::new(None)),
                    history: Arc::new(Mutex::new(RingBuffer::new(history_samples))),
                    running: Arc::new(AtomicBool::new(true)),
                    visible: Arc::new(AtomicBool::new(true)),
                    edit_mode: Arc::new(AtomicBool::new(false)),
                    stats: Arc::new(Mutex::new(PerfStats::default())),
                    config: Arc::new(config),
                },
            })
        }

        pub fn run<F>(self, mut next_snapshot: F) -> Result<(), OverlayError>
        where
            F: FnMut() -> Option<TelemetrySnapshot>,
        {
            let hwnd = create_window(self.state.clone())?;
            let repaint_running = self.state.running.clone();
            let repaint_hz = self.state.config.window.refresh_hz;
            let repaint_hwnd = hwnd as isize;

            thread::spawn(move || {
                let hwnd = repaint_hwnd as HWND;
                while repaint_running.load(Ordering::Relaxed) {
                    unsafe {
                        InvalidateRect(hwnd, ptr::null(), 0);
                    }
                    let refresh_ms = 1000 / repaint_hz;
                    thread::sleep(Duration::from_millis(refresh_ms.max(1)));
                }
            });

            let mut last_sample = Instant::now();
            let mut last_stats = Instant::now();
            let mut message: MSG = unsafe { zeroed() };
            let sample_interval = Duration::from_millis(self.state.config.window.sample_ms);

            loop {
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
                            return Ok(());
                        }
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }

                if last_sample.elapsed() >= sample_interval {
                    if let Some(snapshot) = next_snapshot() {
                        if let Ok(mut latest) = self.state.latest.lock() {
                            *latest = Some(snapshot);
                        }
                        if let Ok(mut history) = self.state.history.lock() {
                            history.push(snapshot);
                        }
                        if let Ok(mut stats) = self.state.stats.lock() {
                            stats.telemetry_samples += 1;
                        }
                    }
                    last_sample = Instant::now();
                }

                if last_stats.elapsed() >= Duration::from_secs(1) {
                    if let Ok(mut stats) = self.state.stats.lock() {
                        stats.refresh();
                    }
                    last_stats = Instant::now();
                }

                thread::sleep(Duration::from_millis(1));
            }
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

            let x = state.config.window.x;
            let y = state.config.window.y;
            let width = state.config.window.width;
            let height = state.config.window.height;
            let opacity = state.config.style.opacity;
            let toggle_hotkey = virtual_key(&state.config.hotkeys.toggle_overlay);
            let edit_hotkey = virtual_key(&state.config.hotkeys.edit_mode);
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
            if let Some(key) = toggle_hotkey {
                RegisterHotKey(hwnd, HOTKEY_TOGGLE_OVERLAY, 0, key);
            }
            if let Some(key) = edit_hotkey {
                RegisterHotKey(hwnd, HOTKEY_EDIT_MODE, 0, key);
            }

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
            WM_NCHITTEST => edit_mode_hit_test(hwnd, lparam)
                .unwrap_or_else(|| DefWindowProcW(hwnd, message, wparam, lparam)),
            WM_DESTROY => {
                UnregisterHotKey(hwnd, HOTKEY_TOGGLE_OVERLAY);
                UnregisterHotKey(hwnd, HOTKEY_EDIT_MODE);
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
            _ => {}
        }
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
        if let Ok(mut stats) = state.stats.lock() {
            stats.render_frames += 1;
        }
        let latest = state.latest.lock().ok().and_then(|value| *value);

        draw_panel(hdc, state.config.as_ref());

        if let Some(snapshot) = latest {
            draw_snapshot(hdc, snapshot, state.config.as_ref());
            if state.config.widgets.input_history {
                if let Ok(history) = state.history.lock() {
                    draw_history(hdc, &history, state.config.as_ref());
                }
            }
        } else {
            draw_text(
                hdc,
                22,
                24,
                colors(&state.config).primary_text,
                "Waiting for LMU telemetry...",
            );
        }

        if state.config.widgets.performance_monitor {
            draw_performance_monitor(hdc, state);
        }

        if state.edit_mode.load(Ordering::Relaxed) {
            draw_edit_handles(hdc, state.config.as_ref());
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
        if config.widgets.title {
            draw_text(
                hdc,
                scale_px(config, 18),
                scale_px(config, 12),
                colors.primary_text,
                "HashOverlay LMU",
            );
        }
    }

    unsafe fn draw_snapshot(hdc: HDC, snapshot: TelemetrySnapshot, config: &OverlayConfig) {
        let colors = colors(config);
        if config.widgets.speed_gear_rpm {
            draw_text(
                hdc,
                scale_px(config, 18),
                scale_px(config, 34),
                colors.primary_text,
                &format!(
                    "{:.0} km/h   gear {}   {:.0} rpm",
                    snapshot.speed_kph, snapshot.gear, snapshot.rpm
                ),
            );
        }
        if config.widgets.pedals {
            draw_bar(
                hdc,
                Area {
                    x: scale_px(config, 22),
                    y: scale_px(config, 72),
                    width: scale_size(config, 34),
                    height: scale_size(config, 92),
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
                    x: scale_px(config, 70),
                    y: scale_px(config, 72),
                    width: scale_size(config, 34),
                    height: scale_size(config, 92),
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
                    x: scale_px(config, 118),
                    y: scale_px(config, 72),
                    width: scale_size(config, 34),
                    height: scale_size(config, 92),
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
                    x: scale_px(config, 180),
                    y: scale_px(config, 86),
                    width: scale_size(config, 190),
                    height: scale_size(config, 18),
                },
                snapshot.steering,
                colors.steering,
                colors.secondary_text,
            );
        }

        if config.widgets.lap_info {
            if let Some(progress) = snapshot.lap_progress {
                draw_text(
                    hdc,
                    scale_px(config, 180),
                    scale_px(config, 112),
                    colors.secondary_text,
                    &format!("lap {:.1}%", progress * 100.0),
                );
            }
            draw_text(
                hdc,
                scale_px(config, 180),
                scale_px(config, 136),
                colors.secondary_text,
                &format!("lap {} sector {}", snapshot.lap_number, snapshot.sector),
            );
        }

        if config.widgets.delta_timing {
            draw_delta_widget(hdc, snapshot, config);
        }

        if config.widgets.coaching {
            draw_coaching_widget(hdc, snapshot, config);
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
        let origin_x = scale_px(config, 180);
        let origin_y = scale_px(config, 72);
        let width = scale_size(config, 198);
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

    unsafe fn draw_delta_widget(hdc: HDC, snapshot: TelemetrySnapshot, config: &OverlayConfig) {
        let colors = colors(config);
        let x = scale_px(config, 18);
        let y = config
            .window
            .height
            .saturating_sub(scale_size(config, 52))
            .max(scale_size(config, 138));
        if let Some(delta) = snapshot.delta_seconds {
            let color = if delta <= 0.0 {
                colors.delta_gain
            } else {
                colors.delta_loss
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
                scale_px(config, 180),
                y,
                colors.secondary_text,
                &format!("PB {}", lap_time(best)),
            );
        }

        if let Some(best) = snapshot.session_best_seconds {
            draw_text(
                hdc,
                scale_px(config, 180),
                y + scale_size(config, 20),
                colors.secondary_text,
                &format!("SB {}", lap_time(best)),
            );
        }

        if let Some(mini_sector) = snapshot.mini_sector_index {
            draw_text(
                hdc,
                scale_px(config, 320),
                y,
                colors.secondary_text,
                &format!("MS {}", mini_sector + 1),
            );
        }
    }

    unsafe fn draw_coaching_widget(hdc: HDC, snapshot: TelemetrySnapshot, config: &OverlayConfig) {
        let colors = colors(config);
        let mut y = scale_px(config, 52);
        if let Some(hint) = snapshot.brake_hint_meters {
            draw_text(
                hdc,
                scale_px(config, 300),
                y,
                timing_color(hint, colors),
                &format!("BRK {}", brake_timing_hint(hint)),
            );
            y += scale_size(config, 18);
        }
        if let Some(hint) = snapshot.throttle_hint_meters {
            draw_text(
                hdc,
                scale_px(config, 300),
                y,
                timing_color(hint, colors),
                &format!("THR {}", throttle_timing_hint(hint)),
            );
            y += scale_size(config, 18);
        }
        if let Some(message) = input_coaching_message(snapshot) {
            draw_text(hdc, scale_px(config, 300), y, colors.reference, message);
        }
    }

    unsafe fn draw_performance_monitor(hdc: HDC, state: &SharedState) {
        let colors = colors(&state.config);
        let ring_usage = state
            .history
            .lock()
            .ok()
            .map(|history| format!("{}/{}", history.len(), history.capacity()))
            .unwrap_or_else(|| "--".to_string());
        if let Ok(stats) = state.stats.lock() {
            draw_text(
                hdc,
                18,
                state.config.window.height.saturating_sub(18),
                colors.secondary_text,
                &format!(
                    "telemetry {} Hz  render {} FPS  ring {}",
                    stats.telemetry_hz, stats.render_fps, ring_usage
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

    unsafe fn draw_edit_handles(hdc: HDC, config: &OverlayConfig) {
        let colors = colors(config);
        let pen = CreatePen(
            PS_SOLID,
            config.style.line_thickness.max(2),
            colors.reference,
        );
        let old_pen = SelectObject(hdc, pen);
        let right = config.window.width.saturating_sub(scale_size(config, 8));
        let bottom = config.window.height.saturating_sub(scale_size(config, 8));
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
        reference: u32,
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
            reference: parse_color(&config.style.reference, 0x00AAAAAA),
        }
    }

    fn signed_time(seconds: f64) -> String {
        format!("{seconds:+.3}")
    }

    fn lap_time(seconds: f64) -> String {
        let minutes = (seconds / 60.0).floor() as u32;
        let seconds = seconds - f64::from(minutes) * 60.0;
        format!("{minutes}:{seconds:06.3}")
    }

    fn brake_timing_hint(meters: f64) -> String {
        if meters >= 0.0 {
            format!("later +{meters:.0}m")
        } else {
            format!("earlier {meters:.0}m")
        }
    }

    fn throttle_timing_hint(meters: f64) -> String {
        if meters >= 0.0 {
            format!("later +{meters:.0}m")
        } else {
            format!("earlier {meters:.0}m")
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
            assert_eq!(brake_timing_hint(25.0), "later +25m");
            assert_eq!(brake_timing_hint(-12.0), "earlier -12m");
            assert_eq!(throttle_timing_hint(15.0), "later +15m");
            assert_eq!(throttle_timing_hint(-8.0), "earlier -8m");
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
                session_kind: SessionKind::Practice,
                game_phase: GamePhase::GreenFlag,
                in_pits: false,
                in_garage: false,
                delta_seconds: None,
                predicted_lap_seconds: None,
                session_best_seconds: None,
                personal_best_seconds: None,
                reference_lap_seconds: None,
                mini_sector_index: None,
                mini_sector_delta_seconds: None,
                brake_hint_meters: None,
                throttle_hint_meters: None,
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

    pub fn run<F>(self, _next_snapshot: F) -> Result<(), OverlayError>
    where
        F: FnMut() -> Option<TelemetrySnapshot>,
    {
        Err(OverlayError::UnsupportedPlatform)
    }
}
