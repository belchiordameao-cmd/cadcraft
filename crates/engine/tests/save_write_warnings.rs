//! A save says what the file written lacks of the drawing when the host's writer can tell
//! (a DWG read back and counted against the drawing: the DWG writer can leave records out
//! without a word), after what reading the drawing left out, on the command line as well.

use cadcraft_engine::Session;
use cadcraft_engine::cmd::file::{IoHooks, set_io, set_write_noted};
use serde_json::json;

#[test]
fn saving_reports_what_the_writer_left_out() {
    set_io(IoHooks { read: |_, _| Err("no reader in this test".into()), write: |_, _| Ok(b"0\nEOF\n".to_vec()), plot: None });
    set_write_noted(|_, name, _| {
        let lacks =
            if name.ends_with(".dwg") { vec!["The DWG written lacks records the drawing had (ENTITIES: 1 INSERT).".to_string()] } else { Vec::new() };
        Ok((b"0\nEOF\n".to_vec(), lacks))
    });
    let dir = std::env::temp_dir().join(format!("cadcraft-save-lacks-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = Session::new();
    s.cmdline("circle 0,0 5").unwrap();
    std::sync::Arc::make_mut(&mut s.docs[s.active].doc).read_notes = vec!["12 things were left out".into()];
    let r = s.execute("saveas", &json!({ "path": dir.join("a.dwg").to_string_lossy() })).unwrap();
    assert_eq!(r["warnings"], json!(["12 things were left out", "The DWG written lacks records the drawing had (ENTITIES: 1 INSERT)."]));
    assert!(s.log.iter().any(|l| l == "Warning: The DWG written lacks records the drawing had (ENTITIES: 1 INSERT)."), "{:?}", s.log);
    // A DXF: only what reading left out.
    let r = s.execute("saveas", &json!({ "path": dir.join("a.dxf").to_string_lossy() })).unwrap();
    assert_eq!(r["warnings"], json!(["12 things were left out"]));
    // QSAVE goes the same way.
    let r = s.execute("qsave", &json!({ "path": dir.join("b.dwg").to_string_lossy() })).unwrap();
    assert_eq!(r["warnings"].as_array().map(Vec::len), Some(2));
}
