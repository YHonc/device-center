//! Physical-pixel tray popup placement. The caller supplies the target monitor DPI.
pub const WIDTH: f64 = 320.;
pub const HEIGHT: f64 = 320.;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn place(anchor: Rect, work: Rect, size: (u32, u32), gap: i32) -> (i32, i32) {
    let right = work.x + work.width as i32;
    let bottom = work.y + work.height as i32;
    let anchor_right = anchor.x + anchor.width as i32;
    let anchor_bottom = anchor.y + anchor.height as i32;
    let (width, height) = (size.0 as i32, size.1 as i32);
    // Taskbars can occupy any monitor edge. A tray overflow icon inside the work
    // area prefers upward opening, keeping the popup close to its actual icon.
    let (x, y) = if anchor.y >= bottom {
        (anchor_right - width, bottom - height - gap)
    } else if anchor_bottom <= work.y {
        (anchor_right - width, work.y + gap)
    } else if anchor_right <= work.x {
        (work.x + gap, anchor_bottom - height)
    } else if anchor.x >= right {
        (right - width - gap, anchor_bottom - height)
    } else {
        let above = anchor.y - height - gap;
        let y = if above >= work.y { above } else { anchor_bottom + gap };
        (anchor_right - width, y)
    };
    (
        x.clamp(work.x, (right - width).max(work.x)),
        y.clamp(work.y, (bottom - height).max(work.y)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect { Rect { x, y, width, height } }
    #[test]
    fn bottom_and_top_taskbars_stay_in_work_area() {
        assert_eq!(place(rect(1800,1040,24,40),rect(0,0,1920,1040),(320,320),8),(1504,712));
        assert_eq!(place(rect(1800,0,24,40),rect(0,40,1920,1040),(320,320),8),(1504,48));
    }
    #[test]
    fn side_taskbars_open_towards_content() {
        assert_eq!(place(rect(0,1000,40,24),rect(40,0,1880,1080),(320,320),8),(48,704));
        assert_eq!(place(rect(1880,1000,40,24),rect(0,0,1880,1080),(320,320),8),(1552,704));
    }
    #[test]
    fn negative_monitor_and_scaled_window_use_physical_pixels() {
        assert_eq!(place(rect(-100,1560,36,60),rect(-2880,0,2880,1560),(480,480),12),(-544,1068));
    }
    #[test]
    fn all_edges_clamp_and_small_work_area_does_not_panic() {
        assert_eq!(place(rect(10,10,24,24),rect(0,0,1920,1040),(320,320),8),(0,42));
        assert_eq!(place(rect(1900,1020,24,24),rect(0,0,1920,1040),(320,320),8),(1600,692));
        assert_eq!(place(rect(0,100,24,24),rect(0,0,200,100),(320,320),8),(0,0));
    }
}
