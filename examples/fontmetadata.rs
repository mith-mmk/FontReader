mod common;

use common::{font_index, font_path};
use fontloader::FontFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = font_path(&args);
    let selected_index = font_index(&args, 0);

    let file = FontFile::from_file(&path)?;
    println!("{}", file.dump());
    println!("source: {}", path.display());

    for index in 0..file.face_count() {
        let face = file.face(index)?;
        println!();
        println!("[face {}]", index);
        println!("family: {}", face.family());
        println!("full_name: {}", face.full_name());
        println!("weight: {}", face.weight().0);
        println!("italic: {}", face.is_italic());
    }

    let face = file.face(selected_index)?;
    println!();
    println!("[selected face {} dump]", selected_index);
    println!("{}", face.dump());

    Ok(())
}
