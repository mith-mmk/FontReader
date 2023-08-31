
use std::{path::PathBuf, fs::File};
use crate::outline::*;
use crate::requires::*;
use crate::fontheader;

#[cfg(debug_assertions)]
use std::io::{Write, BufWriter};


#[derive(Debug, Clone)]
pub(crate) struct Font {
  pub(crate) font_type: fontheader::FontHeaders,
  pub(crate) cmap: Option<cmap::CmapEncodings>,
  pub(crate) head: Option<head::HEAD>,
  pub(crate) hhea: Option<hhea::HHEA>,
  pub(crate) hmtx: Option<hmtx::HMTX>,
  pub(crate) maxp: Option<maxp::MAXP>,
  pub(crate) name: Option<name::NAME>,
  pub(crate) os2: Option<os2::OS2>,
  pub(crate) post: Option<post::POST>,
  pub(crate) loca: Option<loca::LOCA>,
  pub(crate) grif: Option<glyf::GLYF>,
  hmtx_pos: Option<Pointer>,
  loca_pos: Option<Pointer>,
  glyf_pos: Option<Pointer>,

}

#[derive(Debug, Clone)]
struct Pointer {
  pub(crate) offset: u32,
  pub(crate) length: u32
}


pub fn font_load(filename: &PathBuf) {
  let file = File::open(filename).unwrap();
  let mut font = Font {
    font_type: fontheader::FontHeaders::Unknown,
    cmap: None,
    head: None,
    hhea: None,
    hmtx: None,
    maxp: None,
    name: None,
    os2: None,
    post: None,
    loca: None,
    grif: None,
    hmtx_pos: None,
    loca_pos: None,
    glyf_pos: None,
  };

  match fontheader::get_font_type(&file) {
    fontheader::FontHeaders::OTF(header) => {
      font.font_type = fontheader::FontHeaders::OTF(header.clone());
      header.table_records.into_iter().for_each(|record| {
        let tag: [u8;4] = record.table_tag.to_be_bytes();
        #[cfg(debug_assertions)]
        {
          for i in 0..4 {
            let ch = tag[i] as char;
             print!("{}", ch);
          }
          println!("{:?}", tag);
        }
              
        match &tag {
          b"cmap" => {
            let cmap_encodings = cmap::CmapEncodings::new(&file, record.offset, record.length);
            font.cmap = Some(cmap_encodings);
          }
          b"head" => {
            let head = head::HEAD::new(&file, record.offset, record.length);
            font.head = Some(head);
          }
          b"hhea" => {
            let hhea = hhea::HHEA::new(&file, record.offset, record.length);
            font.hhea = Some(hhea);
          }
          b"hmtx" => {
            let htmx_pos = Pointer {
              offset: record.offset,
              length: record.length,
            };
            font.hmtx_pos = Some(htmx_pos);
          }
          b"maxp" => {
            let maxp = maxp::MAXP::new(&file, record.offset, record.length);
            font.maxp = Some(maxp);
          }
          b"name" => {
            let name = name::NAME::new(&file, record.offset, record.length);
            font.name = Some(name);
          }
          b"OS/2" => {
            let os2 = os2::OS2::new(&file, record.offset, record.length);
            font.os2 = Some(os2);
          }
          b"post" => {
            let post = post::POST::new(&file, record.offset, record.length);
            font.post = Some(post);
          }
          b"loca" => {
            let loca_pos = Pointer {
              offset: record.offset,
              length: record.length,
            };
            font.loca_pos = Some(loca_pos);
          }
          b"glyf" => {
            let glyf_pos = Pointer {
              offset: record.offset,
              length: record.length,
            };
            println!("glyf {:08x} {}", glyf_pos.offset, glyf_pos.length);
            font.glyf_pos = Some(glyf_pos);
          } 
          _ => {
            debug_assert!(true, "Unknown table tag")
          }                        
        }
      });

      let num_glyphs = font.maxp.as_ref().unwrap().num_glyphs;
      let number_of_hmetrics = font.hhea.as_ref().unwrap().number_of_hmetrics;
      let offset = font.hmtx_pos.as_ref().unwrap().offset;
      let length = font.hmtx_pos.as_ref().unwrap().length;

      let hmtx = hmtx::HMTX::new(&file, offset, length, number_of_hmetrics, num_glyphs);
      font.hmtx = Some(hmtx);

      let offset = font.loca_pos.as_ref().unwrap().offset;
      let length = font.loca_pos.as_ref().unwrap().length;
      let loca = loca::LOCA::new(&file, offset, length, num_glyphs);
      font.loca = Some(loca);

      let offset = font.glyf_pos.as_ref().unwrap().offset;
      let length = font.glyf_pos.as_ref().unwrap().length;
      let loca = font.loca.as_ref().unwrap();
      let glyf = glyf::GLYF::new(&file, offset, length, loca);
      font.grif = Some(glyf);

      if font.cmap.is_none() {
        debug_assert!(true, "No cmap table");
        return
      }
      if font.head.is_none() {
        debug_assert!(true, "No head table");
        return
      }
      if font.hhea.is_none() {
        debug_assert!(true, "No hhea table");
        return
      }

      #[cfg(debug_assertions)]
      {
        // create or open file
        let file = match File::create("test/font.txt") {
            Ok(it) => it,
            Err(_) => {
              File::open("test/font.txt").unwrap()
            }
        };
        let mut writer = BufWriter::new(file);

        let encoding_records = &font.cmap.as_ref().unwrap().get_encoding_engine();
        writeln!(&mut writer, "{}", &font.cmap.as_ref().unwrap().cmap.to_string()).unwrap();
        for i in 0..encoding_records.len() {
          writeln!(&mut writer, "{} {}", i,encoding_records[i].to_string()).unwrap();
        }
        writeln!(&mut writer, "{}", font.head.unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", font.hhea.unwrap().to_string()).unwrap();  
            writeln!(&mut writer, "{}", font.maxp.unwrap().to_string()).unwrap();
            writeln!(&mut writer, "{}", font.hmtx.unwrap().to_string()).unwrap();
            writeln!(&mut writer, "{}", font.os2.unwrap().to_string()).unwrap();
            writeln!(&mut writer, "{}", font.post.unwrap().to_string()).unwrap();
            writeln!(&mut writer, "{}", font.loca.unwrap().to_string()).unwrap();

            writeln!(&mut writer, "{}", font.name.unwrap().to_string()).unwrap();

            writeln!(&mut writer, "long cmap -> griph").unwrap();
            let cmap_encodings = font.cmap.unwrap().clone();
            let glyf = font.grif.as_ref().unwrap();
   
            for i in 0x25A0..0x25Af {
                let pos = cmap_encodings.get_griph_position(i);
                let glyph = glyf.get_glyph(pos as usize).unwrap();
                let ch = char::from_u32(i).unwrap();
                writeln!(&mut writer,"{}:{:04} ", ch , pos).unwrap();
                writeln!(&mut writer,"{}", glyph.to_string()).unwrap();
            }
            writeln!(&mut writer, "").unwrap();

            for i in 0x4e00 .. 0x4eff {
                if i as u32 % 16 == 0 {
                    writeln!(&mut writer, "").unwrap();
                }
                let pos = cmap_encodings.get_griph_position(i as u32);
                let ch = char::from_u32(i as u32).unwrap();
                write!(&mut writer,"{}:{:04} ", ch , pos).unwrap();
            }

            writeln!(&mut writer, "").unwrap();
            let i = 0x2a6b2;
            let pos = cmap_encodings.get_griph_position(i as u32);
            let ch = char::from_u32(i as u32).unwrap();
            writeln!(&mut writer, "{}:{:04} ", ch , pos).unwrap();

        }
    },
    _ => {
       debug_assert!(true, "Unknown font type");
       return
    }
  } 
}
