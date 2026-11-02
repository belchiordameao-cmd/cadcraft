//! An INSERT of an anonymous block (`*U…`, as AutoCAD makes for dynamic block instances) with
//! attributes, in model space, survives a save as DWG here: a real drawing lost one that way (its
//! INSERT, ATTRIB, SEQEND, BLOCK and BLOCK_RECORD all gone, without a word) while a save as DXF
//! kept it, and this shape does not reproduce it. Each stage is checked on its own: CADCraft's
//! DXF text, acadrust's DWG written and read back, CADCraft's reading of that; and the save
//! reports nothing lost (`write_with_warnings` counts the DWG read back against the drawing).

use cadcraft_doc::*;

fn anonymous_block_dxf() -> String {
    let mut s = String::new();
    s += "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1027\n9\n$HANDSEED\n5\nFFFF\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n5\n2\n330\n0\n100\nAcDbSymbolTable\n70\n1\n\
0\nLAYER\n5\n10\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n0\n70\n0\n62\n7\n6\nContinuous\n0\nENDTAB\n\
0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n330\n0\n100\nAcDbSymbolTable\n70\n2\n\
0\nBLOCK_RECORD\n5\n1F\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Model_Space\n\
0\nBLOCK_RECORD\n5\n1B\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Paper_Space\n\
0\nBLOCK_RECORD\n5\n100\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*U5\n\
0\nENDTAB\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n5\n20\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Model_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Model_Space\n1\n\n\
0\nENDBLK\n5\n21\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nBLOCK\n5\n22\n330\n1B\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Paper_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Paper_Space\n1\n\n\
0\nENDBLK\n5\n23\n330\n1B\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nBLOCK\n5\n102\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*U5\n70\n3\n10\n0\n20\n0\n30\n0\n3\n*U5\n1\n\n\
0\nLINE\n5\n104\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n10\n21\n0\n31\n0\n\
0\nATTDEF\n5\n105\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n2\n30\n0\n40\n2.5\n1\nX\n100\nAcDbAttributeDefinition\n3\nTag:\n2\nTAG1\n70\n0\n\
0\nENDBLK\n5\n103\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nENDSEC\n";
    s += "0\nSECTION\n2\nENTITIES\n\
0\nINSERT\n5\n400\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockReference\n66\n1\n2\n*U5\n10\n0\n20\n0\n30\n0\n\
0\nATTRIB\n5\n401\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n2\n30\n0\n40\n2.5\n1\nVAL\n100\nAcDbAttribute\n2\nTAG1\n70\n0\n\
0\nSEQEND\n5\n402\n330\n1F\n100\nAcDbEntity\n8\n0\n\
0\nLINE\n5\n500\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n5\n21\n5\n31\n0\n\
0\nENDSEC\n0\nEOF\n";
    s
}

fn records(text: &str, kind: &str) -> usize {
    text.lines().map(str::trim).collect::<Vec<_>>().chunks(2).filter(|p| *p == ["0", kind]).count()
}

fn inserts(d: &Drawing) -> Vec<(String, usize)> {
    d.model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Insert(i) => Some((i.block.clone(), i.attribs.len())),
            _ => None,
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn anonymous_block_insert_with_attributes_survives_a_dwg_save() {
    let d = cadcraft_io::read_dxf(anonymous_block_dxf().as_bytes()).unwrap();
    assert_eq!(inserts(&d), vec![("*U5".to_string(), 1)]);
    assert!(d.blocks.contains_key("*U5"), "{:?}", d.blocks.keys().collect::<Vec<_>>());

    // Stage 1: CADCraft's own DXF text keeps the block and the reference.
    let text = cadcraft_io::write_dxf(&d);
    assert_eq!((records(&text, "BLOCK"), records(&text, "INSERT"), records(&text, "ATTRIB"), records(&text, "SEQEND")), (3, 1, 1, 1), "CADCraft DXF");
    assert!(text.contains("\n*U5\n") || text.contains("\r\n*U5\r\n"), "the anonymous block keeps its name in the DXF text");

    // Stage 2: the DWG acadrust writes from that text, read back by acadrust.
    let dwg = cadcraft_io::write(&d, "x.dwg").unwrap();
    let back = String::from_utf8_lossy(&cadcraft_dwg::dwg_to_dxf(&dwg).unwrap()).to_string();
    assert_eq!(
        (records(&back, "BLOCK"), records(&back, "INSERT"), records(&back, "ATTRIB"), records(&back, "SEQEND")),
        (3, 1, 1, 1),
        "acadrust DWG write + read"
    );

    // Stage 3: CADCraft reading that DWG. The bridge numbers anonymous blocks afresh (AutoCAD
    // does too): the reference follows its block under the new name.
    let again = cadcraft_io::read(&dwg, "x.dwg").unwrap();
    let ins = inserts(&again);
    assert_eq!(ins.len(), 1, "{ins:?}");
    assert!(ins[0].0.starts_with("*U") && ins[0].1 == 1, "{ins:?}");
    assert!(again.blocks.contains_key(&ins[0].0), "{:?}", again.blocks.keys().collect::<Vec<_>>());

    // And the save reports nothing lost.
    let (_, warnings) = cadcraft_io::write_with_warnings(&d, "x.dwg", &cadcraft_io::SaveOptions::default()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
}
