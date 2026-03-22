mod common;

use common::{font_index, font_path, output_path, text_content};
use fontloader::Font;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let filename = font_path(&args);
    let output_file = output_path(&args, "./test/read.html");
    print!("{:?}, output filename:{:?}", filename, output_file);
    let mut font = Font::get_font_from_file(&filename).unwrap();
    let font_count = font.get_font_count();
    if font_count > 0 {
        let index = font_index(&args, 1).min(font_count.saturating_sub(1));
        font.set_font(index).unwrap();
    }

    let string = font.get_info()?;
    println!("{}", string);
    let string = text_content(&args, "./test/read.txt")?;
    match font.get_html(&string, 64.0, "px") {
        Ok(html) => std::fs::write(output_file, html)?,
        Err(err) => eprintln!("render skipped: {}", err),
    }
    Ok(())
}
