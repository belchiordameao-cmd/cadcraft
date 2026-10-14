A real-world data point for the 331 part of this PR: a DXF written by ezdxf 1.4 (which follows the DXF Reference and writes VIEWPORT frozen layers in group 331, as AutoCAD 2004+ does) loses its per-viewport frozen layers on `main` — `convert --layout` plots the frozen layer in that viewport. libredwg's `dwg.spec` agrees: 341 (hard pointer) only for R2000–R2002, 331 (soft pointer) from R2004.

Reading both 331 and 341 (with the VPFROZEN xdata still winning) fixes it here; the test I used is below in case it helps:

```rust
#[test]
fn viewport_frozen_layers_read_from_331_as_autocad_2004_and_later_write_them() {
    let (_, p) = frozen_vp_dxf();
    let r = vp_record(&p, "2");
    let x = p[r.clone()].iter().position(|(c, _)| c == "1001").unwrap() + r.start;
    let mut q: Vec<(String, String)> = p[..r.start].to_vec();
    q.extend(p[r.start..x].iter().map(|(c, v)| (if c == "341" { "331".into() } else { c.clone() }, v.clone())));
    q.extend(p[r.end..].iter().cloned());
    let back = read_dxf(join(&q).as_bytes()).unwrap();
    assert_eq!(viewport_frozen(&back, 2), vec!["Walls".to_string(), "A B".to_string()]);
}
```

(Written by Claude Code, an AI agent, on behalf of @belchiordameao.)
