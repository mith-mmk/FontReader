use std::{path::PathBuf, fs::File};
use crate::*;
#[derive(Debug, Clone)]
pub(crate) struct Font {
    pub(crate) font_type: fontheader::FontHeaders,
    pub(crate) cmap: Box<Vec<cmap::CmapEncodings>>,
    pub(crate) head: Box<Vec<head::HEAD>>,
    pub(crate) hhea: Box<Vec<hhea::HHEA>>,
    pub(crate) hmtx: Box<Vec<hmtx::HMTX>>,
    pub(crate) maxp: Box<Vec<maxp::MAXP>>,
    pub(crate) names: Box<Vec<name::NAME>>,
    pub(crate) os2s: Box<Vec<os2::OS2>>,
    pub(crate) posts: Box<Vec<post::POST>>,
}


pub fn font_load(filename: &PathBuf) {
  let file = File::open(filename).unwrap();
  let font;
  match fontheader::get_font_type(&file) {
      fontheader::FontHeaders::OTF(header) => {
          let mut cmaps = Vec::new();
          let mut heads = Vec::new();
          let mut hheas = Vec::new();
          let hmtxs = Vec::new();
          let mut maxps: Vec<maxp::MAXP> = Vec::new();
          let mut names = Vec::new();
          let mut os2s = Vec::new();
          let mut posts = Vec::new();
          header.table_records.into_iter().for_each(|record| {
              let tag: [u8;4] = record.table_tag.to_be_bytes();
              #[cfg(debug_assertions)]
              println!("{:?}", tag);
              
              match &tag {
                  b"cmap" => {
                      let cmap_encodings = cmap::CmapEncodings::new(&file, record.offset, record.length);
                      cmaps.push(cmap_encodings);
                      
                  }
                  b"head" => {
                      let head = head::HEAD::new(&file, record.offset, record.length);
                      heads.push(head);
                  }
                  b"hhea" => {
                      let hhea = hhea::HHEA::new(&file, record.offset, record.length);
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
                      let maxp = maxp::MAXP::new(&file, record.offset, record.length);
                      maxps.push(maxp);
                  }
                  b"name" => {
                      let name = name::NAME::new(&file, record.offset, record.length);
                      names.push(name)
                  }
                  b"OS/2" => {
                      let os2 = os2::OS2::new(&file, record.offset, record.length);
                      os2s.push(os2);
                  }
                  b"post" => {
                      let post = post::POST::new(&file, record.offset, record.length);
                      posts.push(post);
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
              font_type: fontheader::get_font_type(&file),
              cmap: Box::new(cmaps),
              head: Box::new(heads),
              hhea: Box::new(hheas),
              hmtx: Box::new(hmtxs),
              maxp: Box::new(maxps),
              names: Box::new(names),
              os2s: Box::new(os2s),
              posts: Box::new(posts),
          };
          #[cfg(debug_assertions)]
          {
            println!("{} {}", font.cmap.len(),font.cmap[0].cmap.to_string());
            for i in 0..font.cmap[0].cmap.encoding_records.len() {
              println!("{} {}", i,font.cmap[0].cmap.encoding_records[i].to_string());
            }
            println!("{} {}", font.head.len(),font.head[0].to_string());
            println!("{} {}", font.hhea.len(),font.hhea[0].to_string());  
            println!("{} {}", font.maxp.len(),font.maxp[0].to_string());
            println!("{} {}", font.names.len(),font.names[0].to_string());
            println!("{} {}", font.os2s.len(),font.os2s[0].to_string());
            println!("{} {}", font.posts.len(),font.posts[0].to_string());

            println!("long cmap -> griph");
            let cmap_encodings = font.cmap[0].cmap_encodings.clone();
            for (i, cmap_encoding) in cmap_encodings.iter().enumerate() {
              print!("No {}:", i);

              let encode_record = cmap_encoding.encoding_record.clone();
              println!("encode_record: {:?}", encode_record);
              let subtable = cmap_encoding.cmap_subtable.clone();
              println!("subtable: {}", subtable.get_part_of_string(16));
            }

          }
      },
      _ => {
          debug_assert!(true, "Unknown font type");
          return
      }
  } 
}
