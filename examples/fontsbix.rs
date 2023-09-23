use fontloader::{opentype::NameID, Font};
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
    if let Ok(mut font) = font {
        let font_number = font.get_font_count();
        println!("fontfile: {:?} {}", filename, font_number);
        for i in 0..font_number {
            font.set_font(i).unwrap();
            println!("\nfont number: {} ", i);
            NameID::iter().into_iter().for_each(|name_id| {
                let name = font.get_name(name_id, &"ja".to_string());
                if !name.is_err() {
                    println!("{:?}: {:?}", name_id, name.unwrap());
                }
            });
            println!("{}", font.get_sbix_raw());
            #[cfg(debug_assertions)]
            {
                println!("{}", font.get_sbix_raw());
            }
        }
    }
}
