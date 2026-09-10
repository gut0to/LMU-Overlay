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
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
            InvalidateRect, LineTo, MoveToEx, Rectangle, SelectObject, SetBkMode, SetTextColor,
            TextOutW, HDC, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::SetProcessDpiAwarenessContext,
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, PostQuitMessage,
                RegisterClassW, SetLayeredWindowAttributes, TranslateMessage, CS_HREDRAW,
                CS_VREDRAW, CW_USEDEFAULT, HWND_TOPMOST, LWA_COLORKEY, MSG, SWP_NOACTIVATE,
                SW_SHOW, WM_DESTROY, WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
                WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    };

    use super::{config::parse_color, OverlayConfig, OverlayError};

    const CLASS_NAME: &[u16] = &[
        'H' as u16, 'a' as u16, 's' as u16, 'h' as u16, 'O' as u16, 'v' as u16, 'e' as u16,
        'r' as u16, 'l' as u16, 'a' as u16, 'y' as u16, 0,
    ];
    const COLOR_KEY: u32 = 0x000000;

    #[derive(Clone, Copy)]
    struct Area {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    #[derive(Clone)]
    struct SharedState {
        latest: Arc<Mutex<Option<TelemetrySnapshot>>>,
        history: Arc<Mutex<RingBuffer<TelemetrySnapshot>>>,
        running: Arc<AtomicBool>,
        config: Arc<OverlayConfig>,
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
                    }
                    last_sample = Instant::now();
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

            SetLayeredWindowAttributes(hwnd, COLOR_KEY, opacity, LWA_COLORKEY);
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            );
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, SW_SHOW);

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
            WM_DESTROY => {
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

        EndPaint(hwnd, &paint);
    }

    unsafe fn draw_panel(hdc: HDC, config: &OverlayConfig) {
        let colors = colors(config);
        let bg = CreateSolidBrush(colors.background);
        let border = CreatePen(PS_SOLID, 1, colors.border);
        let old_brush = SelectObject(hdc, bg);
        let old_pen = SelectObject(hdc, border);
        Rectangle(hdc, 0, 0, config.window.width, config.window.height);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(border);
        DeleteObject(bg);
        if config.widgets.title {
            draw_text(hdc, 18, 12, colors.primary_text, "HashOverlay LMU");
        }
    }

    unsafe fn draw_snapshot(hdc: HDC, snapshot: TelemetrySnapshot, config: &OverlayConfig) {
        let colors = colors(config);
        if config.widgets.speed_gear_rpm {
            draw_text(
                hdc,
                18,
                34,
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
                    x: 22,
                    y: 72,
                    width: 34,
                    height: 92,
                },
                snapshot.throttle,
                colors.throttle,
                "THR",
                colors.secondary_text,
            );
            draw_bar(
                hdc,
                Area {
                    x: 70,
                    y: 72,
                    width: 34,
                    height: 92,
                },
                snapshot.brake,
                colors.brake,
                "BRK",
                colors.secondary_text,
            );
            draw_bar(
                hdc,
                Area {
                    x: 118,
                    y: 72,
                    width: 34,
                    height: 92,
                },
                snapshot.clutch,
                colors.clutch,
                "CLT",
                colors.secondary_text,
            );
        }
        if config.widgets.steering {
            draw_center_bar(
                hdc,
                Area {
                    x: 180,
                    y: 86,
                    width: 190,
                    height: 18,
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
                    180,
                    112,
                    colors.secondary_text,
                    &format!("lap {:.1}%", progress * 100.0),
                );
            }
            draw_text(
                hdc,
                180,
                136,
                colors.secondary_text,
                &format!("lap {} sector {}", snapshot.lap_number, snapshot.sector),
            );
        }
    }

    unsafe fn draw_bar(
        hdc: HDC,
        area: Area,
        value: f64,
        color: u32,
        label: &str,
        label_color: u32,
    ) {
        let clamped = value.clamp(0.0, 1.0);
        let filled = (area.height as f64 * clamped).round() as i32;
        let outline = CreatePen(PS_SOLID, 1, 0x00888888);
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

        let brush = CreateSolidBrush(color);
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
            label_color,
            label,
        );
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
        let origin_x = 180;
        let origin_y = 72;
        let width = 198;
        let height = 34;
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
            |s| s.throttle,
        );
        draw_series(
            hdc,
            history,
            Area {
                x: origin_x,
                y: origin_y + 42,
                width,
                height,
            },
            colors.brake,
            |s| s.brake,
        );
    }

    unsafe fn draw_series(
        hdc: HDC,
        history: &RingBuffer<TelemetrySnapshot>,
        area: Area,
        color: u32,
        value: impl Fn(TelemetrySnapshot) -> f64,
    ) {
        let pen = CreatePen(PS_SOLID, 2, color);
        let old_pen = SelectObject(hdc, pen);
        let samples: Vec<_> = history.iter().copied().collect();
        for (index, pair) in samples.windows(2).enumerate() {
            let x1 =
                area.x + ((index as f64 / samples.len().max(1) as f64) * area.width as f64) as i32;
            let x2 = area.x
                + (((index + 1) as f64 / samples.len().max(1) as f64) * area.width as f64) as i32;
            let y1 =
                area.y + area.height - (value(pair[0]).clamp(0.0, 1.0) * area.height as f64) as i32;
            let y2 =
                area.y + area.height - (value(pair[1]).clamp(0.0, 1.0) * area.height as f64) as i32;
            MoveToEx(hdc, x1, y1, ptr::null_mut());
            LineTo(hdc, x2, y2);
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

    struct Colors {
        background: u32,
        border: u32,
        primary_text: u32,
        secondary_text: u32,
        throttle: u32,
        brake: u32,
        clutch: u32,
        steering: u32,
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
