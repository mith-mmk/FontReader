pub mod outline;
pub mod requires;
pub use outline::glyf::Glyph;

use crate::util::u32_to_string;

use self::requires::cmap;

#[derive(Debug, Clone)]
pub struct OTFHeader {
    pub(crate) sfnt_version: u32,
    pub(crate) num_tables: u16,
    pub(crate) search_range: u16,
    pub(crate) entry_selector: u16,
    pub(crate) range_shift: u16,
    pub(crate) table_records: Box<Vec<TableRecord>>,
}

#[derive(Debug, Clone)]
pub(crate) struct TableRecord {
    pub(crate) table_tag: u32,
    pub(crate) check_sum: u32,
    pub(crate) offset: u32,
    pub(crate) length: u32,
}

impl TableRecord {
    pub(crate) fn to_string(self) -> String {
        let mut string = String::new();
        string.push_str(&"table tag: ".to_string());
        string.push_str(u32_to_string(self.table_tag).as_str());
        string.push_str(&" check sum: ".to_string());
        // hex digit
        string.push_str(&format!("{:08X}", self.check_sum));
        string.push_str(&" offset: ".to_string());
        string.push_str(&format!("{:08X}", self.offset));
        string.push_str(&" length: ".to_string());
        string.push_str(&format!("{:08X}", self.length));
        string
    }
}


pub(crate) enum FontTable {
  // required
      CMAP(cmap::CMAP),
      HEAD,
      HHEA,
      HMTX,
      MAXP,
      NAME,
      OS2,
      POST,
  // tables related to TrueType outlines
      CVT,
      FPGM,
      GLYF,
      LOCA,
      PREP,
      GASP,
  // tables related to CFF outlines
      CFF,
      CFF2,
      VORG,
  // SVG
      SVG,
  // Bitmap Glyphs
      EBDT,
      EBLC,
      EBSV,
  // Color Bitmap Glyphs
      CBDT,
      CBLC,
      COLR,
      CPAL,
  // Advanced Typographics
      BASE,
      GDEF,
      GPOS,
      GSUB,
      JSTF,
      MATH,
      MERG,
      PROP,
      ZWJ,
  // OpenType Font Variant
      AVAR,
      CVAR,
      FVAR,
      GVAR,
      HVAR,
      MVAR,
      STAT,
      VVAR,
  // Color Fonts
      // COLR,
      // CPAL,
      // CBDT,
      // CBLC,
      SBIX,
      // SVG,
  // Other OpenType Tables
      DISG,
      HDMX,
      KERN,
      LTSH,
  //    MERG,
      META,
  //    STAT,
      PCLT,
      VDMX,
      VHEA,
      VMTX,
      UNKNOWN
  }