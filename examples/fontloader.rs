use fontloader::Font;
use std::{env, path::PathBuf};

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

    let output_file = "./test/read.html";
    let filename: PathBuf = PathBuf::from(fontname);
    let font = Font::get_font_from_file(&filename).unwrap();
    let _len = font.get_font_count();
    /*
    let res = font.set_font(len - 1);
    if res.is_err() {
        print!("error: {:?}", res);
    }
    */

    let string = font.get_info();
    println!("{}", string);
    let text_file = "./test/read.txt";
    let string = std::fs::read_to_string(text_file).unwrap();
    let html = font.get_html(&string);
    std::fs::write(output_file, html).unwrap();
}
