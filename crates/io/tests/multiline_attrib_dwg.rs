//! A block reference's multiline attribute (its text an object embedded in the ATTRIB record)
//! through a save as DWG. Measured version by version: the bridge (acadrust 0.6.4) keeps the
//! embedded text at 2018 and drops it at 2007, 2010 and 2013 (the ATTRIB record stays, its value
//! the multiline contents with their codes), saying nothing. The check of the DWG read back now
//! names it. At 2004 and before there are no multiline attributes: the text goes as single-line
//! by design.

use cadcraft_doc::*;
use cadcraft_io::Version;

/// One block reference, 1x1, with one multiline attribute (value `VAL`, second line `LINE2`).
fn drawing() -> Drawing {
    let mut s = String::new();
    s += "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1027\n9\n$HANDSEED\n5\nFFFF\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n330\n0\n100\nAcDbSymbolTable\n70\n1\n\
0\nBLOCK_RECORD\n5\n100\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\nTAGBLK\n0\nENDTAB\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n5\n102\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\nTAGBLK\n70\n2\n10\n0\n20\n0\n30\n0\n3\nTAGBLK\n1\n\n\
0\nLINE\n5\n104\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n10\n21\n0\n31\n0\n\
0\nATTDEF\n5\n105\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n2\n30\n0\n40\n2.5\n1\nX\n100\nAcDbAttributeDefinition\n3\nTag:\n2\nTAG1\n70\n0\n\
0\nENDBLK\n5\n103\n330\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n0\nENDSEC\n";
    s += "0\nSECTION\n2\nENTITIES\n\
0\nINSERT\n5\n400\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockReference\n66\n1\n2\nTAGBLK\n10\n0\n20\n0\n30\n0\n\
0\nATTRIB\n5\n401\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n2\n30\n0\n40\n2.5\n1\nVAL\n100\nAcDbAttribute\n2\nTAG1\n70\n0\n\
71\n2\n72\n1\n11\n0\n21\n2\n31\n0\n101\nEmbedded Object\n10\n0\n20\n2\n30\n0\n40\n2.5\n41\n20\n46\n0\n71\n1\n72\n5\n1\nVAL\\PLINE2\n7\nStandard\n73\n1\n44\n1\n\
0\nSEQEND\n5\n402\n330\n1F\n100\nAcDbEntity\n8\n0\n0\nENDSEC\n0\nEOF\n";
    cadcraft_io::read_dxf(s.as_bytes()).unwrap()
}

/// The attributes of the drawing's block references: (multiline, value).
fn attribs(d: &Drawing) -> Vec<(bool, String)> {
    d.model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Insert(i) => Some(i.attribs.iter().map(|a| (a.multiline.is_some(), a.text.value.clone()))),
            _ => None,
        })
        .flatten()
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_multiline_attribute_lost_through_the_dwg_bridge_is_said() {
    let d = drawing();
    assert_eq!(attribs(&d), vec![(true, "VAL\nLINE2".to_string())]);
    let mut report = String::new();
    let mut wrong = Vec::new();
    for v in [Version::R2007, Version::R2010, Version::R2013, Version::R2018] {
        let opts = cadcraft_io::SaveOptions { version: Some(v), ..Default::default() };
        let (dwg, warnings) = cadcraft_io::write_with_warnings(&d, "x.dwg", &opts).unwrap();
        let back = attribs(&cadcraft_io::read(&dwg, "x.dwg").unwrap());
        report += &format!("{v:?}: back {back:?}, warnings {warnings:?}\n");
        let kept = back == vec![(true, "VAL\nLINE2".to_string())];
        let said = warnings.len() == 1 && warnings[0].contains("1 ATTRIB with multiline text");
        // Kept whole and nothing said, or lost and said: never lost in silence.
        if !(kept && warnings.is_empty() || !kept && said) {
            wrong.push(format!("{v:?}"));
        }
    }
    eprintln!("{report}");
    assert!(wrong.is_empty(), "lost in silence, or warned for nothing, at {wrong:?}:\n{report}");
    // A DXF keeps it, and says nothing.
    let opts = cadcraft_io::SaveOptions { version: Some(Version::R2013), ..Default::default() };
    let (dxf, warnings) = cadcraft_io::write_with_warnings(&d, "x.dxf", &opts).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(attribs(&cadcraft_io::read(&dxf, "x.dxf").unwrap()), vec![(true, "VAL\nLINE2".to_string())]);
}
