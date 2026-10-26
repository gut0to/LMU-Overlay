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
