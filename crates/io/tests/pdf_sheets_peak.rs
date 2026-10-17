//! Fork-only measurement for #1068: peak RSS of a many-sheet PDF plot. Ignored by default;
//! CI runs it with `--ignored --nocapture` on this branch and on its base to compare.

use cadcraft_doc::{Common, Drawing, EntityKind, Layout, Line, Space, Viewport};
use cadcraft_geom::{Vec2, Vec3};
use cadcraft_io::PdfOptions;
use cadcraft_io::pdf::pdf_sheets;

fn hwm_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0)
}

#[test]
#[ignore]
fn peak_many_sheets() {
    let sheets: usize = std::env::var("PEAK_SHEETS").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let lines: usize = std::env::var("PEAK_LINES").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let mut d = Drawing::new_metric();
    for i in 0..lines {
        let (x, y) = ((i % 1000) as f64, (i / 1000) as f64);
        let l = Line { a: Vec3::new(x, y, 0.0), b: Vec3::new(x + 0.8, y + 0.6, 0.0) };
        d.add(&Space::Model, Common::default(), EntityKind::Line(l)).unwrap();
    }
    let h = (lines / 1000).max(1) as f64;
    let mut spaces = Vec::new();
    for s in 0..sheets {
        let name = format!("Sheet{s}");
        d.layouts.push(Layout::new(&name, s as u32 + 1));
        let paper = Space::Paper(name);
        let vp = Viewport {
            center: Vec3::new(150.0, 100.0, 0.0),
            width: 280.0,
            height: 190.0,
            view_center: Vec2::new(500.0, h / 2.0),
            view_height: h.max(700.0),
            id: 2,
            locked: false,
            frozen_layers: Vec::new(),
            layer_colors: Vec::new(),
            clip: None,
            on: true,
        };
        d.add(&paper, Common::default(), EntityKind::Viewport(vp)).unwrap();
        spaces.push(paper);
    }
    let before = hwm_kb();
    let bytes = pdf_sheets(&d, &spaces, &PdfOptions { compress: true, ..PdfOptions::default() }).unwrap();
    let after = hwm_kb();
    println!("PEAK sheets={sheets} lines={lines} pdf_bytes={} hwm_before_kb={before} hwm_after_kb={after} delta_kb={}", bytes.len(), after.saturating_sub(before));
}
