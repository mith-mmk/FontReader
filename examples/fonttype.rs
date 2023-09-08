use std::path::Path;

use fontloader::fontheader;

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
    let folder = if args.len() >= 2 {
        args[1].to_string()
    } else {
        "c:\\windows\\fonts".to_string()
    };
    get_font_type(&folder);
}
