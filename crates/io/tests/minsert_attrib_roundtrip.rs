//! A block reference repeated in columns and rows (MINSERT) with attributes survives a save as
//! DWG. A real drawing lost one with 1 column, 3 rows and no spacing (its INSERT, ATTRIB and
//! SEQEND gone, no word said); the fleet reproduced it from the sample: 1x3 and 3x1 with
//! attributes and zero spacing lose it, while 1x1, no attributes, or a spacing of 5 keep it.
//! Each stage is checked on its own: CADCraft's DXF text, the DWG the bridge writes (its record
//! count), the bridge's reading of it, and CADCraft's reading of that.

use cadcraft_doc::*;

/// A drawing with one block reference of `cols` x `rows`, `spacing` apart, with or without an
/// attribute, plus a line to anchor the extents.
fn drawing(cols: i64, rows: i64, attrib: bool, spacing: f64) -> String {
    let mut s = String::new();
    s += "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1027\n9\n$HANDSEED\n5\nFFFF\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n5\n2\n330\n0\n100\nAcDbSymbolTable\n70\n1\n\
0\nLAYER\n5\n10\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n0\n70\n0\n62\n7\n6\nContinuous\n0\nENDTAB\n\
0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n330\n0\n100\nAcDbSymbolTable\n70\n3\n\
0\nBLOCK_RECORD\n5\n1F\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Model_Space\n\
0\nBLOCK_RECORD\n5\n1B\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Paper_Space\n\
0\nBLOCK_RECORD\n5\n100\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\nTAGBLK\n\
0\nENDTAB\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n5\n20\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Model_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Model_Space\n1\n\n\
0\nENDBLK\n5\n21\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nBLOCK\n5\n22\n330\n1B\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Paper_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Paper_Space\n1\n\n\
0\nENDBLK\n5\n23\n330\n1B\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nBLOCK\n5\n102\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\nTAGBLK\n70\n2\n10\n0\n20\n0\n30\n0\n3\nTAGBLK\n1\n\n\
0\nLINE\n5\n104\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n10\n21\n0\n31\n0\n\
0\nATTDEF\n5\n105\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n2\n30\n0\n40\n2.5\n1\nX\n100\nAcDbAttributeDefinition\n3\nTag:\n2\nTAG1\n70\n0\n\
0\nENDBLK\n5\n103\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nENDSEC\n";
    s += "0\nSECTION\n2\nENTITIES\n";
    s += &format!(
        "0\nINSERT\n5\n400\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockReference\n{}2\nTAGBLK\n10\n0\n20\n0\n30\n0\n70\n{cols}\n71\n{rows}\n44\n{spacing}\n45\n{spacing}\n",
        if attrib { "66\n1\n" } else { "" }
    );
    if attrib {
        s += "0\nATTRIB\n5\n401\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n2\n30\n0\n40\n2.5\n1\nVAL\n100\nAcDbAttribute\n2\nTAG1\n70\n0\n\
0\nSEQEND\n5\n402\n330\n1F\n100\nAcDbEntity\n8\n0\n";
    }
    s += "0\nLINE\n5\n500\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n5\n21\n5\n31\n0\n";
    s += "0\nENDSEC\n0\nEOF\n";
    s
}

fn records(text: &str, kind: &str) -> usize {
    text.lines().map(str::trim).collect::<Vec<_>>().chunks(2).filter(|p| *p == ["0", kind]).count()
}

/// (INSERT, ATTRIB, SEQEND) records in a DXF text.
fn iag(text: &str) -> (usize, usize, usize) {
    (records(text, "INSERT"), records(text, "ATTRIB"), records(text, "SEQEND"))
}

fn inserts(d: &Drawing) -> Vec<(u32, u32, f64, usize)> {
    d.model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Insert(i) => Some((i.cols, i.rows, i.col_spacing, i.attribs.len())),
            _ => None,
        })
        .collect()
}

/// Every stage of a save as DWG, for one shape of reference.
#[cfg(not(target_arch = "wasm32"))]
fn stages(cols: i64, rows: i64, attrib: bool, spacing: f64) -> String {
    let d = cadcraft_io::read_dxf(drawing(cols, rows, attrib, spacing).as_bytes()).unwrap();
    let text = cadcraft_io::write_dxf(&d);
    let (dwg, warnings) = cadcraft_io::write_with_warnings(&d, "x.dwg", &cadcraft_io::SaveOptions::default()).unwrap();
    let (back, _, probe) = cadcraft_dwg::dwg_to_dxf_probed(&dwg, cadcraft_dwg::Limits::DEFAULT).unwrap();
    let back = String::from_utf8_lossy(&back).to_string();
    let again = cadcraft_io::read(&dwg, "x.dwg").unwrap();
    format!(
        "text {:?}; dwg records {} ({} entities read, {} skipped); reread {:?}; drawing {:?}; warnings {}",
        iag(&text),
        probe.source_records,
        probe.entities,
        probe.skipped_records,
        iag(&back),
        inserts(&again),
        warnings.len()
    )
}

/// The fleet's matrix, measured: (columns, rows, attribute, spacing). With attributes the bridge
/// lost 1x3, 3x1 and 2x2 at zero spacing (its reader skipped the record) and read 1x3 and 2x2 at
/// spacing 5 as 1x1 and 1x2 (wrong sizes); without attributes, or 1x1, it kept them. Now a
/// repeated reference with attributes goes to the bridge as one plain reference per copy, with
/// the attributes moved along: every copy comes back, placed as the renderer placed it.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repeated_block_references_with_attributes_survive_a_dwg_save() {
    let cases: [(i64, i64, bool, f64); 7] =
        [(1, 1, true, 0.0), (1, 3, true, 0.0), (3, 1, true, 0.0), (1, 3, false, 0.0), (1, 3, true, 5.0), (2, 2, true, 5.0), (2, 2, true, 0.0)];
    let mut report = String::new();
    let mut wrong = Vec::new();
    for (cols, rows, attrib, spacing) in cases {
        report += &format!("{cols}x{rows} attrib={attrib} spacing={spacing}: {}\n", stages(cols, rows, attrib, spacing));
        let d = cadcraft_io::read_dxf(drawing(cols, rows, attrib, spacing).as_bytes()).unwrap();
        let (dwg, warnings) = cadcraft_io::write_with_warnings(&d, "x.dwg", &cadcraft_io::SaveOptions::default()).unwrap();
        let again = cadcraft_io::read(&dwg, "x.dwg").unwrap();
        let mut ins: Vec<(u32, u32, f64, usize, f64, f64)> = again
            .model
            .iter()
            .filter_map(|e| match &e.kind {
                EntityKind::Insert(i) => Some((i.cols, i.rows, i.col_spacing, i.attribs.len(), i.insert.x, i.insert.y)),
                _ => None,
            })
            .collect();
        ins.sort_by(|a, b| (a.4, a.5).partial_cmp(&(b.4, b.5)).unwrap_or(std::cmp::Ordering::Equal));
        let expected: Vec<(u32, u32, f64, usize, f64, f64)> = if attrib && (cols > 1 || rows > 1) {
            let mut v: Vec<_> = (0..rows).flat_map(|r| (0..cols).map(move |c| (1, 1, 0.0, 1, spacing * c as f64, spacing * r as f64))).collect();
            v.sort_by(|a, b| (a.4, a.5).partial_cmp(&(b.4, b.5)).unwrap_or(std::cmp::Ordering::Equal));
            v
        } else {
            vec![(cols as u32, rows as u32, spacing, usize::from(attrib), 0.0, 0.0)]
        };
        if ins != expected || !warnings.is_empty() {
            wrong.push(format!("{cols}x{rows} attrib={attrib} spacing={spacing}: got {ins:?}, expected {expected:?}, warnings {warnings:?}"));
        }
    }
    eprintln!("{report}");
    assert!(wrong.is_empty(), "through the DWG save:\n{}\n{report}", wrong.join("\n"));
}

/// The bridge gets a copy per cell up to a limit (10 000); a reference with
/// more cells goes to the bridge as the one record it is (the DWG keeps its shape, the bridge may
/// lose it, and the check says so), instead of a text, a DWG and a reading of it the size of the
/// array. Measured before the limit: 500x500 wrote a 13 MB DWG, 1000x1000 ran out of memory
/// under a 4 GB ceiling and aborted without a file.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_repeated_reference_beyond_the_copy_limit_stays_one_record() {
    let (cols, rows) = (101, 100);
    assert!(cols * rows > 10_000, "over the limit of 10 000 copies");
    eprintln!("{cols}x{rows} attrib=true spacing=0: {}", stages(cols, rows, true, 0.0));
    eprintln!("{cols}x{rows} attrib=true spacing=5: {}", stages(cols, rows, true, 5.0));
    let d = cadcraft_io::read_dxf(drawing(cols, rows, true, 0.0).as_bytes()).unwrap();
    let (dwg, warnings) = cadcraft_io::write_with_warnings(&d, "x.dwg", &cadcraft_io::SaveOptions::default()).unwrap();
    assert!(dwg.len() < 100_000, "a DWG of one record, not of {} copies: {} bytes", cols * rows, dwg.len());
    // The bridge reads the record back with the wrong sizes (1 x 101) and the record count
    // unchanged, so the DWG read back says nothing: the text says it on its own.
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("TAGBLK 101 x 100") && warnings[0].contains("wrong columns and rows"), "{}", warnings[0]);
    // Within the limit, every copy is there.
    let d = cadcraft_io::read_dxf(drawing(100, 100, true, 0.0).as_bytes()).unwrap();
    let (dwg, warnings) = cadcraft_io::write_with_warnings(&d, "x.dwg", &cadcraft_io::SaveOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let again = cadcraft_io::read(&dwg, "x.dwg").unwrap();
    assert_eq!(inserts(&again).len(), 10_000);
}
