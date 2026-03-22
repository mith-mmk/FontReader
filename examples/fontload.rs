mod common;

use common::{font_index, font_path, output_path};
use fontloader::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let filename = font_path(&args);
    let output_file = output_path(&args, "./test/loader.html");
    let mut font = Font::get_font_from_file(&filename).unwrap();
    let font_count = font.get_font_count();
    if font_count > 0 {
        let index = font_index(&args, font_count.saturating_sub(1)).min(font_count.saturating_sub(1));
        font.set_font(index).unwrap();
    }

    let string = font.get_info()?;
    println!("{}", string);
    let mut html = "<html><head><meta charset=\"utf-8\"></head><body>".to_string();
    for i in 0..65535 {
        let j = i;
        let result = font.get_svg_from_id(j, 32.0, "px");
        if result.is_err() {
            break;
        }
        html += result.unwrap().as_str();
    }
    html += "</body></html>";
    std::fs::write(output_file, html)?;
    Ok(())
}
