//! Machine-readable public mapped-shaping/caret probe for compare_gdef_carets.py.
use fontcore::{CaretAffinity, CaretLayout, FontFile, ShapingPolicy, TextPosition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("usage: gdef_carets FONT FACE SIZE ltr|rtl|ttb WEIGHT|- ASCII-TEXT".into());
    }
    let file = FontFile::from_file(&args[1])?;
    let face = file.face(args[2].parse()?)?;
    let direction = match args[4].as_str() {
        "ltr" => ShapingPolicy::LeftToRight,
        "rtl" => ShapingPolicy::RightToLeft,
        "ttb" => ShapingPolicy::TopToBottom,
        _ => return Err("direction must be ltr, rtl or ttb".into()),
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
            "{{\"source\":[{},{}],\"x\":{},\"y\":{},\"advance_xy\":[{},{}],\"candidates\":",
            range.start, range.end, p.x, p.y, p.glyph.metrics.advance_x, p.glyph.metrics.advance_y
        );
        match &shaped.glyph_ligature_carets()[i] {
            Some(values) => print!("{values:?}"),
            None => print!("null"),
        }
        print!(",\"bounds\":");
        if let Some(b) = p.glyph.metrics.bounds {
            print!(
                "[{},{},{},{}]",
                p.x + b.min_x,
                p.y + b.min_y,
                p.x + b.max_x,
                p.y + b.max_y
            );
        } else {
            print!("null");
        }
        print!("}}");
    }
    print!("],\"stops\":[");
    let layout = CaretLayout::new(&shaped);
    if let Err(error) = &layout {
        println!("] ,\"layout_error\":{:?}}}", error.to_string());
        return Ok(());
    }
    let layout = layout?;
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
