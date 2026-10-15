//! Raster images in PDF plots: an IMAGE whose file is found is placed as an image XObject
//! (with a soft mask for its transparency) instead of its crossed placeholder frame; one whose
//! file is missing keeps the placeholder.

use cadcraft_doc::{Common, Drawing, EntityKind, Image, Space};
use cadcraft_geom::{Vec2, Vec3};
use cadcraft_io::{PdfOptions, pdf};

/// A drawing with one 4 × 2 pixel image at (10, 20), one drawing unit per pixel.
fn with_image(path: &str) -> Drawing {
    let mut d = Drawing::new_metric();
    let img = Image { insert: Vec3::new(10.0, 20.0, 0.0), size: Vec2::new(4.0, 2.0), path: path.into(), ..Image::default() };
    d.add(&Space::Model, Common::default(), EntityKind::Image(img)).unwrap();
    d
}

fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| if b.is_ascii() { char::from(*b) } else { '?' }).collect()
}

#[test]
fn found_image_is_placed_instead_of_its_placeholder() {
    let dir = std::env::temp_dir().join(format!("cadcraft-pdf-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut png = image::RgbaImage::new(4, 2);
    png.put_pixel(0, 0, image::Rgba([255, 0, 0, 128]));
    png.save(dir.join("logo.png")).unwrap();
    // A path from another machine: found by its file name in the drawing's folder.
    let d = with_image("C:\\Projetos\\Logos\\logo.png");
    let o = PdfOptions { image_dir: Some(dir.to_string_lossy().into_owned()), ..PdfOptions::default() };
    let t = text(&pdf(&d, &Space::Model, &o).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
    assert!(t.contains("/Subtype /Image /Width 4 /Height 2 /ColorSpace /DeviceRGB"), "image XObject");
    assert!(t.contains("/ColorSpace /DeviceGray"), "soft mask for the transparent pixel");
    assert!(t.contains("/XObject << /Im0 "), "page resources");
    let content = &t[t.find("stream\nq\n").unwrap()..];
    assert!(content.contains(" cm\n/Im0 Do\nQ"), "drawn");
    // Frame and diagonals are not drawn: no path ops after the plot-area clip.
    let after_clip = &content[content.find(" re W n\n").unwrap() + 8..];
    assert!(!after_clip.contains(" m\n"), "placeholder left out: {after_clip}");
}

#[test]
fn missing_image_keeps_its_placeholder() {
    let d = with_image("nowhere/missing.png");
    let o = PdfOptions { image_dir: Some(std::env::temp_dir().to_string_lossy().into_owned()), ..PdfOptions::default() };
    let t = text(&pdf(&d, &Space::Model, &o).unwrap());
    assert!(!t.contains("/Subtype /Image"));
    assert!(!t.contains("/XObject"));
    let content = &t[t.find("stream\nq\n").unwrap()..];
    assert_eq!(content.matches(" m\n").count(), 3, "frame and two diagonals");
}
