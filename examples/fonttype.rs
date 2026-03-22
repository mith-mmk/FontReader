mod common;

use common::font_folder;
use fontloader::fontheader;

fn get_font_type(folder: &std::path::Path) {
    let dir = folder.read_dir().unwrap();
    let mut font_files = Vec::new();
    for filename in dir {
        let filename = filename.unwrap().path();
        font_files.push(filename);
    }
    let mut fonts = Vec::new();
    for mut font in font_files {
        // is dir?
        if font.is_dir() {
            continue;
        }
        let font_type = fontheader::get_font_type_from_file(&mut font);
        if let Err(e) = font_type {
            println!("font load error: {}", e);
            continue;
        }
        let font_type = font_type.unwrap();
        println!("filename: {:?}", font);
        println!("fonttype: {}", font_type.to_string());
        fonts.push(font_type);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let folder = font_folder(&args);
    get_font_type(folder.as_path());
}
