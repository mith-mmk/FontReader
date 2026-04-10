mod common;

use common::font_path;
use fontcore::Font;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = font_path(&args);
    let font = Font::get_font_from_file(&filename);
    if let Ok(mut font) = font {
        let font_number = font.get_font_count();
        println!("fontfile: {:?} {}", filename, font_number);
        for i in 0..font_number {
            font.set_font(i).unwrap();
            println!("\nfont number: {} ", i);
            #[cfg(feature = "layout")]
            {
                #[cfg(debug_assertions)]
                {
                    let gdef = font.get_gdef_raw();
                    println!("GDEF: {}", gdef);
                    let gsub = font.get_gsub_raw();
                    println!("GSUB: {}", gsub);
                }
            }
        }
    }
}
