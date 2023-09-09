use fontloader::Font;
use std::{path::PathBuf, env};

fn main() {
    // agrs[1] is the folder name
    let args: Vec<String> = std::env::args().collect();
    // argv.len()?
    let fontname = if args.len() >= 2 {
        args[1].to_string()
    } else {
        #[cfg(target_os = "windows")]
        {
            // $env:windir\fonts\msgothic.ttc
            let windir = env::var("windir").unwrap();
            format!("{}\\fonts\\msgothic.ttc", windir)
        }
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").unwrap();
            format!("{}/Library/Fonts/ヒラギノ角ゴシック W4.ttc", home)
        }
        #[cfg(target_os = "linux")]
        {
            "/usr/share/fonts".to_string()
        }
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
