//! How long a large drawing takes to open as DWG and to plot its sheets, and how much memory
//! it needs (Linux). Ignored: run with
//! `cargo test -p cadcraft-io --release --test perf_large_drawing -- --ignored --nocapture`;
//! `CADCRAFT_PERF_ENTITIES` sets the size (default 400 000 model entities, a DWG of about 16 MB)
//! and `CADCRAFT_PERF_LAYERS` the number of layers (default 4; a drawing with bound xrefs has thousands).
//! Measured on 02/11/2026 (dev profile, 400 000 entities): open 8 s and 1.3 GB peak, each sheet
//! under a second, the model page 9 s plus its compression.

use std::time::Instant;

use cadcraft_doc::*;
use cadcraft_geom::{Vec2, Vec3};

/// Resident and peak memory of this process (Linux), for the report.
fn mem() -> String {
    #[cfg(target_os = "linux")]
    {
        let kb = |key: &str| {
            std::fs::read_to_string("/proc/self/status")
                .ok()
                .and_then(|s| s.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse::<u64>().ok()))
                .unwrap_or(0)
                / 1024
        };
        format!("rss {} MB, peak {} MB", kb("VmRSS"), kb("VmHWM"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        String::new()
    }
}

/// A metric drawing of `n` model entities (lines, texts and circles on four layers, spread
/// over 3000 x 2000) and two A4 layouts with one and two viewports onto it. The same every run.
fn synthetic(n: usize, layers: usize) -> Drawing {
    let mut d = Drawing::new_metric();
    for name in ["WALLS", "TEXT", "DIMS"] {
        d.layers.push(Layer { name: name.into(), ..Layer::default() });
    }
    // Many layers, as a drawing with bound xrefs has: entities spread over them.
    let names: Vec<String> = (4..=layers).map(|i| format!("XREF-{i}|A-WALL-{}", i % 97)).collect();
    for name in &names {
        d.layers.push(Layer { name: name.clone(), ..Layer::default() });
    }
    let mut seed: u64 = 7;
    let mut rnd = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f64) / ((1u64 << 31) as f64)
    };
    let base = ["0", "WALLS", "TEXT", "DIMS"];
    for i in 0..n {
        let (x, y) = (rnd() * 3000.0, rnd() * 2000.0);
        let layer = if names.is_empty() || i % 2 == 0 { base[i % 4].to_string() } else { names[i % names.len()].clone() };
        let common = Common { layer, ..Common::default() };
        let kind = match i % 10 {
            0 | 1 => EntityKind::Text(Text {
                insert: Vec3::new(x, y, 0.0),
                align_pt: None,
                height: 2.5,
                value: format!("ROOM {i} AREA {}.5 m2", i % 97),
                rotation: 0.0,
                width_factor: 1.0,
                oblique: 0.0,
                style: "Standard".into(),
                halign: HAlign::Left,
                valign: VAlign::Baseline,
            }),
            2 => EntityKind::Circle(Circle { center: Vec3::new(x, y, 0.0), radius: 0.5 + rnd() * 7.5 }),
            _ => EntityKind::Line(Line { a: Vec3::new(x, y, 0.0), b: Vec3::new(x + rnd() * 80.0 - 40.0, y + rnd() * 80.0 - 40.0, 0.0) }),
        };
        d.add(&Space::Model, common, kind).unwrap();
    }
    let sheets: [(&str, &[[f64; 4]]); 2] =
        [("SHEET1", &[[10.0, 10.0, 280.0, 190.0]]), ("SHEET2", &[[10.0, 10.0, 140.0, 190.0], [150.0, 10.0, 280.0, 190.0]])];
    let mut id = 2;
    for (name, rects) in sheets {
        let tab = d.layouts.len() as u32 + 1;
        let mut l = Layout::new(name, tab);
        l.page.width_mm = 210.0;
        l.page.height_mm = 297.0;
        l.page.margins_mm = [5.0; 4];
        l.page.landscape = true;
        d.layouts.push(l);
        let sheet = Space::Paper(name.into());
        for r in rects {
            let vp = Viewport {
                center: Vec3::new((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0, 0.0),
                width: r[2] - r[0],
                height: r[3] - r[1],
                view_center: Vec2::new(1500.0, 1000.0),
                view_height: 2000.0,
                id,
                ..Viewport::default()
            };
            id += 1;
            d.add(&sheet, Common::default(), EntityKind::Viewport(vp)).unwrap();
        }
    }
    d
}

#[test]
#[ignore]
fn open_and_plot_a_large_dwg() {
    let n: usize = std::env::var("CADCRAFT_PERF_ENTITIES").ok().and_then(|v| v.parse().ok()).unwrap_or(400_000);
    let layers: usize = std::env::var("CADCRAFT_PERF_LAYERS").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let t = Instant::now();
    let d = synthetic(n, layers);
    eprintln!("synthetic drawing, {n} entities on {} layers: {:.1}s, {}", d.layers.len(), t.elapsed().as_secs_f64(), mem());
    let t = Instant::now();
    let dwg = cadcraft_io::write(&d, "perf.dwg").unwrap();
    eprintln!("write DWG: {:.1}s, {} MB, {}", t.elapsed().as_secs_f64(), dwg.len() / 1048576, mem());
    drop(d);
    let t = Instant::now();
    let back = cadcraft_io::read(&dwg, "perf.dwg").unwrap();
    eprintln!("open DWG: {:.1}s, {} entities, {}", t.elapsed().as_secs_f64(), back.model.len(), mem());
    for name in ["SHEET1", "SHEET2"] {
        let t = Instant::now();
        let list = cadcraft_render::build_plot(&back, &Space::Paper(name.into()), &cadcraft_render::Options::default());
        eprintln!("  display list of {name}: {:.1}s, {} prims", t.elapsed().as_secs_f64(), list.prims.len());
        drop(list);
        let t = Instant::now();
        let pdf = cadcraft_io::plot(&back, &Space::Paper(name.into()), &serde_json::json!({})).unwrap();
        eprintln!("plot {name}: {:.1}s, {} KB, {}", t.elapsed().as_secs_f64(), pdf.len() / 1024, mem());
        let t = Instant::now();
        let pdf = cadcraft_io::plot(&back, &Space::Paper(name.into()), &serde_json::json!({"layers": false})).unwrap();
        eprintln!("  plot {name} without PDF layers: {:.1}s, {} KB", t.elapsed().as_secs_f64(), pdf.len() / 1024);
        let t = Instant::now();
        let pdf = cadcraft_io::plot(&back, &Space::Paper(name.into()), &serde_json::json!({"compress": false})).unwrap();
        eprintln!("  plot {name} uncompressed: {:.1}s, {} KB", t.elapsed().as_secs_f64(), pdf.len() / 1024);
    }
    let t = Instant::now();
    let pdf = cadcraft_io::plot(&back, &Space::Model, &serde_json::json!({})).unwrap();
    eprintln!("plot Model: {:.1}s, {} KB, {}", t.elapsed().as_secs_f64(), pdf.len() / 1024, mem());
}
