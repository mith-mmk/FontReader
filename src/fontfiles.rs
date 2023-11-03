// new implement font files

use crate::binary_reader::BinaryReader;

pub struct FontFiles {
  font_style: String,
  font_file: Vec<FontFile>
}

pub struct FontFile<R: BinaryReader> {
  name: Option<String>,
  uri: String,  // file path or url
  uri_type: URIType,
  fonts: Vec<Font>,
  font_header: FontHeaders,
  readers: Option<Vec<R>>
}

impl FontFile {
  pub fn new(uri: String, uri_type: URIType) -> Self {
    // name is Self.get_font_name(uri, uri_type);
    Self {
      name: None,
      uri,
      uri_type,
      fonts: Vec::new(),
      font_header: FontHeader::new(),
      readers: None
    }
  }

}

pub enum URIType {
  File, // local file
  URL,  // internet url
  Base64, // base64 encoded data
  Binary, // binary data
}

pub struct Font {
  header: FontHeader,
  table_records: HashMap<TableRecord>,
  parsed_tables: HashMap<Table>
}

pub enum Table {
  Cmap(cmap),
  Glyf(glyf),
  Head(head),
  Hhea(hhea),
  Hmtx(hmtx),
  Loca(loca),
  Maxp(maxp),
  Name(name),
  Os2(os2),
  Post(post),
  Sbix(sbix),
  CLOR(clor),
  CBDT(cbdt),
  Vhea(vhea),
  Vmtx(vmtx),
  VORG(vorg),
  VDMX(vdmx),
  GDEF(gdef),
  GPOS(gpos),
  GSUB(gsub),
  BASE(base),
}