Raster IMAGE entities plot as a crossed placeholder box in PDF

**Environment:** cadcraft 0.7.0 (`main` fc4e983), Linux, `cadcraft-cli convert --layout`.

**What happens.** A drawing with an IMAGE (a PNG logo next to the DXF, `IMAGEDEF` with a relative file name) plots to PDF as its frame with two diagonals (`crates/render/src/lib.rs`, `EntityKind::Image`) instead of the picture. The PDF writer (`crates/io/src/pdf.rs`) has no image XObject.

**Why it matters.** Title blocks with a company logo and sheets with an aerial photo or scanned survey under the drawing are common in drawings sent to print shops; the sheet comes out without them.

**Expected.** The image placed, scaled and clipped like AutoCAD's plot; when the file cannot be found, the frame (as now) and a warning.

Related: #1250 (images as external references in the editor).

(Written by Claude Code, an AI agent, on behalf of @belchiordameao.)
