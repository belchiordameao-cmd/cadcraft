//! A layout plotted from a DWG matches the plot of the drawing it was made from: the viewport's
//! view centre (stored relative to the view target in the file), its per-viewport frozen layers
//! (group 331, without CADCraft xdata) and its clip boundary all reach the PDF through the DWG
//! bridge, as they do through the DXF reader.

use cadcraft_doc::*;
use cadcraft_geom::{PolyVertex, Vec2, Vec3};
use cadcraft_io::{PdfOptions, pdf};

/// A metric A4 sheet with a frame and an L-clipped viewport (frozen on `Walls`) onto model lines
/// far from the origin, on layers 0 and `Walls`.
fn drawing() -> (Drawing, Space) {
    let mut d = Drawing::new_metric();
    d.layers.push(Layer { name: "Walls".into(), ..Layer::default() });
    for i in 0..=4 {
        let y = 1000.0 + f64::from(i) * 25.0;
        d.add(&Space::Model, Common::default(), EntityKind::Line(Line { a: Vec3::new(2000.0, y, 0.0), b: Vec3::new(2100.0, y, 0.0) })).unwrap();
    }
    let walls = Common { layer: "Walls".into(), ..Common::default() };
    d.add(&Space::Model, walls, EntityKind::Line(Line { a: Vec3::new(2000.0, 1000.0, 0.0), b: Vec3::new(2100.0, 1100.0, 0.0) })).unwrap();
    let sheet = Space::Paper(d.layouts[0].name.clone());
    {
        let p = &mut d.layouts[0].page;
        p.width_mm = 210.0;
        p.height_mm = 297.0;
        p.margins_mm = [5.0; 4];
        p.landscape = true;
    }
    let poly = |pts: &[[f64; 2]]| {
        EntityKind::LwPolyline(LwPolyline {
            vertices: pts.iter().map(|p| PolyVertex::new(Vec2::new(p[0], p[1]))).collect(),
            closed: true,
            const_width: 0.0,
            elevation: 0.0,
            plinegen: false,
        })
    };
    d.add(&sheet, Common::default(), poly(&[[0.0, 0.0], [287.0, 0.0], [287.0, 200.0], [0.0, 200.0]])).unwrap();
    let b = d
        .add(&sheet, Common::default(), poly(&[[20.0, 20.0], [220.0, 20.0], [220.0, 100.0], [120.0, 100.0], [120.0, 180.0], [20.0, 180.0]]))
        .unwrap();
    let vp = Viewport {
        center: Vec3::new(120.0, 100.0, 0.0),
        width: 200.0,
        height: 160.0,
        view_center: Vec2::new(2050.0, 1050.0),
        view_height: 160.0,
        id: 2,
        frozen_layers: vec!["Walls".into()],
        clip: Some(b),
        ..Viewport::default()
    };
    d.add(&sheet, Common::default(), EntityKind::Viewport(vp)).unwrap();
    (d, sheet)
}

/// The view target AutoCAD's groups are written against.
const TARGET: Vec2 = Vec2::new(1234.5, -678.0);

/// The drawing's DXF as AutoCAD 2004 and later write the viewport: view centre (12/22) relative
/// to the view target (17/27/37), frozen layers as 331 layer handles, no CADCraft xdata.
fn autocad_style(d: &Drawing) -> String {
    let text = cadcraft_io::write_dxf(d);
    let lines: Vec<&str> = text.lines().collect();
    let pairs: Vec<(String, String)> = lines.chunks(2).map(|c| (c[0].trim().to_string(), c.get(1).copied().unwrap_or("").to_string())).collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < pairs.len() {
        let (c, v) = &pairs[i];
        if c == "0" && v == "VIEWPORT" {
            let mut j = i + 1;
            while j < pairs.len() && pairs[j].0 != "0" {
                j += 1;
            }
            let rec = &pairs[i..j];
            if rec.iter().any(|(c, v)| c == "69" && v.trim() == "2") {
                for (c, v) in rec.iter().take_while(|(c, _)| c != "1001") {
                    let f = || v.trim().parse::<f64>().unwrap();
                    match c.as_str() {
                        "12" => out.push(("12".into(), (f() - TARGET.x).to_string())),
                        "22" => out.push(("22".into(), (f() - TARGET.y).to_string())),
                        "45" => {
                            out.push((c.clone(), v.clone()));
                            out.push(("17".into(), TARGET.x.to_string()));
                            out.push(("27".into(), TARGET.y.to_string()));
                            out.push(("37".into(), "0.0".into()));
                        }
                        "341" => out.push(("331".into(), v.clone())),
                        _ => out.push((c.clone(), v.clone())),
                    }
                }
                i = j;
                continue;
            }
        }
        out.push((c.clone(), v.clone()));
        i += 1;
    }
    out.iter().map(|(c, v)| format!("{c}\n{v}\n")).collect()
}

/// The `x y m` / `x y l` path points of an uncompressed PDF, to 0.01 pt, sorted.
fn points(pdf: &[u8]) -> Vec<(i64, i64)> {
    let t = String::from_utf8_lossy(pdf);
    let toks: Vec<&str> = t.split_whitespace().collect();
    let mut out: Vec<(i64, i64)> = toks
        .windows(3)
        .filter(|w| w[2] == "m" || w[2] == "l")
        .filter_map(|w| Some(((w[0].parse::<f64>().ok()? * 100.0).round() as i64, (w[1].parse::<f64>().ok()? * 100.0).round() as i64)))
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn autocad_style_viewport_reaches_the_plot_through_dwg_as_through_dxf() {
    let (d, sheet) = drawing();
    let text = autocad_style(&d);
    let from_dxf = cadcraft_io::read_dxf(text.as_bytes()).unwrap();
    let dwg = cadcraft_dwg::dxf_to_dwg(text.as_bytes()).unwrap();
    let from_dwg = cadcraft_io::read(&dwg, "t.dwg").unwrap();
    let vp = |d: &Drawing| {
        d.space(&sheet)
            .unwrap()
            .iter()
            .find_map(|e| match &e.kind {
                EntityKind::Viewport(v) if v.id == 2 => Some(v.clone()),
                _ => None,
            })
            .expect("viewport 2")
    };
    for (what, back) in [("DXF", &from_dxf), ("DWG", &from_dwg)] {
        let v = vp(back);
        assert!(v.view_center.near(Vec2::new(2050.0, 1050.0), 1e-6), "{what}: view centre {:?}", v.view_center);
        assert_eq!(v.frozen_layers, vec!["Walls".to_string()], "{what}: frozen layers");
        assert!(v.clip.is_some(), "{what}: clip boundary");
        assert!(v.on, "{what}: on");
    }
    let o = PdfOptions { compress: false, ..PdfOptions::default() };
    let reference = points(&pdf(&d, &sheet, &o).unwrap());
    assert!(reference.len() > 10, "{}", reference.len());
    assert_eq!(points(&pdf(&from_dxf, &sheet, &o).unwrap()), reference, "the DXF plot");
    assert_eq!(points(&pdf(&from_dwg, &sheet, &o).unwrap()), reference, "the DWG plot");
}
