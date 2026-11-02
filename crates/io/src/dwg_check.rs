//! What a DWG written lacks of what the drawing had. The DWG bridge (acadrust) can leave records
//! out without a word: a real drawing saved as DWG lost an INSERT of an anonymous block with its
//! attribute, and the block with its record, while the same drawing saved as DXF kept them. So a
//! DWG written is read back and its records are counted against the DXF text it was written from:
//! what the count lost is reported as a warning, as a plot reports a blank page.

use std::collections::BTreeMap;

/// Records inside another record (a polyline's vertices, a block reference's attributes, the
/// end of either) and the records that frame a section's contents: not counted as things of
/// their own, but named when what holds them went.
const NOT_ON_THEIR_OWN: &[&str] = &["VERTEX", "ATTRIB", "SEQEND", "ENDBLK", "TABLE", "ENDTAB"];

/// Record types the bridge may write one as another of: fewer of one is no loss while the
/// family has as many (a polyline read back as a lightweight one).
const SAME_FAMILY: &[&[&str]] = &[&["POLYLINE", "LWPOLYLINE"], &["TEXT", "MTEXT"]];

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
        for (kind, tags) in cadcraft_dxf::record_slices(&s.tags) {
            // An attribute's multiline text is an object embedded in its record (`101`), which
            // the bridge drops at some versions while keeping the record: counted as a thing of
            // its own.
            if tags.iter().any(|t| t.code == 101) {
                *by_type.entry(format!("{kind} with multiline text")).or_insert(0) += 1;
            }
            *by_type.entry(kind).or_insert(0) += 1;
        }
    }
    Some(out)
}

/// The warnings for a DWG whose rendition `read_back` (the bridge's DXF text of the file
/// written) has fewer records than `written_from` (the text it was written from): one warning,
/// naming each section with fewer records of some type (type by type, so that a record the
/// bridge adds elsewhere in the section hides nothing; a type written as another of its family
/// is no loss), with every type that has fewer, sub-records included.
pub(crate) fn dwg_loss(written_from: &[u8], read_back: &[u8]) -> Vec<String> {
    let (Some(before), Some(after)) = (counts(written_from), counts(read_back)) else {
        return vec!["The DWG written could not be counted against the drawing: whether it kept everything is not known.".into()];
    };
    let mut lines = Vec::new();
    for section in SECTIONS {
        let (Some(b), a) = (before.get(section), after.get(section).cloned().unwrap_or_default()) else { continue };
        let n = |m: &BTreeMap<String, usize>, k: &str| m.get(k).copied().unwrap_or(0);
        let family_lost = |k: &str| match SAME_FAMILY.iter().find(|f| f.contains(&k)) {
            Some(f) => f.iter().map(|t| n(b, t)).sum::<usize>() > f.iter().map(|t| n(&a, t)).sum::<usize>(),
            None => true,
        };
        let lost_own = b.iter().any(|(k, c)| !NOT_ON_THEIR_OWN.contains(&k.as_str()) && *c > n(&a, k) && family_lost(k));
        if !lost_own {
            continue;
        }
        let fewer: Vec<String> = b
            .iter()
            .filter_map(|(k, c)| {
                let lost = c.saturating_sub(n(&a, k));
                (lost > 0 && (NOT_ON_THEIR_OWN.contains(&k.as_str()) || family_lost(k))).then(|| format!("{lost} {k}"))
            })
            .collect();
        lines.push(format!("{section}: {}", fewer.join(", ")));
    }
    if lines.is_empty() {
        return Vec::new();
    }
    vec![format!(
        "The DWG written, read back, lacks records the drawing had ({}): the DWG bridge lost them, writing or reading them. Saving as DXF keeps them.",
        lines.join("; ")
    )]
}

#[cfg(test)]
mod tests {
    use super::dwg_loss;

    #[test]
    fn an_attribute_whose_multiline_text_went_is_named() {
        let mut before = text(&["INSERT", "ATTRIB", "SEQEND"], &[], &[]);
        let marker = b"0\nATTRIB\n8\n0\n";
        let at = before.windows(marker.len()).position(|w| w == marker).unwrap() + marker.len();
        let mut with_text = before.split_off(at);
        before.extend_from_slice(b"71\n2\n101\nEmbedded Object\n10\n0\n20\n0\n30\n0\n1\nA\\PB\n");
        before.append(&mut with_text);
        let after = text(&["INSERT", "ATTRIB", "SEQEND"], &[], &[]);
        let w = dwg_loss(&before, &after);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("(ENTITIES: 1 ATTRIB with multiline text)"), "{}", w[0]);
        assert!(dwg_loss(&before, &before).is_empty());
    }

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
            "The DWG written, read back, lacks records the drawing had \
             (ENTITIES: 1 ATTRIB, 1 INSERT, 1 SEQEND; BLOCKS: 1 BLOCK, 1 ENDBLK, 1 LINE; TABLES: 1 BLOCK_RECORD): \
             the DWG bridge lost them, writing or reading them. Saving as DXF keeps them."
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

    /// A record the bridge adds hides nothing: the count is type by type, not the section's.
    #[test]
    fn a_record_added_elsewhere_in_the_section_hides_no_loss() {
        let before = text(&["LINE", "INSERT", "ATTRIB", "SEQEND"], &[], &[]);
        let after = text(&["LINE", "VIEWPORT", "VIEWPORT", "VIEWPORT"], &[], &[]);
        let w = dwg_loss(&before, &after);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("(ENTITIES: 1 ATTRIB, 1 INSERT, 1 SEQEND)"), "{}", w[0]);
    }

    #[test]
    fn a_rendition_that_cannot_be_counted_is_said() {
        let before = text(&["LINE"], &[], &[]);
        let w = dwg_loss(&before, b"\x00\x01not dxf");
        assert_eq!(w.len(), 1);
        assert!(w[0].starts_with("The DWG written could not be counted"), "{}", w[0]);
    }
}
