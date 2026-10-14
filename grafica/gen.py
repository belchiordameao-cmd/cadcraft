"""Corpus de casos da grafica (DWG/DXF de cliente -> PDF de plotagem). Uso: python3 gen.py OUTDIR"""
import sys, os
import ezdxf
from ezdxf.enums import TextEntityAlignment

out = sys.argv[1]
os.makedirs(out, exist_ok=True)

PAPERS = {"A0": (1189, 841), "A1": (841, 594), "A3": (420, 297), "A4": (297, 210)}


def base(meas=1, ins=4):
    d = ezdxf.new("R2018", setup=True)
    d.header["$MEASUREMENT"] = meas
    d.header["$INSUNITS"] = ins
    return d


def planta(msp, ox=0.0, oy=0.0):
    """Planta 10x8 m em mm, com hachura solida, texto e cota."""
    msp.add_lwpolyline([(ox, oy), (ox + 10000, oy), (ox + 10000, oy + 8000), (ox, oy + 8000)], close=True, dxfattribs={"color": 7, "lineweight": 50})
    h = msp.add_hatch(color=1)
    h.paths.add_polyline_path([(ox + 1000, oy + 1000), (ox + 3000, oy + 1000), (ox + 3000, oy + 3000), (ox + 1000, oy + 3000)], is_closed=True)
    msp.add_text("SALA", height=300, dxfattribs={"color": 5}).set_placement((ox + 5000, oy + 4000), align=TextEntityAlignment.MIDDLE_CENTER)
    msp.add_linear_dim(base=(ox, oy - 600), p1=(ox, oy), p2=(ox + 10000, oy)).render()
    return (ox + 5000, oy + 4000)


def folha(d, nome, paper, escala, centro, landscape=True):
    w, h = PAPERS[paper]
    if not landscape:
        w, h = h, w
    L = d.layouts.new(nome)
    L.page_setup(size=(w, h), margins=(0, 0, 0, 0), units="mm")
    L.add_lwpolyline([(10, 10), (w - 10, 10), (w - 10, h - 10), (10, h - 10)], close=True, dxfattribs={"color": 7})
    L.add_text(f"{nome} {paper} 1:{escala}", height=5).set_placement((w - 15, 15), align=TextEntityAlignment.BOTTOM_RIGHT)
    vw, vh = w - 40, h - 40
    L.add_viewport(center=(w / 2, h / 2), size=(vw, vh), view_center_point=centro, view_height=vh * escala)
    return L


# G01 folha A1 1:50, planta em mm
d = base(); c = planta(d.modelspace()); folha(d, "PRANCHA", "A1", 50, c); d.saveas(f"{out}/G01_layout_A1_1-50.dxf")
# G02 A0 retrato 1:100
d = base(); c = planta(d.modelspace()); folha(d, "A0R", "A0", 100, c, landscape=False); d.saveas(f"{out}/G02_layout_A0_retrato.dxf")
# G03 coordenadas UTM (longe da origem), model space
d = base(); planta(d.modelspace(), 562000000.0, 9567000000.0); d.saveas(f"{out}/G03_utm_model.dxf")
# G04 tres pranchas
d = base(); c = planta(d.modelspace())
for n, p in (("P01", "A1"), ("P02", "A3"), ("P03", "A4")):
    folha(d, n, p, 50 if p == "A1" else 100, c)
d.saveas(f"{out}/G04_tres_pranchas.dxf")
# G05 layer desligada / congelada / nao-plotavel / Defpoints nao devem sair
d = base(); msp = d.modelspace(); c = planta(msp)
for nome, attrs in (("OFF", {"off": True}), ("FRZ", {"freeze": True}), ("NOPLOT", {"plot": False})):
    lay = d.layers.add(nome)
    if attrs.get("off"): lay.off()
    if attrs.get("freeze"): lay.freeze()
    if attrs.get("plot") is False: lay.dxf.plot = 0
    msp.add_circle((c[0], c[1]), 3500, dxfattribs={"layer": nome, "color": 3})
msp.add_circle((c[0], c[1]), 3800, dxfattribs={"layer": "Defpoints", "color": 3})
folha(d, "PRANCHA", "A3", 100, c); d.saveas(f"{out}/G05_layers_nao_plotam.dxf")
# G06 desenho em polegadas
d = base(meas=0, ins=1); msp = d.modelspace(); msp.add_lwpolyline([(0, 0), (100, 0), (100, 60), (0, 60)], close=True)
L = d.layouts.new("SHEET"); L.page_setup(size=(36, 24), margins=(0, 0, 0, 0), units="inch")
L.add_viewport(center=(18, 12), size=(30, 20), view_center_point=(50, 30), view_height=80); d.saveas(f"{out}/G06_polegadas_ARCHD.dxf")
# G07 xref ausente
d = base(); msp = d.modelspace(); c = planta(msp)
d.add_xref_def("C:/obra/base_inexistente.dwg", "BASE"); msp.add_blockref("BASE", (0, 0))
folha(d, "PRANCHA", "A3", 100, c); d.saveas(f"{out}/G07_xref_ausente.dxf")
# G08 layout sem page setup (paper 0x0)
d = base(); c = planta(d.modelspace()); L = d.layouts.new("SEMSETUP")
L.add_viewport(center=(150, 100), size=(250, 180), view_center_point=c, view_height=12000); d.saveas(f"{out}/G08_layout_sem_pagesetup.dxf")
# G09 viewport com camada congelada so nele (VPLAYER)
d = base(); msp = d.modelspace(); c = planta(msp); d.layers.add("MOBILIA")
msp.add_circle(c, 1500, dxfattribs={"layer": "MOBILIA", "color": 4})
L = folha(d, "PRANCHA", "A3", 100, c)
vp = [e for e in L if e.dxftype() == "VIEWPORT"][-1]; vp.frozen_layers = ["MOBILIA"]
d.saveas(f"{out}/G09_vp_layer_congelada.dxf")
print("ok")
# G10 texto TTF (Arial) e MTEXT acentuado
d = base(); msp = d.modelspace(); c = planta(msp)
d.styles.add("ARIAL", font="arial.ttf")
msp.add_text("PLANTA BAIXA - ÁREA 80m²", height=400, dxfattribs={"style": "ARIAL", "color": 1}).set_placement((0, 9000))
msp.add_mtext("Observação:\\Paçoca ção ã", dxfattribs={"char_height": 250, "insert": (0, -1500), "color": 1})
folha(d, "PRANCHA", "A3", 100, c); d.saveas(f"{out}/G10_texto_ttf_acentos.dxf")
# G11 viewport rotacionado (twist 90) e escala diferente em 2 viewports
d = base(); msp = d.modelspace(); c = planta(msp); L = folha(d, "PRANCHA", "A1", 50, c)
v2 = L.add_viewport(center=(700, 450), size=(200, 150), view_center_point=(c[0] - 3000, c[1] - 2000), view_height=150 * 20)
v2.dxf.view_twist_angle = 90
d.saveas(f"{out}/G11_vp_twist_detalhe.dxf")
# G12 bloco com atributos (carimbo) no layout
d = base(); msp = d.modelspace(); c = planta(msp)
blk = d.blocks.new("CARIMBO"); blk.add_lwpolyline([(0, 0), (180, 0), (180, 50), (0, 50)], close=True)
blk.add_attdef("CLIENTE", (5, 30), height=6); blk.add_attdef("PRANCHA", (5, 10), height=6)
L = folha(d, "PRANCHA", "A3", 100, c)
L.add_blockref("CARIMBO", (230, 15)).add_auto_attribs({"CLIENTE": "GRAFICA EUSEBIO", "PRANCHA": "01/01"})
d.saveas(f"{out}/G12_carimbo_atributos.dxf")
print("ok2")
# G13 plot style monochrome.ctb na prancha (cor deve sair preta)
d = base(); c = planta(d.modelspace()); L = folha(d, "PRANCHA", "A3", 100, c)
L.dxf_layout.dxf.current_style_sheet = "monochrome.ctb"; L.dxf_layout.dxf.plot_layout_flags |= 0x20
d.saveas(f"{out}/G13_ctb_monochrome.dxf")
# G14 hachura ANSI31 + gradiente
d = base(); msp = d.modelspace(); c = planta(msp)
h = msp.add_hatch(color=3); h.set_pattern_fill("ANSI31", scale=50)
h.paths.add_polyline_path([(6000, 1000), (9000, 1000), (9000, 3000), (6000, 3000)], is_closed=True)
g = msp.add_hatch(); g.set_gradient((255, 0, 0), (0, 0, 255), name="LINEAR")
g.paths.add_polyline_path([(6000, 5000), (9000, 5000), (9000, 7000), (6000, 7000)], is_closed=True)
folha(d, "PRANCHA", "A3", 100, c); d.saveas(f"{out}/G14_hachura_padrao_gradiente.dxf")
# G15 imagem raster referenciada (logo)
d = base(); msp = d.modelspace(); c = planta(msp)
idef = d.add_image_def(filename="logo_g15.png", size_in_pixel=(200, 100))
msp.add_image(idef, insert=(500, 5000), size_in_units=(2000, 1000))
folha(d, "PRANCHA", "A3", 100, c); d.saveas(f"{out}/G15_imagem_raster.dxf")
print("ok3")
