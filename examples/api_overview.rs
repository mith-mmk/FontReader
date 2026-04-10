mod common;

use common::{font_path, output_path, text_content, variant_name, vertical_enabled};
use fontcore::{FontFile, FontVariant, ShapingPolicy};

fn apply_variant(
    engine: fontcore::FontEngine<'_>,
    variant: Option<String>,
) -> fontcore::FontEngine<'_> {
    match variant.as_deref() {
        Some("jp78") => engine.with_font_variant(FontVariant::Jis78),
        Some("jp90") => engine.with_font_variant(FontVariant::Jis90),
        Some("trad") => engine.with_font_variant(FontVariant::TraditionalForms),
        Some("nlck") => engine.with_font_variant(FontVariant::NlcKanjiForms),
        _ => engine,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let filename = font_path(&args);
    let output_file = output_path(&args, "./test/api_overview.html");
    let text = text_content(&args, "./test/read.txt")?;
    let is_vertical = vertical_enabled(&args);
    let variant = variant_name(&args);

    let file = FontFile::from_file(&filename)?;
    let face = file.current_face()?;
    let engine = apply_variant(
        face.engine()
            .with_shaping_policy(ShapingPolicy::LeftToRight)
            .with_font_size(64.0)
            .with_line_height(72.0)
            .with_svg_unit("px"),
        variant.clone(),
    );
    let engine = if is_vertical {
        engine.with_vertical_flow()
    } else {
        engine
    };

    let run = engine.shape(&text)?;
    let width = engine.measure(&text)?;
    let svg = if is_vertical {
        engine.render_svg_vertical(&text)?
    } else {
        engine.render_svg(&text)?
    };

    let mut html = String::from(
        "<html>\n<head>\n<meta charset=\"UTF-8\">\n<title>fontcore api</title>\n</head>\n<body>\n",
    );
    html.push_str(&format!("<pre>{}</pre>\n", face.dump()));
    html.push_str(&format!(
        "<p>direction: {}</p>\n",
        if is_vertical {
            "vertical"
        } else {
            "horizontal"
        }
    ));
    html.push_str(&format!(
        "<p>variant: {}</p>\n",
        variant.as_deref().unwrap_or("normal")
    ));
    html.push_str(&format!("<p>measure: {:.2}px</p>\n", width));
    html.push_str(&format!("<p>glyphs: {}</p>\n", run.glyphs.len()));
    html.push_str(&svg);
    html.push_str("\n</body>\n</html>\n");
    std::fs::write(output_file, html)?;
    Ok(())
}
