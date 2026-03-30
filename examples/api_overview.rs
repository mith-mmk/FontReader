mod common;

use common::{font_path, output_path, text_content};
use fontloader::FontFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let filename = font_path(&args);
    let output_file = output_path(&args, "./test/api_overview.html");
    let text = text_content(&args, "./test/read.txt")?;

    let file = FontFile::from_file(&filename)?;
    let face = file.current_face()?;
    let engine = face
        .engine()
        .with_font_size(64.0)
        .with_line_height(72.0)
        .with_svg_unit("px");

    let run = engine.shape(&text)?;
    let width = engine.measure(&text)?;
    let svg = engine.render_svg(&text)?;

    let mut html = String::from(
        "<html>\n<head>\n<meta charset=\"UTF-8\">\n<title>fontloader api</title>\n</head>\n<body>\n",
    );
    html.push_str(&format!("<pre>{}</pre>\n", face.dump()));
    html.push_str(&format!("<p>measure: {:.2}px</p>\n", width));
    html.push_str(&format!("<p>glyphs: {}</p>\n", run.glyphs.len()));
    html.push_str(&svg);
    html.push_str("\n</body>\n</html>\n");
    std::fs::write(output_file, html)?;
    Ok(())
}
