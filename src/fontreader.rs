use std::{path::PathBuf, fs::File};
use crate::outline::*;
use crate::requires::*;
use crate::fontheader;
use crate::requires::hmtx::LongHorMetric;

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

impl Font {
  pub fn get_font_from_file(filename: &PathBuf) -> Option<Self> {
    font_load(filename)
  }

  pub fn get_h_metrix(&self, id: usize) -> LongHorMetric {
    let hmtx = self.hmtx.as_ref().unwrap();
    hmtx.get_metrix(id)
  }
  pub fn get_horizontal_layout(&self, id:usize) -> HorizontalLayout{
    let lsb = self.get_h_metrix(id).left_side_bearing as isize;
    let advance_width = self.get_h_metrix(id).advance_width as isize;

    let accender = self.hhea.as_ref().unwrap().get_accender() as isize;
    let descender = self.hhea.as_ref().unwrap().get_descender() as isize;
    let line_gap = self.hhea.as_ref().unwrap().get_line_gap() as isize;

    HorizontalLayout {
      lsb,
      advance_width,
      accender,
      descender,
      line_gap,
    }
  }

  pub fn get_svg(&self, ch: char) -> String {
    // utf-32
    let code = ch as u32;
    let pos = self.cmap.as_ref().unwrap().get_griph_position(code);
    let glyf = self.grif.as_ref().unwrap().get_glyph(pos as usize).unwrap();
    let layout: HorizontalLayout = self.get_horizontal_layout(pos as usize);
    let width = "24pt";
    let height = "24pt";
    let svg = glyf.to_svg(width, height,&layout);
    svg
  }

  pub fn get_html(&self, string: &str) -> String {
    let mut html = String::new();
    html += "<html>\n";
    html += "<head>\n";
    html += "<meta charset=\"UTF-8\">\n";
    html += "<title>fontreader</title>\n";
    html += "</head>\n";
    html += "<body>\n";
    for ch in string.chars() {
      if ch == '\n' || ch == '\r'{
        html += "<br>\n";
        continue;
      }
      if ch == '\t' {
        html += "<span style=\"width: 4em; display: inline-block;\"></span>\n";
        continue;
      }
      if ch == ' ' {
        html += "<span style=\"width: 1em; display: inline-block;\"></span>\n";
        continue;
      }
      let svg = self.get_svg(ch);
      html += &svg;
    }
    html += "</body>\n";
    html += "</html>\n";
    html
  }

}

#[derive(Debug, Clone)]
pub struct HorizontalLayout {
  pub lsb: isize,
  pub advance_width: isize,
  pub accender: isize,
  pub descender: isize,
  pub line_gap: isize,

}

#[derive(Debug, Clone)]
struct Pointer {
  pub(crate) offset: u32,
  pub(crate) length: u32
}

fn font_load(filename: &PathBuf) -> Option<Font> {
  let file = File::open(filename).unwrap();
  font_load_from_file(file)
}


fn font_load_from_file(file: File) -> Option<Font> {
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
        return None
      }
      if font.head.is_none() {
        debug_assert!(true, "No head table");
        return None
      }
      if font.hhea.is_none() {
        debug_assert!(true, "No hhea table");
        return None
      }
      if font.hmtx.is_none() {
        debug_assert!(true, "No hmtx table");
        return None
      }
      if font.maxp.is_none() {
        debug_assert!(true, "No maxp table");
        return None
      }
      if font.name.is_none() {
        debug_assert!(true, "No name table");
        return None
      }
      if font.os2.is_none() {
        debug_assert!(true, "No OS/2 table");
        return None
      }
      if font.post.is_none() {
        debug_assert!(true, "No post table");
        return None
      }
      if font.loca.is_none() {
        debug_assert!(true, "Not support no loca table");
        return None
      }
      if font.grif.is_none() {
        debug_assert!(true, "Not support no glyf table");
        return None
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
        writeln!(&mut writer, "{}", &font.head.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", &font.hhea.as_ref().unwrap().to_string()).unwrap();  
        writeln!(&mut writer, "{}", &font.maxp.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", &font.hmtx.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", &font.os2.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", &font.post.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", &font.loca.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "{}", &font.name.as_ref().unwrap().to_string()).unwrap();
        writeln!(&mut writer, "long cmap -> griph").unwrap();
        let cmap_encodings = &font.cmap.as_ref().unwrap().clone();
        let glyf = font.grif.as_ref().unwrap();
        for i in 0x0020..0x0ff {
            let pos = cmap_encodings.get_griph_position(i);
            let glyph = glyf.get_glyph(pos as usize).unwrap();
            let layout = font.get_horizontal_layout(pos as usize);
            let svg = glyph.to_svg("100px", "100px",&layout);
            let ch = char::from_u32(i).unwrap();
            writeln!(&mut writer,"{}:{:04} ", ch , pos).unwrap();
            writeln!(&mut writer,"{}", glyph.to_string()).unwrap();
            writeln!( &mut writer,"{}:{:?}", i, layout).unwrap();
            writeln!(&mut writer,"{}", svg).unwrap();
        }
        writeln!(&mut writer, "").unwrap();
        for i in 0x4e00 .. 0x4eff {
            if i as u32 % 16 == 0 {
                writeln!(&mut writer, "").unwrap();
            }
            let pos = cmap_encodings.get_griph_position(i as u32);
            let glyph = glyf.get_glyph(pos as usize).unwrap();
            let layout = font.get_horizontal_layout(pos as usize);
            let svg = glyph.to_svg(&"100px", &"100px",&layout);
            let ch = char::from_u32(i as u32).unwrap();
            write!(&mut writer,"{}:{:04} ", ch , pos).unwrap();
            writeln!(&mut writer,"{}", svg).unwrap();
          }
          writeln!(&mut writer, "").unwrap();
          let i = 0x2a6b2;
          let pos = cmap_encodings.get_griph_position(i as u32);
          let ch = char::from_u32(i as u32).unwrap();
          writeln!(&mut writer, "{}:{:04} ", ch , pos).unwrap();

        }
    },
    _ => {
       debug_assert!(true, "not support type");
       return None
    }
  }
  Some(font)
}
