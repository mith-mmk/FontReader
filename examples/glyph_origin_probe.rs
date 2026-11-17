//! JSON path probe for opt-in unhinted glyph-origin comparisons.
use fontcore::{Command, FontFile, GlyphLayer, ShapingPolicy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: glyph_origin_probe FONT FACE SIZE ltr|ttb TEXT".into());
    }
    let file = FontFile::from_file(&args[1])?;
    let face = file.face(args[2].parse()?)?;
    let policy = match args[4].as_str() {
        "ltr" => ShapingPolicy::LeftToRight,
        "ttb" => ShapingPolicy::TopToBottom,
        _ => return Err("direction must be ltr or ttb".into()),
    };
    let shaped = face
        .engine()
        .with_font_size(args[3].parse()?)
        .with_shaping_policy(policy)
        .shape_mapped(&args[5])?;
    print!("{{\"glyphs\":[");
    for (i, p) in shaped.glyph_run().glyphs.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        let range = shaped.glyph_sources()[i].byte_range();
        print!(
            "{{\"source\":[{},{}],\"advance\":[{},{}],\"layers\":[",
            range.start, range.end, p.glyph.metrics.advance_x, p.glyph.metrics.advance_y
        );
        for (j, layer) in p.glyph.layers.iter().enumerate() {
            if j > 0 {
                print!(",");
            }
            let GlyphLayer::Path(layer) = layer else {
                return Err("path-only probe".into());
            };
            print!(
                "{{\"origin\":[{},{}],\"commands\":[",
                p.x + layer.offset_x,
                p.y + layer.offset_y
            );
            for (k, c) in layer.commands.iter().enumerate() {
                if k > 0 {
                    print!(",");
                }
                match c {
                    Command::MoveTo(x, y) => print!("[\"M\",{x},{y}]"),
                    Command::Line(x, y) => print!("[\"L\",{x},{y}]"),
                    Command::Bezier((cx, cy), (x, y)) => print!("[\"Q\",{cx},{cy},{x},{y}]"),
                    Command::CubicBezier((a, b), (c, d), (x, y)) => {
                        print!("[\"C\",{a},{b},{c},{d},{x},{y}]")
                    }
                    Command::Close => print!("[\"Z\"]"),
                }
            }
            print!("]}}");
        }
        print!("]}}");
    }
    println!("]}}");
    Ok(())
}
