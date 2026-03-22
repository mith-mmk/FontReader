mod common;

use common::font_path;
use fontloader::Font;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = font_path(&args);
    let font = Font::get_font_from_file(&filename).unwrap();

    print!("font: {:?}", font);
}
