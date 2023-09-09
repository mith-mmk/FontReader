use std::path::PathBuf;
use fontloader::{Font, opentype::NameID};

fn main() {
  // agrs[1] is the folder name
  let args: Vec<String> = std::env::args().collect();
  // argv.len()?
  let filename = if args.len() >= 2 {
      args[1].to_string()
  } else {
      "c:\\windows\\fonts\\msgothic.ttc".to_string()
  };
  let filename = PathBuf::from(filename);
      let font = Font::get_font_from_file(&filename);
      if let Some(mut font) = font {
        let font_number = font.get_font_count();
        println!("fontfile: {:?} {}", filename, font_number);
        for i in 0..font_number {
          println!("{:?}", font.set_font(i));
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
