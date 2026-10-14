# Bateria cadcraft — casos da gráfica (DWG/DXF de cliente → PDF de plotagem)

Objetivo: validar o `cadcraft-cli convert` como conversor do /orc (arquivo de arquiteto/engenheiro →
PDF da prancha no tamanho e escala reais, para medir papel e cobertura).

## Como rodar
1. Binário: branch `build-*` no fork `graficaeusebio-cloud/cadcraft` → workflow `build-cli` → artefato
   `cadcraft-cli` (build local proibido no VPS). Salvo em `/home/graficae/codex/work/cadcraft_bin/`.
2. `python3 gen.py corpus` — gera os casos G01–G12 (ezdxf, sintéticos, sem dado de cliente).
   DWG reais de referência: amostras do libredwg (`example_2000/2007/2013/2018`, `sample_2018`).
3. `python3 run.py <cli> corpus out > rN.json` — para cada arquivo: `info`, `convert` model space e
   `convert --layout all`; mede páginas, tamanho (mm), % de tinta e % de ciano/verde (layers-sonda).

## Casos e o que verificam
| Caso | Uso real | Verificação |
|---|---|---|
| G01 layout A1 1:50 | prancha padrão de arquiteto | PDF 841×594 mm, planta na escala |
| G02 A0 retrato 1:100 | prancha grande vertical | 841×1189 mm |
| G03 coordenadas UTM | levantamento topográfico/loteamento | fit em model space não vira ponto |
| G04 três pranchas A1/A3/A4 | jogo de pranchas num arquivo | 1 PDF por layout, cada um no seu papel |
| G05 layers off/frozen/no-plot/Defpoints | camadas de trabalho | nada disso sai no PDF |
| G06 polegadas ARCH D | arquivo de fora (EUA) | 914×610 mm |
| G07 xref ausente | cliente manda só o DWG principal | converte sem erro |
| G08 layout sem page setup | arquivo mal configurado | cai num papel padrão, não quebra |
| G09 layer congelada só no viewport | VPLAYER (mobiliário fora da prancha) | camada some só naquele viewport |
| G10 texto TTF + acentos | carimbo/legenda em português | texto e acentos presentes |
| G11 2 viewports, detalhe 1:20 girado | detalhe ampliado | os dois viewports saem |
| G12 bloco com atributos | carimbo com cliente/prancha | valores dos atributos saem |
| G13 monochrome.ctb | prancha plotada em P&B | cores saem pretas |
| G14 hachura ANSI31 + gradiente | piso/corte, fachada | padrão e degradê saem |
| G15 imagem raster (PNG) | logo no carimbo, foto aérea | imagem sai no PDF |

## Resultados (14/10/2026, cadcraft 0.7.0 + `convert --layout`)
- OK: G01–G08, G10–G12 — tamanho de papel exato, escala do viewport certa, layers não-plotáveis
  fora, UTM ok (fix do tilted insert já no upstream), xref ausente tolerado.
- Layouts `Layout1/Layout2` das amostras DWG saem em branco: **correto** (só têm o viewport de papel).
- **BUG G09** — layer congelada por viewport aparece no PDF. Causa: o leitor DXF lê a lista de layers
  congeladas do VIEWPORT só no grupo 341 (R2000–R2002); AutoCAD 2004+ e ezdxf gravam no **331**
  (libredwg `dwg.spec`, VIEWPORT). Fix: branch `vp-frozen-331` (338aab1), lê 331 e 341; teste novo
  `viewport_frozen_layers_read_from_331_as_autocad_2004_and_later_write_them`.
- G13 e G14 OK (CTB monocromático respeitado; padrão e gradiente).
- **LIMITAÇÃO G15** — IMAGE raster sai como caixa com X (render só tem placeholder; o PDF não tem XObject de imagem). Rascunho de issue: `ISSUE_raster_plot.md` (relacionada à #1250).
- Upstream: o bug do 331 já está coberto pelo PR #1004 (terceiro, aberto); rascunho de comentário com a evidência em `COMMENT_1004.md`. Nosso fix fica de reserva.
- Limitação conhecida (não-bug do corpus): cota com DIMSCALE 1 num desenho em mm fica minúscula na
  prancha — é o arquivo, não o conversor.

## Pendências / próximos casos
- Espessura de pena por cor (CTB colorido).
- DWG real de cliente (anonimizado) — só com autorização.
- Medida de cobertura (tinta) comparada com `application/libraries/cobertura_fator.php`.
