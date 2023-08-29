use std::{path::PathBuf, fs::File};
use crate::{fontheader::{get_font_type, FontHeaders}, cmap::{self, CmapSubtable, CMAP}, head::{self, HEAD}, hhea::{self, HHEA}, hmtx::HMTX, maxp::{MAXP, self}, name::{self, NAME}};

#[derive(Debug, Clone)]
pub(crate) struct Font {
    pub(crate) font_type: FontHeaders,
    pub(crate) cmap: Box<Vec<CMAP>>,
    pub(crate) head: Box<Vec<HEAD>>,
    pub(crate) hhea: Box<Vec<HHEA>>,
    pub(crate) hmtx: Box<Vec<HMTX>>,
    pub(crate) maxp: Box<Vec<MAXP>>,
    pub(crate) names: Box<Vec<NAME>>,
}


pub fn font_load(filename: &PathBuf) {
  let file = File::open(filename).unwrap();
  let font;
  match get_font_type(&file) {
      FontHeaders::OTF(header) => {
          let mut cmaps = Vec::new();
          let mut heads = Vec::new();
          let mut hheas: Vec<HHEA> = Vec::new();
          let mut hmtxs: Vec<HMTX> = Vec::new();
          let mut maxps: Vec<MAXP> = Vec::new();
          let mut names: Vec<NAME> = Vec::new();
          header.table_records.into_iter().for_each(|record| {
              let tag: [u8;4] = record.table_tag.to_be_bytes();
              #[cfg(debug_assertions)]
              println!("{:?}", tag);
              
              match &tag {
                  b"cmap" => {
                      cmaps.push(cmap::load_cmap_table(&file, record.offset, record.length));
                      
                  }
                  b"head" => {
                      let head = head::get_head(&file, record.offset, record.length);
                      heads.push(head);
                  }
                  b"hhea" => {
                      let hhea = hhea::get_hhea(&file, record.offset, record.length);
                      hheas.push(hhea);
                  }
                  b"hmtx" => {
                    if maxps.len() == 0 {
                      debug_assert!(true, "No maxp table");
                      return
                    }
                    let num_glyphs = maxps[0].num_glyphs;
                      #[cfg(debug_assertions)]
                      println!("hmtx now not implemented");              
                  }
                  b"maxp" => {
                      let maxp = maxp::get_maxp(&file, record.offset, record.length);
                      maxps.push(maxp);
                  }
                  b"name" => {
                      let name: NAME = name::get_names(&file, record.offset, record.length);
                      names.push(name)
                  }

                  _ => {
                      debug_assert!(true, "Unknown table tag")
                  }                        
              }
          });
          if cmaps.len() == 0 {
              debug_assert!(true, "No cmap table");
              return
          }
          if heads.len() == 0 {
              debug_assert!(true, "No head table");
              return
          }
          if hheas.len() == 0 {
              debug_assert!(true, "No hhea table");
              return
          }
          font = Font {
              font_type: get_font_type(&file),
              cmap: Box::new(cmaps),
              head: Box::new(heads),
              hhea: Box::new(hheas),
              hmtx: Box::new(hmtxs),
              maxp: Box::new(maxps),
              names: Box::new(names),
          };
          #[cfg(debug_assertions)]
          {
            println!("{} cmap", font.cmap.len());
            println!("{} {}", font.head.len(),font.head[0].to_string());
            println!("{} {}", font.hhea.len(),font.hhea[0].to_string());  
            println!("{} {}", font.maxp.len(),font.maxp[0].to_string());
            println!("{} {}", font.names.len(),font.names[0].to_string());
          }
      },
      _ => {
          debug_assert!(true, "Unknown font type");
          return
      }
  }
  // debug
  #[cfg(all(debug_assertions,feature = "print_cmap"))]
  font.cmap.into_iter().for_each(|cmap| {
      print!("cmap: {} {} {}", cmap.version, cmap.num_tables, cmap.encoding_records.len());

      println!("Version {} Tables {}", cmap.version, cmap.num_tables);

      let encodings = cmap::select_encoding(&cmap.encoding_records);

      println!("main");
      encodings.records.iter().for_each(|encoding| {
          println!("Platform {:?} Encoding {:?}", encoding.get_platform(), encoding.get_encoding());
      });

      println!("substitute");
      encodings.substitute.iter().for_each(|encoding| {
          println!("Platform {:?} Encoding {:?}", encoding.get_platform(), encoding.get_encoding());
      });
      println!("uvs");

      encodings.uvs.iter().for_each(|encoding| {
          println!("Platform {:?} Encoding {:?}", encoding.get_platform(), encoding.get_encoding());
      });
      // grif
      let maps = cmap::get_cmap_maps(&cmap);

      print!("subtable: ");
      maps.iter().for_each(|cmap_encoding| {
          let subtable = cmap_encoding.cmap_subtable.clone();
          match *subtable {
              CmapSubtable::Format0(_) => {
                println!("Format0");
              },
              CmapSubtable::Format2(_) => {
                println!("Format2");
              },
              CmapSubtable::Format4(_) => {
                println!("Format4");
              },
              CmapSubtable::Format6(_) => {
                println!("Format6");
              },
              CmapSubtable::Format8(_) => {
                println!("Format8");
              },
              CmapSubtable::Format10(_) => {
                println!("Format10");
              },
              CmapSubtable::Format12(format12) => {
                println!("Format12");
                print!("  num_groups: {}", format12.num_groups);
                print!(" groups: {}", format12.groups.len());
                println!("  length: {}", format12.length);
                for i in 0..255 {
                  let group = &format12.groups[i];
                  print!("  start_char_code: {:08X}", group.start_char_code);
                  print!("  end_char_code: {:08X}", group.end_char_code);
                  println!("  start_glyph_id: {:08X}", group.start_glyph_id);
        
                }
              },
              CmapSubtable::Format13(_) => {
                println!("Format13");
              },
              CmapSubtable::Format14(format14) => {
                println!("Format14");
                print!("  num_var_selector_records: {}", format14.num_var_selector_records);
                print!(" var_selector_records: {}", format14.var_selector_records.len());
                println!("  length: {}", format14.length);
                // 256 glyphs
                for i in 0..255 {
                  print!("[{:02X}] ", i);
                  let griph = format14.var_selector_records[0].default_uvs_offset + i;
                  print!("glyph: {:08X} ", griph);
                  let gryph = format14.var_selector_records[0].non_default_uvs_offset + i;
                  print!("glyph: {:08X} ", gryph);
                  println!("");
                }
              }
            }
      });


  });

 
}
