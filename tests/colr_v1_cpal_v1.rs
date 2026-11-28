//! Synthetic COLR v1 / CPAL v1 integration tests without external font dependencies.

use fontcore::{Command, FontFile, GlyphGradientSpread, GlyphLayer, GlyphPaint};

struct SfntBuilder {
    tables: Vec<([u8; 4], Vec<u8>)>,
}

impl SfntBuilder {
    fn new() -> Self {
        Self { tables: Vec::new() }
    }

    fn push_table(&mut self, tag: [u8; 4], data: Vec<u8>) {
        self.tables.push((tag, data));
    }

    fn build(mut self) -> Vec<u8> {
        self.tables.sort_by_key(|(tag, _)| *tag);
        let num_tables = self.tables.len() as u16;
        let max_pow2 = (num_tables as u32).next_power_of_two()
            >> if num_tables.is_power_of_two() { 0 } else { 1 };
        let search_range = (max_pow2.max(1) * 16) as u16;
        let entry_selector = max_pow2.max(1).trailing_zeros() as u16;
        let range_shift = num_tables * 16 - search_range;

        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(&num_tables.to_be_bytes());
        out.extend_from_slice(&search_range.to_be_bytes());
        out.extend_from_slice(&entry_selector.to_be_bytes());
        out.extend_from_slice(&range_shift.to_be_bytes());

        let header_len = 12 + self.tables.len() * 16;
        let mut cursor = header_len;
        let mut dir = Vec::new();
        let mut body = Vec::new();
        for (tag, mut data) in self.tables {
            let length = data.len() as u32;
            while data.len() % 4 != 0 {
                data.push(0);
            }
            let checksum = calc_checksum(&data);
            dir.extend_from_slice(&tag);
            dir.extend_from_slice(&checksum.to_be_bytes());
            dir.extend_from_slice(&(cursor as u32).to_be_bytes());
            dir.extend_from_slice(&length.to_be_bytes());
            cursor += data.len();
            body.extend_from_slice(&data);
        }
        out.extend_from_slice(&dir);
        out.extend_from_slice(&body);
        out
    }
}

fn calc_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut buf = [0u8; 4];
        buf[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(buf));
    }
    sum
}

fn push_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn push_i16(buf: &mut Vec<u8>, v: i16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn push_u24(buf: &mut Vec<u8>, v: u32) {
    let b = v.to_be_bytes();
    buf.extend_from_slice(&b[1..4]);
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn make_rect_glyph(x0: i16, y0: i16, x1: i16, y1: i16) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i16(&mut buf, 1); // numberOfContours
    push_i16(&mut buf, x0);
    push_i16(&mut buf, y0);
    push_i16(&mut buf, x1);
    push_i16(&mut buf, y1);
    push_u16(&mut buf, 3); // endPtsOfContours[0]
    push_u16(&mut buf, 0); // instructionLength
    buf.extend_from_slice(&[0x01, 0x01, 0x01, 0x01]); // on-curve, 16-bit deltas
                                                      // x deltas
    push_i16(&mut buf, x0);
    push_i16(&mut buf, x1 - x0);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, x0 - x1);
    // y deltas
    push_i16(&mut buf, y0);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, y1 - y0);
    push_i16(&mut buf, 0);
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
    buf
}

fn add_base_tables(builder: &mut SfntBuilder, num_glyphs: u16, cmap_entries: &[(u16, u16)]) {
    // head (54 bytes)
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm
    head[40..42].copy_from_slice(&1000i16.to_be_bytes());
    head[42..44].copy_from_slice(&1000i16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes()); // long loca
    builder.push_table(*b"head", head);

    // hhea (36 bytes)
    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
    hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
    hhea[10..12].copy_from_slice(&600u16.to_be_bytes());
    hhea[34..36].copy_from_slice(&num_glyphs.to_be_bytes());
    builder.push_table(*b"hhea", hhea);

    // maxp (32 bytes, v1.0)
    let mut maxp = vec![0u8; 32];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());
    builder.push_table(*b"maxp", maxp);

    // hmtx
    let mut hmtx = Vec::new();
    for _ in 0..num_glyphs {
        push_u16(&mut hmtx, 600);
        push_i16(&mut hmtx, 0);
    }
    builder.push_table(*b"hmtx", hmtx);

    // glyf + loca: gid 0 empty, gid 1..n = unit rectangles
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    push_u32(&mut loca, 0); // gid 0 start (empty)
    for gid in 1..num_glyphs {
        push_u32(&mut loca, glyf.len() as u32);
        let rect = match gid {
            1 => make_rect_glyph(0, 0, 100, 100),
            2 => make_rect_glyph(10, 10, 60, 60),
            _ => make_rect_glyph(0, 0, 80, 80),
        };
        glyf.extend_from_slice(&rect);
    }
    push_u32(&mut loca, glyf.len() as u32);
    builder.push_table(*b"glyf", glyf);
    builder.push_table(*b"loca", loca);

    // cmap format 4 with one-char segments + 0xFFFF sentinel
    let seg_count = (cmap_entries.len() + 1) as u16;
    let seg_count_x2 = seg_count * 2;
    let max_pow2 =
        (seg_count as u32).next_power_of_two() >> if seg_count.is_power_of_two() { 0 } else { 1 };
    let search_range = (max_pow2.max(1) * 2) as u16;
    let entry_selector = max_pow2.max(1).trailing_zeros() as u16;
    let range_shift = seg_count_x2 - search_range;

    let mut subtable = Vec::new();
    push_u16(&mut subtable, 4); // format
    let sub_len = 16 + (seg_count as usize) * 8;
    push_u16(&mut subtable, sub_len as u16);
    push_u16(&mut subtable, 0); // language
    push_u16(&mut subtable, seg_count_x2);
    push_u16(&mut subtable, search_range);
    push_u16(&mut subtable, entry_selector);
    push_u16(&mut subtable, range_shift);
    for &(ch, _) in cmap_entries {
        push_u16(&mut subtable, ch);
    }
    push_u16(&mut subtable, 0xFFFF);
    push_u16(&mut subtable, 0); // reservedPad
    for &(ch, _) in cmap_entries {
        push_u16(&mut subtable, ch);
    }
    push_u16(&mut subtable, 0xFFFF);
    for &(ch, gid) in cmap_entries {
        let delta = gid.wrapping_sub(ch) as i16;
        push_i16(&mut subtable, delta);
    }
    push_i16(&mut subtable, 1);
    for _ in 0..seg_count {
        push_u16(&mut subtable, 0);
    }

    let mut cmap = Vec::new();
    push_u16(&mut cmap, 0);
    push_u16(&mut cmap, 1);
    push_u16(&mut cmap, 3);
    push_u16(&mut cmap, 1);
    push_u32(&mut cmap, 12);
    cmap.extend_from_slice(&subtable);
    builder.push_table(*b"cmap", cmap);

    // minimal name table
    let mut name = Vec::new();
    let label = "Test";
    push_u16(&mut name, 0); // format
    push_u16(&mut name, 1); // count
    push_u16(&mut name, 18); // stringOffset = 6 + 12
    push_u16(&mut name, 3); // platformID
    push_u16(&mut name, 1); // encodingID
    push_u16(&mut name, 0x0409); // languageID
    push_u16(&mut name, 1); // nameID (Font Family)
    push_u16(&mut name, (label.len() * 2) as u16);
    push_u16(&mut name, 0);
    for ch in label.encode_utf16() {
        push_u16(&mut name, ch);
    }
    builder.push_table(*b"name", name);
}

fn build_cpal_v1() -> Vec<u8> {
    // CPAL v1: 3 entries, 2 palettes (6 color records)
    // Header size = 12 + 2*2 + 12 = 28
    // Color records = 6 * 4 = 24 bytes (at 28..52)
    // Palette types = 2 * 4 = 8 bytes (at 52..60)
    // Palette labels = 2 * 2 = 4 bytes (at 60..64)
    // Entry labels = 3 * 2 = 6 bytes (at 64..70)
    let mut cpal = Vec::new();
    push_u16(&mut cpal, 1); // version = 1
    push_u16(&mut cpal, 3); // numPaletteEntries
    push_u16(&mut cpal, 2); // numPalettes
    push_u16(&mut cpal, 6); // numColorRecords
    push_u32(&mut cpal, 28); // colorRecordsArrayOffset
    push_u16(&mut cpal, 0); // palette 0 start
    push_u16(&mut cpal, 3); // palette 1 start
    push_u32(&mut cpal, 52); // paletteTypesArrayOffset
    push_u32(&mut cpal, 60); // paletteLabelsArrayOffset
    push_u32(&mut cpal, 64); // paletteEntryLabelsArrayOffset
                             // Color records (BGRA): palette 0 = [Red, Blue, Green], palette 1 = [Yellow, Cyan, Magenta]
    cpal.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]); // Red -> 0xFFFF0000
    cpal.extend_from_slice(&[0xFF, 0x00, 0x00, 0xFF]); // Blue -> 0xFF0000FF
    cpal.extend_from_slice(&[0x00, 0xFF, 0x00, 0xFF]); // Green -> 0xFF00FF00
    cpal.extend_from_slice(&[0x00, 0xFF, 0xFF, 0xFF]); // Yellow
    cpal.extend_from_slice(&[0xFF, 0xFF, 0x00, 0xFF]); // Cyan
    cpal.extend_from_slice(&[0xFF, 0x00, 0xFF, 0xFF]); // Magenta
                                                       // Palette types
    push_u32(&mut cpal, 1); // USABLE_WITH_LIGHT_BACKGROUND
    push_u32(&mut cpal, 2); // USABLE_WITH_DARK_BACKGROUND
                            // Palette labels
    push_u16(&mut cpal, 256);
    push_u16(&mut cpal, 257);
    // Entry labels
    push_u16(&mut cpal, 258);
    push_u16(&mut cpal, 259);
    push_u16(&mut cpal, 260);
    cpal
}

fn build_color_line(extend: u8, stops: &[(i16, u16, i16)]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(extend);
    push_u16(&mut buf, stops.len() as u16);
    for &(offset, palette_idx, alpha) in stops {
        push_i16(&mut buf, offset);
        push_u16(&mut buf, palette_idx);
        push_i16(&mut buf, alpha);
    }
    buf
}

fn build_paint_solid(palette_index: u16, alpha: i16) -> Vec<u8> {
    let mut p = Vec::new();
    p.push(2); // PaintSolid
    push_u16(&mut p, palette_index);
    push_i16(&mut p, alpha);
    p
}

fn build_paint_glyph_solid(glyph_id: u16, palette_index: u16) -> Vec<u8> {
    // PaintGlyph (6 bytes): format(1) + paintOffset(3) + glyphID(2)
    let mut p = Vec::new();
    p.push(10);
    push_u24(&mut p, 6);
    push_u16(&mut p, glyph_id);
    p.extend_from_slice(&build_paint_solid(palette_index, 0x4000));
    p
}

fn build_colr_v1_comprehensive() -> Vec<u8> {
    // Build Paint trees:
    // GID 1 ('A'): PaintColrLayers (2 layers: layer 0 = PaintGlyph(gid 1, Red), layer 1 = PaintTranslate(20, 40, PaintGlyph(gid 2, CurrentColor)))
    // GID 2 ('B'): PaintGlyph(gid 1, LinearGradient Pad)
    // GID 3 ('C'): PaintGlyph(gid 1, RadialGradient Reflect)
    // GID 4 ('D'): PaintColrGlyph(gid 1) (references 'A')
    // GID 5 ('E'): PaintColrGlyph(gid 6) -> GID 6: PaintColrGlyph(gid 5) (cycle!)
    // ClipList: gid 2 -> ClipBoxFormat1(0, 0, 100, 100)

    let mut layer0 = Vec::new();
    layer0.extend_from_slice(&build_paint_glyph_solid(1, 0));

    let mut layer1 = Vec::new();
    // PaintTranslate: format(14) + paintOffset(3 -> 8) + dx(i16=20) + dy(i16=40)
    layer1.push(14);
    push_u24(&mut layer1, 8);
    push_i16(&mut layer1, 20);
    push_i16(&mut layer1, 40);
    layer1.extend_from_slice(&build_paint_glyph_solid(2, 0xFFFF)); // CurrentColor

    // LayerList: numLayers(4) + 2*4=8 offsets -> header = 12 bytes
    let mut layer_list = Vec::new();
    push_u32(&mut layer_list, 2);
    let l0_off = 12u32;
    let l1_off = l0_off + layer0.len() as u32;
    push_u32(&mut layer_list, l0_off);
    push_u32(&mut layer_list, l1_off);
    layer_list.extend_from_slice(&layer0);
    layer_list.extend_from_slice(&layer1);

    // Base glyph paints:
    let mut paint_a = Vec::new();
    paint_a.push(1); // PaintColrLayers
    paint_a.push(2); // numLayers
    push_u32(&mut paint_a, 0); // firstLayerIndex

    let mut paint_b = Vec::new();
    // PaintGlyph -> PaintLinearGradient
    paint_b.push(10);
    push_u24(&mut paint_b, 6);
    push_u16(&mut paint_b, 1);
    // PaintLinearGradient: format(4) + colorLineOff(3 -> 16) + 6*i16(12) = 16 bytes
    paint_b.push(4);
    push_u24(&mut paint_b, 16);
    push_i16(&mut paint_b, 0); // x0
    push_i16(&mut paint_b, 0); // y0
    push_i16(&mut paint_b, 100); // x1
    push_i16(&mut paint_b, 0); // y1
    push_i16(&mut paint_b, 0); // x2
    push_i16(&mut paint_b, 100); // y2
    paint_b.extend_from_slice(&build_color_line(0, &[(0, 0, 0x4000), (0x4000, 1, 0x4000)]));

    let mut paint_c = Vec::new();
    // PaintGlyph -> PaintRadialGradient Reflect (extend=2)
    paint_c.push(10);
    push_u24(&mut paint_c, 6);
    push_u16(&mut paint_c, 1);
    // PaintRadialGradient: format(6) + colorLineOff(3 -> 16) + x0,y0,r0,x1,y1,r1 (12 bytes) = 16 bytes
    paint_c.push(6);
    push_u24(&mut paint_c, 16);
    push_i16(&mut paint_c, 50);
    push_i16(&mut paint_c, 50);
    push_u16(&mut paint_c, 10);
    push_i16(&mut paint_c, 50);
    push_i16(&mut paint_c, 50);
    push_u16(&mut paint_c, 50);
    paint_c.extend_from_slice(&build_color_line(2, &[(0, 1, 0x4000), (0x4000, 2, 0x4000)]));

    let mut paint_d = Vec::new();
    paint_d.push(11); // PaintColrGlyph
    push_u16(&mut paint_d, 1);

    let mut paint_e = Vec::new();
    paint_e.push(11);
    push_u16(&mut paint_e, 6);

    let mut paint_f = Vec::new();
    paint_f.push(11);
    push_u16(&mut paint_f, 5);

    let paints = [paint_a, paint_b, paint_c, paint_d, paint_e, paint_f];
    // BaseGlyphList: numBaseGlyphPaintRecords(4) + 6 * 6 bytes = 40 bytes header
    let mut base_glyph_list = Vec::new();
    push_u32(&mut base_glyph_list, paints.len() as u32);
    let mut cur = 4 + paints.len() * 6;
    for (idx, p) in paints.iter().enumerate() {
        push_u16(&mut base_glyph_list, (idx + 1) as u16);
        push_u32(&mut base_glyph_list, cur as u32);
        cur += p.len();
    }
    for p in paints {
        base_glyph_list.extend_from_slice(&p);
    }

    // ClipList: format=1(1) + numClips=1(4) + 1*(start(2)+end(2)+off24(3)) = 12 bytes
    let mut clip_list = Vec::new();
    clip_list.push(1);
    push_u32(&mut clip_list, 1);
    push_u16(&mut clip_list, 2); // startGlyphID = 2
    push_u16(&mut clip_list, 2); // endGlyphID = 2
    push_u24(&mut clip_list, 12); // clipBoxOffset from ClipList start
                                  // ClipBoxFormat1: format=1(1) + xMin, yMin, xMax, yMax (8 bytes)
    clip_list.push(1);
    push_i16(&mut clip_list, 0);
    push_i16(&mut clip_list, 0);
    push_i16(&mut clip_list, 100);
    push_i16(&mut clip_list, 100);

    // COLR v1 header = 34 bytes (v0 14 bytes + 5 * 4 = 20 bytes)
    let mut colr = Vec::new();
    push_u16(&mut colr, 1); // version = 1
    push_u16(&mut colr, 0);
    push_u32(&mut colr, 0);
    push_u32(&mut colr, 0);
    push_u16(&mut colr, 0);
    let base_off = 34u32;
    let layer_off = base_off + base_glyph_list.len() as u32;
    let clip_off = layer_off + layer_list.len() as u32;
    push_u32(&mut colr, base_off);
    push_u32(&mut colr, layer_off);
    push_u32(&mut colr, clip_off);
    push_u32(&mut colr, 0); // varIdxMapOffset
    push_u32(&mut colr, 0); // itemVariationStoreOffset
    colr.extend_from_slice(&base_glyph_list);
    colr.extend_from_slice(&layer_list);
    colr.extend_from_slice(&clip_list);
    colr
}

fn build_colr_v1_font() -> Vec<u8> {
    let mut builder = SfntBuilder::new();
    add_base_tables(
        &mut builder,
        7,
        &[
            ('A' as u16, 1),
            ('B' as u16, 2),
            ('C' as u16, 3),
            ('D' as u16, 4),
            ('E' as u16, 5),
            ('F' as u16, 6),
        ],
    );
    builder.push_table(*b"CPAL", build_cpal_v1());
    builder.push_table(*b"COLR", build_colr_v1_comprehensive());
    builder.build()
}

#[test]
fn colr_v1_layers_transforms_and_gradients_render() {
    let font_bytes = build_colr_v1_font();
    let file = FontFile::from_buffer(&font_bytes).unwrap();
    let face = file.face(0).unwrap();
    let engine = face.engine().with_font_size(1000.0);

    // 'A': PaintColrLayers + PaintSolid + PaintTranslate(20, 40) + CurrentColor
    let run_a = engine.shape("A").unwrap();
    assert_eq!(run_a.glyphs[0].glyph.layers.len(), 2);
    let GlyphLayer::Path(l0) = &run_a.glyphs[0].glyph.layers[0] else {
        panic!("expected path layer");
    };
    assert_eq!(l0.paint, GlyphPaint::Solid(0xFFFF_0000));
    assert!(!l0.commands.is_empty());

    let GlyphLayer::Path(l1) = &run_a.glyphs[0].glyph.layers[1] else {
        panic!("expected translated path layer");
    };
    assert_eq!(l1.paint, GlyphPaint::CurrentColor);
    // Original rect gid 2 is (10,10)-(60,60). Translated by (+20, +40) in font coords =>
    // first point x = 30, y = -(10 + 40) = -50.
    assert!(matches!(
        l1.commands.first(),
        Some(Command::MoveTo(30.0, -50.0))
    ));

    // 'B': LinearGradient Pad + ClipBox [0,0,100,100] -> MoveTo(0, 0)..Close
    let run_b = engine.shape("B").unwrap();
    assert_eq!(run_b.glyphs[0].glyph.layers.len(), 1);
    let GlyphLayer::Path(lb) = &run_b.glyphs[0].glyph.layers[0] else {
        panic!("expected gradient layer");
    };
    let GlyphPaint::LinearGradient(ref lg) = lb.paint else {
        panic!("expected LinearGradient, got {:?}", lb.paint);
    };
    assert_eq!(lg.spread, GlyphGradientSpread::Pad);
    assert_eq!(lg.stops.len(), 2);
    assert_eq!(lg.stops[0].color, 0xFFFF_0000);
    assert_eq!(lg.stops[1].color, 0xFF00_00FF);
    assert_eq!(lb.clip_commands.len(), 5);

    // 'C': RadialGradient Reflect
    let run_c = engine.shape("C").unwrap();
    let GlyphLayer::Path(lc) = &run_c.glyphs[0].glyph.layers[0] else {
        panic!("expected radial gradient layer");
    };
    let GlyphPaint::RadialGradient(ref rg) = lc.paint else {
        panic!("expected RadialGradient, got {:?}", lc.paint);
    };
    assert_eq!(rg.spread, GlyphGradientSpread::Reflect);
    assert_eq!(rg.stops[0].color, 0xFF00_00FF);
    assert_eq!(rg.stops[1].color, 0xFF00_FF00);

    // 'D': PaintColrGlyph referencing 'A' produces identical 2 layers
    let run_d = engine.shape("D").unwrap();
    assert_eq!(run_d.glyphs[0].glyph.layers.len(), 2);

    // 'E': cyclic PaintColrGlyph terminates safely without overflow
    let run_e = engine.shape("E").unwrap();
    assert!(run_e.glyphs[0].glyph.layers.len() <= 64);

    // SVG rendering handles LinearGradient & Solid cleanly
    let svg = engine.render_svg("AB").unwrap();
    assert!(svg.contains("<svg"), "{svg}");
    assert!(svg.contains("#ff0000"), "{svg}");
}
