use std::path::PathBuf;
use std::env;
use fontloader::{Font, opentype::NameID};

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
          let home = env::var("HOME").unwrap();
          format!("{}/Library/Fonts/ヒラギノ角ゴシック W4.ttc", home)
      }
      #[cfg(target_os = "linux")]
      {
          let home = env::var("HOME").unwrap();
          format!("{}/.fonts/NotoSansJP-Regular.otf", home)
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
          let namelist = font.get_name(&"ja-JP".to_string());
          NameID::iter().into_iter().for_each(|name_id| {
            let name = namelist.get(&(name_id as u16));
            if let Some(name) = name {
              println!("{:?}: {:?}", name_id, name);
            }
          });
        }
    } 
}
