use std::{path::PathBuf, fs::File};
use crate::outline::*;
use crate::requires::*;
use crate::fontheader;
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
  pub(crate) locas: Box<Vec<loca::LOCA>>,
  pub(crate) grifs: Box<Vec<glyf::GLYF>>,
}


pub fn font_load(filename: &PathBuf) {
  let file = File::open(filename).unwrap();
  let font;
  match fontheader::get_font_type(&file) {
    fontheader::FontHeaders::OTF(header) => {
      let mut cmaps = Vec::new();
      let mut heads = Vec::new();
      let mut hheas = Vec::new();
      let mut hmtxs = Vec::new();
      let mut maxps: Vec<maxp::MAXP> = Vec::new();
      let mut names = Vec::new();
      let mut os2s = Vec::new();
      let mut posts = Vec::new();
      let mut locas = Vec::new();
      let mut grifs = Vec::new();
      let mut loca_offset = 0;
      let mut loca_length = 0;
      let mut griph_offset = 0;
      let mut griph_length = 0;
      header.table_records.into_iter().for_each(|record| {
        let tag: [u8;4] = record.table_tag.to_be_bytes();
        #[cfg(debug_assertions)]
        {
          for i in 0..4 {
            let ch = tag[i] as char;
             print!("{}", ch);
          }
           println!(" {:?}", tag);
        }
              
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
            if maxps.len() == 0 || hheas.len() == 0 {
              debug_assert!(true, "No maxp table");
              return
            } else {
              let num_glyphs = maxps[0].num_glyphs;
              let number_of_hmetrics = hheas[0].number_of_hmetrics;
              let hmtx = hmtx::HMTX::new(&file, record.offset, record.length,
                                                  number_of_hmetrics, num_glyphs);
                hmtxs.push(hmtx);
            }
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
          b"loca" => {
            loca_offset = record.offset;
            loca_length = record.length;
          }
          b"glyf" => {
            griph_offset = record.offset;
            griph_length = record.length;
          } 
          _ => {
            debug_assert!(true, "Unknown table tag")
          }                        
        }
      });
      if (loca_offset == 0) {
        debug_assert!(true, "No loca table");
        return
      }
      if (griph_offset == 0) {
        debug_assert!(true, "No glyf table");
        return
      }
      let num_glyphs = maxps[0].num_glyphs;
      let loca = loca::LOCA::new(&file, loca_offset, loca_length, num_glyphs);
      locas.push(loca);
      let grif = glyf::GLYF::new(&file, griph_offset, griph_length, &Box::new(locas[0].clone()));
      grifs.push(grif);

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
        locas: Box::new(locas),
        grifs: Box::new(grifs),
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
            if font.hmtx.len() > 0 {
              println!("{} {}", font.hmtx.len(),font.hmtx[0].to_string());
            } else {
              println!("{} {}", font.hmtx.len(),"No hmtx table");
            }
            println!("{} {}", font.os2s.len(),font.os2s[0].to_string());
            println!("{} {}", font.posts.len(),font.posts[0].to_string());
            println!("{} {}", font.locas.len(),font.locas[0].to_string());

            println!("{} {}", font.names.len(),font.names[0].to_string());

            println!("long cmap -> griph");
            let cmap_encodings = font.cmap[0].clone();
   
            for i in 0x20..0xff {
                if i % 16 == 0 {
                    println!("");
                }
                let pos = cmap_encodings.get_griph_position(i);
                let ch = char::from_u32(i).unwrap();
                print!("{}:{:04} ", ch , pos);
            }
            println!("");

            for i in 0x4e00 .. 0x4eff {
                if i as u32 % 16 == 0 {
                    println!("");
                }
                let pos = cmap_encodings.get_griph_position(i as u32);
                let ch = char::from_u32(i as u32).unwrap();
                print!("{}:{:04} ", ch , pos);
            }

            println!("");
            let i = 0x2a6b2;
            let pos = cmap_encodings.get_griph_position(i as u32);
            let ch = char::from_u32(i as u32).unwrap();
            println!("{}:{:04} ", ch , pos);

        }
    },
    _ => {
       debug_assert!(true, "Unknown font type");
       return
    }
  } 
}
