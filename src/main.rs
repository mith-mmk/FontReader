use std::path::{Path, PathBuf};
mod fontheader;
mod cmap;
mod fontreader;
mod head;
mod hhea;
mod hmtx;
mod maxp;
mod name;
mod os2;

fn main() {
    let font_dir = "./fonts";
    // let font_dir = "C:\\Windows\\Fonts";

    // open dir
    let dir = Path::new(font_dir).read_dir().unwrap();
    let mut font_files = Vec::new();
    for filename in dir {
        let filename = filename.unwrap().path();
        font_files.push(filename);
    }
    let mut fonts = Vec::new();
    for font in font_files {
        println!("fontfile: {:?}", font);
        let file = std::fs::File::open(font).unwrap();
        let font_type = fontheader::get_font_type(&file);
        println!("fonttype: {}", font_type.to_string());
        fonts.push(font_type);
    }
    let fontname = "./fonts/NotoSansJP-Regular.ttf";
    let filename: PathBuf = PathBuf::from(fontname);    
    fontreader::font_load(&filename);


}

