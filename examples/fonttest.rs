use fontloader::Font;
use std::path::PathBuf;

fn main() {
    // agrs[1] is the folder name
    let args: Vec<String> = std::env::args().collect();
    // argv.len()?
    let fontname = if args.len() >= 2 {
        args[1].to_string()
    } else {
        "e:\\data\\fonts\\NotoSansJP-SemiBold.ttf".to_string()
    };

    let filename: PathBuf = PathBuf::from(fontname);
    let font = Font::get_font_from_file(&filename).unwrap();
    print!("font: {:?}", font);
    let text = "Hello, world!";
    for ch in text.chars() {
        let glyph = font.get_gryph(ch);
        println!("glyph: {:?}", glyph);
    }
}
