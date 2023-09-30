use std::{env, path::Path};

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
        let font_type = fontheader::get_font_type_from_file(&mut font);
        if let Err(e) = font_type {
            println!("font load error: {}", e);
            continue;
        }
        let font_type = font_type.unwrap();
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
        #[cfg(target_os = "windows")]
        {
            // $env:windir\fonts\msgothic.ttc
            let windir = env::var("windir").unwrap();
            format!("{}\\fonts\\", windir)
        }
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").unwrap();
            format!("{}/Library/Fonts/", home)
        }
        #[cfg(target_os = "linux")]
        {
            "/usr/share/fonts".to_string()
        }
    };
    get_font_type(&folder);
}
