//! What a DWG written lacks of what the drawing had. The DWG bridge (acadrust) can leave records
//! out without a word: a real drawing saved as DWG lost an INSERT of an anonymous block with its
//! attribute, and the block with its record, while the same drawing saved as DXF kept them. So a
//! DWG written is read back and its records are counted against the DXF text it was written from:
//! what the count lost is reported as a warning, as a plot reports a blank page.

use std::collections::BTreeMap;

/// Records inside another record (a polyline's vertices, a block reference's attributes, the
/// end of either) and the records that frame a section's contents: not counted as things on
/// their own, so that a record written another way (a polyline as a lightweight one) is not
/// reported as lost, but named when what holds them went.
const NOT_ON_THEIR_OWN: &[&str] = &["VERTEX", "ATTRIB", "SEQEND", "ENDBLK", "TABLE", "ENDTAB"];

/// The sections compared: what is drawn and what names it. Objects are not: the bridge adds
/// and drops dictionaries of its own.
const SECTIONS: &[&str] = &["ENTITIES", "BLOCKS", "TABLES"];

/// Records by type, per compared section, of a DXF text (DGN line-style objects stepped over:
/// the drawing keeps a bounded number of them, and they live in OBJECTS anyway).
fn counts(text: &[u8]) -> Option<BTreeMap<&'static str, BTreeMap<String, usize>>> {
    let parsed = cadcraft_dxf::parse_sections(text, &cadcraft_dwg::is_dgn_line_style).ok()?;
    let mut out: BTreeMap<&'static str, BTreeMap<String, usize>> = BTreeMap::new();
    for s in &parsed.sections {
        let Some(name) = SECTIONS.iter().find(|n| **n == s.name) else { continue };
        let by_type = out.entry(name).or_default();
        for (kind, _) in cadcraft_dxf::record_slices(&s.tags) {
            *by_type.entry(kind).or_insert(0) += 1;
        }
    }
    Some(out)
}

/// The warnings for a DWG whose rendition `read_back` (the bridge's DXF text of the file
/// written) has fewer records than `written_from` (the text it was written from): one per
/// section that lost records of their own, naming every type with fewer records.
pub(crate) fn dwg_loss(written_from: &[u8], read_back: &[u8]) -> Vec<String> {
    let (Some(before), Some(after)) = (counts(written_from), counts(read_back)) else {
        return vec!["The DWG written could not be counted against the drawing: whether it kept everything is not known.".into()];
    };
    let mut lines = Vec::new();
    for section in SECTIONS {
        let (Some(b), a) = (before.get(section), after.get(section).cloned().unwrap_or_default()) else { continue };
        let own = |m: &BTreeMap<String, usize>| m.iter().filter(|(k, _)| !NOT_ON_THEIR_OWN.contains(&k.as_str())).map(|(_, n)| n).sum::<usize>();
        if own(b) <= own(&a) {
            continue;
        }
        let fewer: Vec<String> = b
            .iter()
            .filter_map(|(k, n)| {
                let lost = n.saturating_sub(a.get(k).copied().unwrap_or(0));
                (lost > 0).then(|| format!("{lost} {k}"))
            })
            .collect();
        lines.push(format!("{section}: {}", fewer.join(", ")));
    }
    if lines.is_empty() {
        return Vec::new();
    }
    vec![format!("The DWG written lacks records the drawing had; the DWG writer left them out ({}). Saving as DXF keeps them.", lines.join("; "))]
}

#[cfg(test)]
mod tests {
    use super::dwg_loss;

    fn text(entities: &[&str], blocks: &[&str], records: &[&str]) -> Vec<u8> {
        let mut s = String::from("0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n70\n1\n");
        for r in records {
            s += &format!("0\nBLOCK_RECORD\n5\n2\n2\n{r}\n");
        }
        s += "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n";
        for b in blocks {
            s += &format!("0\nBLOCK\n2\n{b}\n0\nLINE\n8\n0\n0\nENDBLK\n");
        }
        s += "0\nENDSEC\n0\nSECTION\n2\nENTITIES\n";
        for e in entities {
            s += &format!("0\n{e}\n8\n0\n");
        }
        s += "0\nENDSEC\n0\nEOF\n";
        s.into_bytes()
    }

    #[test]
    fn a_lost_block_reference_with_its_block_is_named() {
        let before = text(&["LINE", "INSERT", "ATTRIB", "SEQEND", "CIRCLE"], &["*U5"], &["*U5"]);
        let after = text(&["LINE", "CIRCLE"], &[], &[]);
        let w = dwg_loss(&before, &after);
        assert_eq!(w.len(), 1, "{w:?}");
        assert_eq!(
            w[0],
            "The DWG written lacks records the drawing had; the DWG writer left them out \
             (ENTITIES: 1 ATTRIB, 1 INSERT, 1 SEQEND; BLOCKS: 1 BLOCK, 1 ENDBLK, 1 LINE; TABLES: 1 BLOCK_RECORD). Saving as DXF keeps them."
        );
    }

    #[test]
    fn nothing_lost_and_records_written_another_way_say_nothing() {
        let before = text(&["LINE", "POLYLINE", "VERTEX", "VERTEX", "SEQEND"], &["A"], &["A"]);
        assert!(dwg_loss(&before, &before).is_empty());
        // A polyline read back as a lightweight one: as many things of their own.
        let after = text(&["LINE", "LWPOLYLINE"], &["A"], &["A"]);
        assert!(dwg_loss(&before, &after).is_empty());
        // Anonymous blocks renumbered by the bridge: the same count of blocks.
        let renamed = text(&["LINE", "POLYLINE", "VERTEX", "VERTEX", "SEQEND"], &["*U0"], &["*U0"]);
        assert!(dwg_loss(&before, &renamed).is_empty());
    }

    #[test]
    fn a_rendition_that_cannot_be_counted_is_said() {
        let before = text(&["LINE"], &[], &[]);
        let w = dwg_loss(&before, b"\x00\x01not dxf");
        assert_eq!(w.len(), 1);
        assert!(w[0].starts_with("The DWG written could not be counted"), "{}", w[0]);
    }
}
