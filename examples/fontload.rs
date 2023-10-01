use fontloader::Font;
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // agrs[1] is the folder name
    let args: Vec<String> = std::env::args().collect();
    // argv.len()?
    let fontname = if args.len() >= 2 {
        args[1].to_string()
    } else {
        #[cfg(target_os = "windows")]
        {
            // $env:windir\fonts\msgothic.ttc
            let windir = env::var("windir")?;
            format!("{}\\fonts\\msgothic.ttc", windir)
        }
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME")?;
            format!("{}/Library/Fonts/ヒラギノ角ゴシック W4.ttc", home)
        }
        #[cfg(target_os = "linux")]
        {
            "/usr/share/fonts".to_string()
        }
    };

    let output_file = "./test/loader.html";
    let filename: PathBuf = PathBuf::from(fontname);
    let font = Font::get_font_from_file(&filename).unwrap();
    let _len = font.get_font_count();
    /*
    let res = font.set_font(len - 1);
    if res.is_err() {
        print!("error: {:?}", res);
    }
    */

    let string = font.get_info()?;
    println!("{}", string); 
    let mut html = "<html><head><meta charset=\"utf-8\"></head><body>".to_string();
    for i in 0..65535 {
        let j = i;
        let result = font.get_svg_from_id(j, 32.0, "px");
        if result.is_err() {
            break;
        }
        html += result.unwrap().as_str();
    }
    html += "</body></html>";
    std::fs::write(output_file, html)?;
    Ok(())
}
