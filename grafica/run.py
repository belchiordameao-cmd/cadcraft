"""Roda o corpus contra um cadcraft-cli e mede cada PDF. Uso: python3 run.py CLI CORPUS OUT"""
import subprocess, sys, os, glob, re, json

cli, corpus, out = sys.argv[1:4]
os.makedirs(out, exist_ok=True)
PT = 72 / 25.4


def sh(*a):
    p = subprocess.run(a, capture_output=True, text=True, timeout=300)
    return p.returncode, (p.stdout + p.stderr).strip()


def medir(pdf):
    rc, info = sh("pdfinfo", pdf)
    m = re.search(r"Page size:\s+([\d.]+) x ([\d.]+)", info)
    pages = re.search(r"Pages:\s+(\d+)", info)
    w, h = (float(m.group(1)) / PT, float(m.group(2)) / PT) if m else (0, 0)
    png = pdf[:-4]
    sh("pdftoppm", "-r", "20", "-png", "-singlefile", pdf, png)
    from PIL import Image
    im = Image.open(png + ".png").convert("RGB")
    px = list(im.get_flattened_data()) if hasattr(im, "get_flattened_data") else list(im.getdata())
    tinta = sum(1 for p in px if min(p) < 240) / max(len(px), 1)
    verde = sum(1 for r, g, b in px if g > 150 and r < 100 and b < 100) / max(len(px), 1)
    ciano = sum(1 for r, g, b in px if g > 150 and b > 150 and r < 100) / max(len(px), 1)
    return {"pag": int(pages.group(1)) if pages else 0, "mm": [round(w), round(h)], "tinta%": round(100 * tinta, 2), "verde%": round(100 * verde, 3), "ciano%": round(100 * ciano, 3)}


res = {}
for f in sorted(glob.glob(f"{corpus}/*.dxf") + glob.glob(f"{corpus}/*.dwg")):
    n = os.path.basename(f)[:-4]
    r = {"info": sh(cli, "info", f)[1][:400]}
    rc, o = sh(cli, "convert", f, f"{out}/{n}-model.pdf")
    r["model"] = medir(f"{out}/{n}-model.pdf") if rc == 0 else f"ERRO rc={rc}: {o[:300]}"
    if not n.startswith("G03"):
        rc, o = sh(cli, "convert", f, f"{out}/{n}.pdf", "--layout", "all")
        r["layouts_log"] = o[:400]
        r["layouts"] = {os.path.basename(p): medir(p) for p in sorted(glob.glob(f"{out}/{n}-*.pdf")) if not p.endswith("-model.pdf")} if rc == 0 else f"ERRO rc={rc}: {o[:300]}"
    res[n] = r
print(json.dumps(res, indent=1, ensure_ascii=False))
