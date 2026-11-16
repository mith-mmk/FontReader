//! Machine-readable public mapped-shaping/caret probe for compare_gdef_carets.py.
use fontcore::{CaretAffinity, CaretLayout, FontFile, ShapingPolicy, TextPosition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("usage: gdef_carets FONT FACE SIZE ltr|rtl WEIGHT|- ASCII-TEXT".into());
    }
    let file = FontFile::from_file(&args[1])?;
    let face = file.face(args[2].parse()?)?;
    let direction = match args[4].as_str() {
        "ltr" => ShapingPolicy::LeftToRight,
        "rtl" => ShapingPolicy::RightToLeft,
        _ => return Err("direction must be ltr or rtl".into()),
    };
    let text = &args[6];
    if !text.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err("probe accepts ASCII alphabetic text only".into());
    }
    let mut engine = face
        .engine()
        .with_font_size(args[3].parse()?)
        .with_shaping_policy(direction);
    if args[5] != "-" {
        engine = engine.with_variation("wght", args[5].parse()?);
    }
    let shaped = engine.shape_mapped(text)?;
    print!("{{\"glyphs\":[");
    for (i, p) in shaped.glyph_run().glyphs.iter().enumerate() {
        if i != 0 {
            print!(",");
        }
        let range = shaped.glyph_sources()[i].byte_range();
        print!(
            "{{\"source\":[{},{}],\"x\":{},\"y\":{},\"advance\":{},\"candidates\":",
            range.start, range.end, p.x, p.y, p.glyph.metrics.advance_x
        );
        match &shaped.glyph_ligature_carets()[i] {
            Some(values) => print!("{values:?}"),
            None => print!("null"),
        }
        print!("}}");
    }
    print!("],\"stops\":[");
    let layout = CaretLayout::new(&shaped)?;
    for offset in 0..=text.len() {
        if offset != 0 {
            print!(",");
        }
        let caret =
            layout.caret_positions(TextPosition::new(offset, CaretAffinity::Downstream))?[0];
        print!(
            "{{\"byte\":{},\"x\":{},\"y\":{},\"quality\":\"{:?}\",\"source\":\"{:?}\"}}",
            offset,
            caret.x(),
            caret.y(),
            caret.quality(),
            caret.source()
        );
    }
    println!("]}}");
    Ok(())
}
