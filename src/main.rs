use std::path::{Path, PathBuf};

use fontreader::Font;
mod fontheader;
mod requires;
mod fontreader;
mod outline;

fn get_font_type(folder: &String) {
    let dir = Path::new(&folder).read_dir().unwrap();
    let mut font_files = Vec::new();
    for filename in dir {
        let filename = filename.unwrap().path();
        font_files.push(filename);
    }
    let mut fonts = Vec::new();
    for mut font in font_files {
        println!("fontfile: {:?}", font);
        let font_type = fontheader::get_font_type_from_file(&mut font);
        println!("fonttype: {}", font_type.to_string());
        fonts.push(font_type);
    }
}

fn main() {
    // agrs[1] is the folder name
    let args: Vec<String> = std::env::args().collect();
    // argv.len()?
    let fontname = if args.len() >= 2 {args[1].to_string()} else {
        "e:\\data\\fonts\\NotoSansJP-SemiBold.ttf".to_string()
    };

    let output_file = "./test/read.html";
    let filename: PathBuf = PathBuf::from(fontname);    
    let font = Font::get_font_from_file(&filename).unwrap();
    let text_file = "./test/read.txt";
    let string = std::fs::read_to_string(text_file).unwrap();
    let html = font.get_html(&string);
    std::fs::write(output_file, html).unwrap(); 

}

