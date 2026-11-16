//! Read tab-separated id / direction / locale / hex-scalar rows from stdin.
//! Emit public glyph geometry in JSON lines for an external reference harness.
use fontcore::{FontFile, ShapingPolicy};
use std::io::{self, BufRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: corpus_geometry FONT SIZE".into());
    }
    let size: f32 = args[2].parse()?;
    let file = FontFile::from_file(&args[1])?;
    let face = file.current_face()?;
    for line in io::stdin().lock().lines() {
        let line = line?;
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4
            || !fields[0]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("expected safe-id, direction, locale, hex scalars".into());
        }
        let direction = match fields[1] {
            "ltr" => ShapingPolicy::LeftToRight,
            "rtl" => ShapingPolicy::RightToLeft,
            _ => return Err("only explicit horizontal ltr/rtl supported".into()),
        };
        let text: String = fields[3]
            .split_whitespace()
            .map(|s| {
                let n = u32::from_str_radix(s, 16)?;
                char::from_u32(n).ok_or("invalid scalar".into())
            })
            .collect::<Result<_, Box<dyn std::error::Error>>>()?;
        let engine = face
            .engine()
            .with_font_size(size)
            .with_locale(fields[2])
            .with_shaping_policy(direction);
        let run = engine.shape(&text)?;
        print!(
            "{{\"id\":\"{}\",\"advance\":{},\"glyphs\":[",
            fields[0],
            engine.measure(&text)?
        );
        for (i, p) in run.glyphs.iter().enumerate() {
            if i != 0 {
                print!(",");
            }
            let m = p.glyph.metrics;
            print!("{{\"advance\":{},\"bounds\":", m.advance_x);
            if let Some(b) = m.bounds {
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
        println!("]}}");
    }
    Ok(())
}
