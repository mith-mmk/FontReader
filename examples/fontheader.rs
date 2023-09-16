use fontloader::Font;
#[cfg(target_os = "windows")]
use std::env;
use std::path::PathBuf;

fn main() {
    // agrs[1] is the folder name
    let args: Vec<String> = std::env::args().collect();
    // argv.len()?
    let filename = if args.len() >= 2 {
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
            format!("/System/Library/Fonts/ヒラギノ角ゴシック W4.ttc")
        }
        #[cfg(target_os = "linux")]
        {
            "/usr/share/fonts".to_string()
        }
    };

    let filename = PathBuf::from(filename);
    let font = Font::get_font_from_file(&filename);
    if let Some(mut font) = font {
        let font_number = font.get_font_count();
        println!("fontfile: {:?} {}", filename, font_number);
        for i in 0..font_number {
            font.set_font(i).unwrap();
            println!("\nfont number: {} ", i);
            #[cfg(debug_assertions)]
            {
                println!("{}", font.get_header_raw());
                println!("{}", font.get_os2_raw());
            }
        }
    }
}
