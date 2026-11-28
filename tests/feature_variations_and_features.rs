//! Synthetic GSUB/GPOS v1.1 FeatureVariations and custom OpenType feature regressions.

use fontcore::FontFeatureSetting;
#[cfg(feature = "layout")]
use fontcore::{FontFile, FontVariant};

#[cfg(feature = "layout")]
struct SfntBuilder {
    tables: Vec<([u8; 4], Vec<u8>)>,
}

#[cfg(feature = "layout")]
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

#[cfg(feature = "layout")]
fn calc_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut buf = [0u8; 4];
        buf[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(buf));
    }
    sum
}

#[cfg(feature = "layout")]
fn push_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

#[cfg(feature = "layout")]
fn push_i16(buf: &mut Vec<u8>, v: i16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

#[cfg(feature = "layout")]
fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_be_bytes());
}

#[cfg(feature = "layout")]
fn push_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_be_bytes());
}

#[cfg(feature = "layout")]
fn make_rect_glyph(x1: i16) -> Vec<u8> {
    let mut buf = Vec::new();
    push_i16(&mut buf, 1);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, x1);
    push_i16(&mut buf, 500);
    push_u16(&mut buf, 3);
    push_u16(&mut buf, 0);
    buf.extend_from_slice(&[0x01, 0x01, 0x01, 0x01]);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, x1);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, -x1);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, 0);
    push_i16(&mut buf, 500);
    push_i16(&mut buf, 0);
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
    buf
}

#[cfg(feature = "layout")]
fn add_variable_base_tables(builder: &mut SfntBuilder) {
    // Glyphs: 0=notdef, 1='a' (500), 2='b' (520), 3='A' (700), 4='B' (720),
    //         5='a_b' ligature (900), 6='a_bold' (600)
    let advances = [500u16, 500, 520, 700, 720, 900, 600];
    let num_glyphs = advances.len() as u16;

    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[40..42].copy_from_slice(&1000i16.to_be_bytes());
    head[42..44].copy_from_slice(&1000i16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    builder.push_table(*b"head", head);

    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
    hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
    hhea[10..12].copy_from_slice(&900u16.to_be_bytes());
    hhea[34..36].copy_from_slice(&num_glyphs.to_be_bytes());
    builder.push_table(*b"hhea", hhea);

    let mut maxp = vec![0u8; 32];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());
    builder.push_table(*b"maxp", maxp);

    let mut hmtx = Vec::new();
    for adv in advances {
        push_u16(&mut hmtx, adv);
        push_i16(&mut hmtx, 0);
    }
    builder.push_table(*b"hmtx", hmtx);

    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    push_u32(&mut loca, 0);
    for adv in advances.iter().skip(1) {
        push_u32(&mut loca, glyf.len() as u32);
        glyf.extend_from_slice(&make_rect_glyph(*adv as i16));
    }
    push_u32(&mut loca, glyf.len() as u32);
    builder.push_table(*b"glyf", glyf);
    builder.push_table(*b"loca", loca);

    // cmap: 'A'->3, 'B'->4, 'a'->1, 'b'->2 (must be sorted by codepoint)
    let entries: [(u16, u16); 4] = [
        ('A' as u16, 3),
        ('B' as u16, 4),
        ('a' as u16, 1),
        ('b' as u16, 2),
    ];
    let seg_count = (entries.len() + 1) as u16;
    let seg_count_x2 = seg_count * 2;
    let max_pow2 =
        (seg_count as u32).next_power_of_two() >> if seg_count.is_power_of_two() { 0 } else { 1 };
    let search_range = (max_pow2.max(1) * 2) as u16;
    let entry_selector = max_pow2.max(1).trailing_zeros() as u16;
    let range_shift = seg_count_x2 - search_range;

    let mut subtable = Vec::new();
    push_u16(&mut subtable, 4);
    push_u16(&mut subtable, 16 + seg_count * 8);
    push_u16(&mut subtable, 0);
    push_u16(&mut subtable, seg_count_x2);
    push_u16(&mut subtable, search_range);
    push_u16(&mut subtable, entry_selector);
    push_u16(&mut subtable, range_shift);
    for &(ch, _) in &entries {
        push_u16(&mut subtable, ch);
    }
    push_u16(&mut subtable, 0xFFFF);
    push_u16(&mut subtable, 0);
    for &(ch, _) in &entries {
        push_u16(&mut subtable, ch);
    }
    push_u16(&mut subtable, 0xFFFF);
    for &(ch, gid) in &entries {
        push_i16(&mut subtable, gid.wrapping_sub(ch) as i16);
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
    let label = "VarTest";
    push_u16(&mut name, 0);
    push_u16(&mut name, 1);
    push_u16(&mut name, 18);
    push_u16(&mut name, 3);
    push_u16(&mut name, 1);
    push_u16(&mut name, 0x0409);
    push_u16(&mut name, 1);
    push_u16(&mut name, (label.len() * 2) as u16);
    push_u16(&mut name, 0);
    for ch in label.encode_utf16() {
        push_u16(&mut name, ch);
    }
    builder.push_table(*b"name", name);

    // fvar with wght [400..900] default 400
    let mut fvar = Vec::new();
    push_u16(&mut fvar, 1);
    push_u16(&mut fvar, 0);
    push_u16(&mut fvar, 16);
    push_u16(&mut fvar, 2);
    push_u16(&mut fvar, 1);
    push_u16(&mut fvar, 20);
    push_u16(&mut fvar, 0);
    push_u16(&mut fvar, 4);
    fvar.extend_from_slice(b"wght");
    push_i32(&mut fvar, 400 << 16);
    push_i32(&mut fvar, 400 << 16);
    push_i32(&mut fvar, 900 << 16);
    push_u16(&mut fvar, 0);
    push_u16(&mut fvar, 256);
    builder.push_table(*b"fvar", fvar);
}

#[cfg(feature = "layout")]
fn build_coverage_format1(glyphs: &[u16]) -> Vec<u8> {
    let mut buf = Vec::new();
    push_u16(&mut buf, 1);
    push_u16(&mut buf, glyphs.len() as u16);
    for &g in glyphs {
        push_u16(&mut buf, g);
    }
    buf
}

#[cfg(feature = "layout")]
fn build_feature_variations_table(feature_index: u16, lookup_index: u16) -> Vec<u8> {
    // FeatureVariations:
    // major=1, minor=0, recordCount=1 -> 8 bytes
    // record[0]: condSetOffset=16, featSubstOffset=30 -> 8 bytes (total 16)
    // ConditionSet @16: count=1 (2), offset[0]=4 (4) -> 6 bytes; ConditionFormat1 @22: format=1(2), axis=0(2), min=0x2000(2), max=0x4000(2) -> 8 bytes (total 14 -> ends @30)
    // FeatureTableSubstitution @30: major=1, minor=0, substCount=1 -> 6 bytes; record: featIdx(2), altFeatOff=12(4) -> 6 bytes (ends @42)
    // Alternate Feature @42: featParams=0, lookupCount=1, lookup[0]=lookup_index -> 6 bytes
    let mut fv = Vec::new();
    push_u16(&mut fv, 1); // major
    push_u16(&mut fv, 0); // minor
    push_u32(&mut fv, 1); // featureVariationRecordCount
    push_u32(&mut fv, 16); // conditionSetOffset
    push_u32(&mut fv, 30); // featureTableSubstitutionOffset

    // ConditionSet @16
    push_u16(&mut fv, 1); // conditionCount
    push_u32(&mut fv, 6); // offset from ConditionSet start
                          // ConditionFormat1 @22
    push_u16(&mut fv, 1);
    push_u16(&mut fv, 0); // axisIndex 0 (wght)
    push_i16(&mut fv, 0x2000); // min = 0.5
    push_i16(&mut fv, 0x4000); // max = 1.0

    // FeatureTableSubstitution @30
    push_u16(&mut fv, 1); // major
    push_u16(&mut fv, 0); // minor
    push_u16(&mut fv, 1); // substitutionCount
    push_u16(&mut fv, feature_index);
    push_u32(&mut fv, 12); // alternateFeatureOffset from FeatureTableSubstitution start

    // Alternate Feature @42
    push_u16(&mut fv, 0);
    push_u16(&mut fv, 1);
    push_u16(&mut fv, lookup_index);
    fv
}

#[cfg(feature = "layout")]
fn build_gsub_v11() -> Vec<u8> {
    // ScriptList: DFLT -> features [0 ('calt'), 1 ('liga'), 2 ('smcp')]
    let mut script_list = Vec::new();
    push_u16(&mut script_list, 1); // scriptCount
    script_list.extend_from_slice(b"DFLT");
    push_u16(&mut script_list, 8); // scriptOffset from ScriptList
                                   // Script @8: defaultLangSys=4, langSysCount=0
    push_u16(&mut script_list, 4);
    push_u16(&mut script_list, 0);
    // LangSys @12: lookupOrder=0, req=0xFFFF, count=3, [0, 1, 2]
    push_u16(&mut script_list, 0);
    push_u16(&mut script_list, 0xFFFF);
    push_u16(&mut script_list, 3);
    push_u16(&mut script_list, 0);
    push_u16(&mut script_list, 1);
    push_u16(&mut script_list, 2);

    // FeatureList: 0='calt' (no lookups initially), 1='liga' (lookup 0), 2='smcp' (lookup 1)
    let mut feat_list = Vec::new();
    push_u16(&mut feat_list, 3); // featureCount = 3 -> records take 3*6 = 18 -> starts at 20
    feat_list.extend_from_slice(b"calt");
    push_u16(&mut feat_list, 20);
    feat_list.extend_from_slice(b"liga");
    push_u16(&mut feat_list, 24);
    feat_list.extend_from_slice(b"smcp");
    push_u16(&mut feat_list, 30);
    // feat 0 @20 (4 bytes): empty
    push_u16(&mut feat_list, 0);
    push_u16(&mut feat_list, 0);
    // feat 1 @24 (6 bytes): lookup 0
    push_u16(&mut feat_list, 0);
    push_u16(&mut feat_list, 1);
    push_u16(&mut feat_list, 0);
    // feat 2 @30 (6 bytes): lookup 1
    push_u16(&mut feat_list, 0);
    push_u16(&mut feat_list, 1);
    push_u16(&mut feat_list, 1);

    // Subtable Lookup 0: LigatureSubst Format 1 ('a'=1 + 'b'=2 -> gid 5)
    let mut sub0 = Vec::new();
    let cov0 = build_coverage_format1(&[1]);
    // header: format(2)=1, covOff(2)=8, setCount(2)=1, setOff[0](2)=8+cov.len()
    push_u16(&mut sub0, 1);
    push_u16(&mut sub0, 8);
    push_u16(&mut sub0, 1);
    push_u16(&mut sub0, 8 + cov0.len() as u16);
    sub0.extend_from_slice(&cov0);
    // LigatureSet: count=1, ligOff[0]=4 -> header 4 bytes
    push_u16(&mut sub0, 1);
    push_u16(&mut sub0, 4);
    // Ligature: ligGlyph=5, compCount=2, comp[0]=2
    push_u16(&mut sub0, 5);
    push_u16(&mut sub0, 2);
    push_u16(&mut sub0, 2);

    // Subtable Lookup 1: SingleSubst Format 2 ('a'=1 -> 'A'=3)
    let mut sub1 = Vec::new();
    let cov1 = build_coverage_format1(&[1]);
    push_u16(&mut sub1, 2);
    push_u16(&mut sub1, 8);
    push_u16(&mut sub1, 1);
    push_u16(&mut sub1, 3);
    sub1.extend_from_slice(&cov1);

    // Subtable Lookup 2: SingleSubst Format 2 ('a'=1 -> gid 6 'a_bold') used by FeatureVariations!
    let mut sub2 = Vec::new();
    let cov2 = build_coverage_format1(&[1]);
    push_u16(&mut sub2, 2);
    push_u16(&mut sub2, 8);
    push_u16(&mut sub2, 1);
    push_u16(&mut sub2, 6);
    sub2.extend_from_slice(&cov2);

    // Lookups: 3 lookups -> lookupList header = 2 + 3*2 = 8 bytes
    let wrap_lookup = |subtable: Vec<u8>, ltype: u16| -> Vec<u8> {
        let mut l = Vec::new();
        push_u16(&mut l, ltype);
        push_u16(&mut l, 0); // flag
        push_u16(&mut l, 1); // subTableCount
        push_u16(&mut l, 8); // offset from Lookup
        l.extend_from_slice(&subtable);
        l
    };
    let l0 = wrap_lookup(sub0, 4);
    let l1 = wrap_lookup(sub1, 1);
    let l2 = wrap_lookup(sub2, 1);

    let mut lookup_list = Vec::new();
    push_u16(&mut lookup_list, 3);
    let o0 = 8u16;
    let o1 = o0 + l0.len() as u16;
    let o2 = o1 + l1.len() as u16;
    push_u16(&mut lookup_list, o0);
    push_u16(&mut lookup_list, o1);
    push_u16(&mut lookup_list, o2);
    lookup_list.extend_from_slice(&l0);
    lookup_list.extend_from_slice(&l1);
    lookup_list.extend_from_slice(&l2);

    // FeatureVariations swaps feature 0 ('calt') to lookup 2
    let feat_vars = build_feature_variations_table(0, 2);

    let script_off = 14u16;
    let feat_off = script_off + script_list.len() as u16;
    let lookup_off = feat_off + feat_list.len() as u16;
    let fv_off = lookup_off as u32 + lookup_list.len() as u32;

    let mut gsub = Vec::new();
    push_u16(&mut gsub, 1);
    push_u16(&mut gsub, 1);
    push_u16(&mut gsub, script_off);
    push_u16(&mut gsub, feat_off);
    push_u16(&mut gsub, lookup_off);
    push_u32(&mut gsub, fv_off);
    gsub.extend_from_slice(&script_list);
    gsub.extend_from_slice(&feat_list);
    gsub.extend_from_slice(&lookup_list);
    gsub.extend_from_slice(&feat_vars);
    gsub
}

#[cfg(feature = "layout")]
fn build_gpos_pair_format1(first: u16, second: u16, x_adv: i16) -> Vec<u8> {
    // PairPos Format 1:
    // format=1(2), covOff=10(2), vf1=4(2: xAdvance), vf2=0(2), pairSetCount=1(2) -> 10 bytes
    // Coverage @10: 6 bytes -> ends @16
    // PairSet[0] @16: pairValueCount=1(2), secondGlyph(2), value1.xAdvance(2) = 6 bytes
    let mut sub = Vec::new();
    let cov = build_coverage_format1(&[first]);
    push_u16(&mut sub, 1);
    push_u16(&mut sub, 12); // coverage after pairSetOffset[0]
    push_u16(&mut sub, 0x0004); // valueFormat1 = X_ADVANCE
    push_u16(&mut sub, 0); // valueFormat2
    push_u16(&mut sub, 1); // pairSetCount
    push_u16(&mut sub, 12 + cov.len() as u16); // pairSetOffsets[0]
    sub.extend_from_slice(&cov);
    push_u16(&mut sub, 1); // pairValueCount
    push_u16(&mut sub, second);
    push_i16(&mut sub, x_adv);
    sub
}

#[cfg(feature = "layout")]
fn build_gpos_v11() -> Vec<u8> {
    let mut script_list = Vec::new();
    push_u16(&mut script_list, 1);
    script_list.extend_from_slice(b"DFLT");
    push_u16(&mut script_list, 8);
    push_u16(&mut script_list, 4);
    push_u16(&mut script_list, 0);
    push_u16(&mut script_list, 0);
    push_u16(&mut script_list, 0xFFFF);
    push_u16(&mut script_list, 1);
    push_u16(&mut script_list, 0);

    // FeatureList: 0='kern' -> lookup 0
    let mut feat_list = Vec::new();
    push_u16(&mut feat_list, 1);
    feat_list.extend_from_slice(b"kern");
    push_u16(&mut feat_list, 8);
    push_u16(&mut feat_list, 0);
    push_u16(&mut feat_list, 1);
    push_u16(&mut feat_list, 0);

    let wrap_lookup = |sub: Vec<u8>| -> Vec<u8> {
        let mut l = Vec::new();
        push_u16(&mut l, 2); // PairAdjustment
        push_u16(&mut l, 0);
        push_u16(&mut l, 1);
        push_u16(&mut l, 8);
        l.extend_from_slice(&sub);
        l
    };
    let l0 = wrap_lookup(build_gpos_pair_format1(3, 4, -120));
    let l1 = wrap_lookup(build_gpos_pair_format1(3, 4, -300));

    let mut lookup_list = Vec::new();
    push_u16(&mut lookup_list, 2);
    let o0 = 6u16;
    let o1 = o0 + l0.len() as u16;
    push_u16(&mut lookup_list, o0);
    push_u16(&mut lookup_list, o1);
    lookup_list.extend_from_slice(&l0);
    lookup_list.extend_from_slice(&l1);

    let feat_vars = build_feature_variations_table(0, 1);

    let script_off = 14u16;
    let feat_off = script_off + script_list.len() as u16;
    let lookup_off = feat_off + feat_list.len() as u16;
    let fv_off = lookup_off as u32 + lookup_list.len() as u32;

    let mut gpos = Vec::new();
    push_u16(&mut gpos, 1);
    push_u16(&mut gpos, 1);
    push_u16(&mut gpos, script_off);
    push_u16(&mut gpos, feat_off);
    push_u16(&mut gpos, lookup_off);
    push_u32(&mut gpos, fv_off);
    gpos.extend_from_slice(&script_list);
    gpos.extend_from_slice(&feat_list);
    gpos.extend_from_slice(&lookup_list);
    gpos.extend_from_slice(&feat_vars);
    gpos
}

#[cfg(feature = "layout")]
fn build_test_font() -> Vec<u8> {
    let mut builder = SfntBuilder::new();
    add_variable_base_tables(&mut builder);
    builder.push_table(*b"GSUB", build_gsub_v11());
    builder.push_table(*b"GPOS", build_gpos_v11());
    builder.build()
}

#[test]
fn font_feature_setting_api_validates_tags() {
    let smcp = FontFeatureSetting::enabled("smcp").unwrap();
    assert_eq!(smcp.tag_string(), "smcp");
    assert_eq!(smcp.value, 1);

    let kern_off = FontFeatureSetting::disabled("kern").unwrap();
    assert_eq!(kern_off.value, 0);

    assert!(FontFeatureSetting::new("bad", 1).is_err());
    assert!(FontFeatureSetting::new("toolong", 1).is_err());
}

#[cfg(feature = "layout")]
#[test]
fn custom_features_override_defaults_and_variant() {
    let font_bytes = build_test_font();
    let file = FontFile::from_buffer(&font_bytes).unwrap();
    let face = file.face(0).unwrap();

    // Default enables `liga`: "ab" ligates into gid 5 (advance 900).
    let default_ab = face.engine().with_font_size(1000.0).shape("ab").unwrap();
    assert_eq!(default_ab.glyphs.len(), 1);
    assert_eq!(default_ab.glyphs[0].glyph.metrics.advance_x, 900.0);

    // Explicit `liga=0` suppresses ligature: 2 glyphs ('a' 500 + 'b' 520).
    let no_liga = face
        .engine()
        .with_font_size(1000.0)
        .with_feature("liga", 0)
        .shape("ab")
        .unwrap();
    assert_eq!(no_liga.glyphs.len(), 2);
    assert_eq!(no_liga.glyphs[0].glyph.metrics.advance_x, 500.0);
    assert_eq!(no_liga.glyphs[1].glyph.metrics.advance_x, 520.0);

    // `with_small_caps()` (or `FontVariant::SmallCaps`) + `with_feature("smcp", 1)` substitutes 'a' -> 'A' (700).
    let sc = face
        .engine()
        .with_font_size(1000.0)
        .with_small_caps()
        .shape("a")
        .unwrap();
    assert_eq!(sc.glyphs[0].glyph.metrics.advance_x, 700.0);

    let sc_variant = face
        .engine()
        .with_font_size(1000.0)
        .with_font_variant(FontVariant::SmallCaps)
        .with_feature("smcp", 0)
        .shape("a")
        .unwrap();
    assert_eq!(sc_variant.glyphs[0].glyph.metrics.advance_x, 500.0);

    let explicit_smcp = face
        .engine()
        .with_font_size(1000.0)
        .with_feature("smcp", 1)
        .shape("a")
        .unwrap();
    assert_eq!(explicit_smcp.glyphs[0].glyph.metrics.advance_x, 700.0);

    // Default `kern` adjusts "AB" by -120 (700 - 120 = 580); `kern=0` gives 700.
    let kern_on = face.engine().with_font_size(1000.0).shape("AB").unwrap();
    assert_eq!(kern_on.glyphs[1].x, 580.0);

    let kern_off = face
        .engine()
        .with_font_size(1000.0)
        .with_feature("kern", 0)
        .shape("AB")
        .unwrap();
    assert_eq!(kern_off.glyphs[1].x, 700.0);
}

#[cfg(feature = "layout")]
#[test]
fn feature_variations_switch_gsub_and_gpos_by_axis_coords() {
    let font_bytes = build_test_font();
    let file = FontFile::from_buffer(&font_bytes).unwrap();
    let face = file.face(0).unwrap();

    // At wght=400 (normalized 0.0), ConditionSet [0.5..1.0] does not match.
    let light_a = face
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 400.0)
        .shape("a")
        .unwrap();
    assert_eq!(light_a.glyphs[0].glyph.metrics.advance_x, 500.0);

    let light_ab = face
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 400.0)
        .shape("AB")
        .unwrap();
    assert_eq!(light_ab.glyphs[1].x, 580.0);

    // At wght=900 (normalized 1.0), GSUB 'calt' substitutes 'a' -> gid 6 (advance 600)
    // and GPOS 'kern' switches to -300 (700 - 300 = 400).
    let bold_a = face
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 900.0)
        .shape("a")
        .unwrap();
    assert_eq!(bold_a.glyphs[0].glyph.metrics.advance_x, 600.0);

    let bold_ab = face
        .engine()
        .with_font_size(1000.0)
        .with_variation("wght", 900.0)
        .shape("AB")
        .unwrap();
    assert_eq!(bold_ab.glyphs[1].x, 400.0);
}
