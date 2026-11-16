//! External-font diagnostic. Usage: FONT TEXT FONT_SIZE [ttb].
use fontcore::{FontFile, ShapingPolicy};
fn main() {
    let a: Vec<_> = std::env::args().collect();
    let f = FontFile::from_file(&a[1]).unwrap();
    let face = f.face(0).unwrap();
    let policy = if a.len() > 4 {
        ShapingPolicy::TopToBottom
    } else {
        ShapingPolicy::LeftToRight
    };
    let r = face
        .engine()
        .with_font_size(a[3].parse().expect("font size"))
        .with_locale("ko")
        .with_shaping_policy(policy)
        .shape(&a[2])
        .unwrap();
    for g in r.glyphs {
        let b = g.glyph.metrics.bounds.unwrap();
        println!(
            "[{}, {}, {}, {}, {}, {}]",
            g.x + b.min_x,
            g.y + b.min_y,
            g.x + b.max_x,
            g.y + b.max_y,
            g.glyph.metrics.advance_x,
            g.glyph.metrics.advance_y
        );
    }
}
