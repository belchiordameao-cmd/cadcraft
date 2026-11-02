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

/// A DXF with the shape of a drawing imported from DGN: `n` DGN line-style components (`LS*`
/// objects of about 90 groups each, a nineteenth of them stroke patterns) listed by one
/// dictionary, 2400 definitions, 2400 linetypes, 3200 blocks of lines and 20 000 lines. A real
/// 31 MB DWG had 288 000 of them, 26 million of its 29 million groups.
fn dgn_profile_dxf(n: usize) -> String {
    let strokes = n / 19;
    let internal = n - strokes;
    let mut s = String::with_capacity(n * 800);
    s += "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1027\n9\n$HANDSEED\n5\nFFFFFFF\n9\n$MEASUREMENT\n70\n1\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLTYPE\n5\n5\n330\n0\n100\nAcDbSymbolTable\n70\n2403\n";
    for (name, h) in [("ByBlock", "14"), ("ByLayer", "15"), ("Continuous", "16")] {
        s += &format!(
            "0\nLTYPE\n5\n{h}\n330\n5\n100\nAcDbSymbolTableRecord\n100\nAcDbLinetypeTableRecord\n2\n{name}\n70\n0\n3\n\n72\n65\n73\n0\n40\n0.0\n"
        );
    }
    for i in 0..2400 {
        s += &format!(
            "0\nLTYPE\n5\n{:X}\n330\n5\n100\nAcDbSymbolTableRecord\n100\nAcDbLinetypeTableRecord\n2\nDGN_{i}\n70\n0\n3\n\n72\n65\n73\n2\n40\n1.5\n49\n1.0\n74\n0\n49\n-0.5\n74\n0\n",
            0x100 + i
        );
    }
    s += "0\nENDTAB\n0\nTABLE\n2\nLAYER\n5\n2\n330\n0\n100\nAcDbSymbolTable\n70\n1\n0\nLAYER\n5\n10\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n0\n70\n0\n62\n7\n6\nContinuous\n0\nENDTAB\n";
    s += "0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n330\n0\n100\nAcDbSymbolTable\n70\n3202\n0\nBLOCK_RECORD\n5\n1F\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Model_Space\n0\nBLOCK_RECORD\n5\n1B\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Paper_Space\n";
    for i in 0..3200 {
        s += &format!("0\nBLOCK_RECORD\n5\n{:X}\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\nB{i}\n", 0x1000 + i);
    }
    s += "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n";
    let mut h = 0x10000u64;
    let mut next = || {
        h += 1;
        format!("{h:X}")
    };
    for i in 0..3200 {
        let br = format!("{:X}", 0x1000 + i);
        s += &format!(
            "0\nBLOCK\n5\n{}\n330\n{br}\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\nB{i}\n70\n0\n10\n0.0\n20\n0.0\n30\n0.0\n3\nB{i}\n1\n\n",
            next()
        );
        for k in 0..6 {
            s += &format!(
                "0\nLINE\n5\n{}\n330\n{br}\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n{k}.0\n20\n0.0\n30\n0.0\n11\n{k}.0\n21\n1.0\n31\n0.0\n",
                next()
            );
        }
        s += &format!("0\nENDBLK\n5\n{}\n330\n{br}\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n", next());
    }
    s += "0\nENDSEC\n0\nSECTION\n2\nENTITIES\n";
    for i in 0..20_000u64 {
        let (x, y) = ((i * 7919 % 5000) as f64, (i * 104729 % 3000) as f64);
        s += &format!(
            "0\nLINE\n5\n{}\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n{x}\n20\n{y}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0\n",
            next(),
            x + 10.0,
            y + 5.0
        );
    }
    s += "0\nENDSEC\n0\nSECTION\n2\nOBJECTS\n";
    s += "0\nDICTIONARY\n5\nC\n330\n0\n100\nAcDbDictionary\n281\n1\n3\nACAD_DGNLINESTYLECOMP\n350\nD0\n3\nACAD_GROUP\n350\nD\n";
    s += "0\nDICTIONARY\n5\nD\n330\nC\n100\nAcDbDictionary\n281\n1\n";
    s += "0\nDICTIONARY\n5\nD0\n330\nC\n100\nAcDbDictionary\n280\n1\n281\n1\n";
    let def = |i: usize| format!("{:X}", 0x2000_0000 + i);
    let comp = |i: usize| format!("{:X}", 0x3000_0000 + i);
    for i in 0..2400 {
        s += &format!("3\nDGN_{i}\n360\n{}\n", def(i));
    }
    for i in 0..n {
        s += &format!("3\n{{{i:08X}-0000-4000-8000-{i:012X}}}\n360\n{}\n", comp(i));
    }
    for i in 0..2400 {
        s += &format!(
            "0\nLSDEFINITION\n5\n{}\n102\n{{ACAD_REACTORS\n330\nD0\n102\n}}\n330\nD0\n100\nAcDbLSDefinition\n1\nDGN_{i}\n90\n2\n91\n{i}\n310\n{:032X}\n290\n0\n40\n1.0\n41\n1.0\n92\n0\n291\n0\n292\n1\n293\n0\n294\n1\n340\n{}\n",
            def(i),
            i,
            comp(i % n.max(1))
        );
    }
    let pattern = |strokes: usize| {
        let mut p = format!("290\n0\n291\n0\n92\n0\n41\n0.0\n42\n0.0\n281\n0\n93\n{strokes}\n");
        for k in 0..strokes {
            p += &format!("292\n{}\n293\n0\n294\n1\n295\n0\n296\n0\n43\n{}.5\n44\n0.0\n45\n0.0\n94\n0\n95\n0\n", k % 2, k);
        }
        p
    };
    let (stroke_pattern, internal_pattern) = (pattern(4), pattern(7));
    for i in 0..n {
        let (kind, class, body, tail) = if i < strokes {
            ("LSSTROKEPATTERNCOMPONENT", "AcDbLSStrokePatternComponent", &stroke_pattern, "")
        } else {
            ("LSINTERNALCOMPONENT", "AcDbLSInternalComponent", &internal_pattern, "96\n1\n97\n0\n297\n0\n98\n3\n")
        };
        s += &format!(
            "0\n{kind}\n5\n{}\n102\n{{ACAD_REACTORS\n330\nD0\n102\n}}\n330\nD0\n100\n{class}\n1\nComponent {i}\n90\n2\n91\n1\n310\n{i:032X}\n40\n1.0\n280\n0\n{body}{tail}",
            comp(i)
        );
    }
    let _ = internal;
    s += "0\nENDSEC\n0\nEOF\n";
    s
}

/// How long a drawing full of DGN line-style objects takes to open, as DXF and (with
/// `CADCRAFT_PERF_DGN_DWG=1`, through the DWG bridge, which takes a while to make the DWG) as
/// DWG, with the memory it needs. `CADCRAFT_PERF_DGN` sets the number of objects (default
/// 288 000, the real file's); `CADCRAFT_PERF_DGN_OUT` names a path to keep the DWG at.
/// Measured on 02/11/2026 (release, 288 000 objects, a 223 MB DXF): before this round the
/// drawing took 365 s and 3.1 GB to open (26 million groups kept, then the raw keeper reading
/// the dictionary's head once per object); after, 2.6 s and 353 MB, the objects left out with a note.
#[test]
#[ignore]
fn open_a_drawing_full_of_dgn_line_styles() {
    let n: usize = std::env::var("CADCRAFT_PERF_DGN").ok().and_then(|v| v.parse().ok()).unwrap_or(288_000);
    let t = Instant::now();
    let dxf = dgn_profile_dxf(n);
    eprintln!("DXF of {n} DGN line-style objects: {:.1}s, {} MB, {}", t.elapsed().as_secs_f64(), dxf.len() / 1048576, mem());
    let t = Instant::now();
    let d = cadcraft_io::read_dxf(dxf.as_bytes()).unwrap();
    eprintln!(
        "open DXF: {:.1}s, {} entities, {} blocks, {} kept objects, {}",
        t.elapsed().as_secs_f64(),
        d.model.len(),
        d.blocks.len(),
        d.foreign.objects.len(),
        mem()
    );
    for note in &d.read_notes {
        eprintln!("  note: {note}");
    }
    if std::env::var("CADCRAFT_PERF_DGN_DWG").is_ok_and(|v| v == "1") {
        drop(d);
        let t = Instant::now();
        let dwg = cadcraft_dwg::dxf_to_dwg(dxf.as_bytes()).unwrap();
        eprintln!("DWG through the bridge: {:.1}s, {} MB, {}", t.elapsed().as_secs_f64(), dwg.len() / 1048576, mem());
        if let Ok(path) = std::env::var("CADCRAFT_PERF_DGN_OUT") {
            std::fs::write(&path, &dwg).unwrap();
            eprintln!("  kept at {path}");
        }
        drop(dxf);
        let t = Instant::now();
        let d = cadcraft_io::read(&dwg, "dgn.dwg").unwrap();
        eprintln!("open DWG: {:.1}s, {} entities, {} kept objects, {}", t.elapsed().as_secs_f64(), d.model.len(), d.foreign.objects.len(), mem());
        for note in &d.read_notes {
            eprintln!("  note: {note}");
        }
    }
}
