use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DisplayRect { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }
impl DisplayRect {
    fn from_edges(edges: [i32; 4]) -> Option<Self> {
        let [x, y, right, bottom] = edges;
        let width = right.checked_sub(x)?;
        let height = bottom.checked_sub(y)?;
        (width > 0 && height > 0).then_some(Self { x, y, width, height })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Display {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub bounds: DisplayRect,
    pub work_area: DisplayRect,
}

fn display_from_edges(id: String, monitor: [i32; 4], work: [i32; 4], primary: bool) -> Option<Display> {
    let bounds = DisplayRect::from_edges(monitor)?;
    let work_area = DisplayRect::from_edges(work).unwrap_or_else(|| bounds.clone());
    Some(Display { name: id.trim_start_matches(r"\\.\").to_string(), id, primary, bounds, work_area })
}

#[cfg(windows)]
pub fn list() -> Result<Vec<Display>, String> {
    use windows_sys::Win32::{Foundation::{BOOL, LPARAM, RECT}, Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, MONITORINFOF_PRIMARY}};
    unsafe extern "system" fn collect(monitor: HMONITOR, _dc: HDC, _clip: *mut RECT, data: LPARAM) -> BOOL {
        let displays = &mut *(data as *mut Vec<Display>);
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>()) == 0 { return 1; }
        let end = info.szDevice.iter().position(|value| *value == 0).unwrap_or(info.szDevice.len());
        let id = String::from_utf16_lossy(&info.szDevice[..end]);
        let rect = |value: RECT| [value.left, value.top, value.right, value.bottom];
        if let Some(display) = display_from_edges(id, rect(info.monitorInfo.rcMonitor), rect(info.monitorInfo.rcWork), info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0) { displays.push(display); }
        1
    }
    let mut displays = Vec::new();
    if unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(collect), &mut displays as *mut Vec<Display> as LPARAM) } == 0 { return Err(format!("Could not enumerate Windows displays: {}", std::io::Error::last_os_error())); }
    displays.sort_by(|left, right| right.primary.cmp(&left.primary).then(left.id.cmp(&right.id)));
    Ok(displays)
}

#[cfg(not(windows))]
pub fn list() -> Result<Vec<Display>, String> { Err("Display discovery requires Windows".into()) }
