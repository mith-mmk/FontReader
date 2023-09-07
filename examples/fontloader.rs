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

    let output_file = "./test/read.html";
    let filename: PathBuf = PathBuf::from(fontname);
    let font = Font::get_font_from_file(&filename).unwrap();
    let string = font.get_info();
    println!("{}", string);
    let text_file = "./test/read.txt";
    let string = std::fs::read_to_string(text_file).unwrap();
    let html = font.get_html(&string);
    std::fs::write(output_file, html).unwrap();
}
