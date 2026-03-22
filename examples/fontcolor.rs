mod common;

use common::font_path;
use fontloader::{opentype::NameID, Font};

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
            NameID::iter().into_iter().for_each(|name_id| {
                let name = font.get_name(name_id, &"ja".to_string());
                if !name.is_err() {
                    println!("{:?}: {:?}", name_id, name.unwrap());
                }
            });
            #[cfg(debug_assertions)]
            {
                println!("{}", font.get_header_raw());
                println!("{}", font.get_maxp_raw());
                println!("{}", font.get_hhea_raw());
                println!("{}", font.get_colr_raw());
                println!("{}", font.get_cpal_raw());
            }
        }
    }
}
