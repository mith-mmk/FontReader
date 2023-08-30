use std::{io::{Cursor, SeekFrom, Read, Seek}, fmt::{Display, Formatter, self}};

use byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub(crate) struct OS2 {
  version: u16,
  x_avg_char_width: i16,
  us_weight_class: u16,
  us_width_class: u16,
  fs_type: u16,
  y_subscript_x_size: i16,
  y_subscript_y_size: i16,
  y_subscript_x_offset: i16,
  y_subscript_y_offset: i16,
  y_superscript_x_size: i16,
  y_superscript_y_size: i16,
  y_superscript_x_offset: i16,
  y_superscript_y_offset: i16,
  y_strikeout_size: i16,
  y_strikeout_position: i16,
  s_family_class: i16,
  panose: [u8; 10],
  ul_unicode_range1: u32,
  ul_unicode_range2: u32,
  ul_unicode_range3: u32,
  ul_unicode_range4: u32,
  ach_vend_id: [u8; 4],
  fs_selection: u16,
  us_first_char_index: u16,
  us_last_char_index: u16,
  s_typo_ascender: i16,
  s_typo_descender: i16,
  s_typo_line_gap: i16,
  us_win_ascent: u16,
  us_win_descent: u16,
  ul_code_page_range1: u32,
  ul_code_page_range2: u32,
  sx_height: i16,
  s_cap_height: i16,
  us_default_char: u16,
  us_break_char: u16,
  us_max_context: u16,
  us_lower_optical_point_size: u16,
  us_upper_optical_point_size: u16,
}



impl fmt::Display for OS2 {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl OS2 {
  pub(crate) fn new<R:Read + Seek>(file: R, offest: u32, length: u32) -> Self {
    get_os2(file, offest, length)
  }

  pub(crate) fn to_string(&self) -> String {
    let mut string = "OS/2\n".to_string();
    let version = format!("Version {}\n", self.version);
    string += &version;
    let x_avg_char_width = format!("xAvgCharWidth {}\n", self.x_avg_char_width);
    string += &x_avg_char_width;
    let us_weight_class = format!("usWeightClass {}\n", self.us_weight_class);
    string += &us_weight_class;
    let us_width_class = format!("usWidthClass {}\n", self.us_width_class);
    string += &us_width_class;
    let fs_type = format!("fsType {}\n", self.fs_type);
    string += &fs_type;
    let y_subscript_x_size = format!("ySubscriptXSize {}\n", self.y_subscript_x_size);
    string += &y_subscript_x_size;
    let y_subscript_y_size = format!("ySubscriptYSize {}\n", self.y_subscript_y_size);
    string += &y_subscript_y_size;
    let y_subscript_x_offset = format!("ySubscriptXOffset {}\n", self.y_subscript_x_offset);
    string += &y_subscript_x_offset;
    let y_subscript_y_offset = format!("ySubscriptYOffset {}\n", self.y_subscript_y_offset);
    string += &y_subscript_y_offset;
    let y_superscript_x_size = format!("ySuperscriptXSize {}\n", self.y_superscript_x_size);
    string += &y_superscript_x_size;
    let y_superscript_y_size = format!("ySuperscriptYSize {}\n", self.y_superscript_y_size);
    string += &y_superscript_y_size;
    let y_superscript_x_offset = format!("ySuperscriptXOffset {}\n", self.y_superscript_x_offset);
    string += &y_superscript_x_offset;
    let y_superscript_y_offset = format!("ySuperscriptYOffset {}\n", self.y_superscript_y_offset);
    string += &y_superscript_y_offset;
    let y_strikeout_size = format!("yStrikeoutSize {}\n", self.y_strikeout_size);
    string += &y_strikeout_size;
    let y_strikeout_position = format!("yStrikeoutPosition {}\n", self.y_strikeout_position);
    string += &y_strikeout_position;
    let s_family_class = format!("sFamilyClass {}\n", self.s_family_class);
    string += &s_family_class;
    let mut panose = format!("Panose :");
    for value in self.panose.iter() {
      panose += &format!(" {}", value);
    }
    panose += "\n";
    string += &panose;
    let ul_unicode_range1 = format!("ulUnicodeRange1 {}\n", self.ul_unicode_range1);
    string += &ul_unicode_range1;
    let ul_unicode_range2 = format!("ulUnicodeRange2 {}\n", self.ul_unicode_range2);
    string += &ul_unicode_range2;
    let ul_unicode_range3 = format!("ulUnicodeRange3 {}\n", self.ul_unicode_range3);
    string += &ul_unicode_range3;
    let ul_unicode_range4 = format!("ulUnicodeRange4 {}\n", self.ul_unicode_range4);
    string += &ul_unicode_range4;
    let ach_vend_id = format!("achVendID {:?}\n", self.ach_vend_id);
    string += &ach_vend_id;
    let fs_selection = format!("fsSelection {}\n", self.fs_selection);
    string += &fs_selection;
    let us_first_char_index = format!("usFirstCharIndex {}\n", self.us_first_char_index);
    string += &us_first_char_index;
    let us_last_char_index = format!("usLastCharIndex {}\n", self.us_last_char_index);
    string += &us_last_char_index;
    let s_typo_ascender = format!("sTypoAscender {}\n", self.s_typo_ascender);
    string += &s_typo_ascender;
    let s_typo_descender = format!("sTypoDescender {}\n", self.s_typo_descender);
    string += &s_typo_descender;
    let s_typo_line_gap = format!("sTypoLineGap {}\n", self.s_typo_line_gap);
    string += &s_typo_line_gap;
    let us_win_ascent = format!("usWinAscent {}\n", self.us_win_ascent);
    string += &us_win_ascent;
    let us_win_descent = format!("usWinDescent {}\n", self.us_win_descent);
    string += &us_win_descent;
    if self.version == 0 {
      return string;
    }
    let ul_code_page_range1 = format!("ulCodePageRange1 {}\n", self.ul_code_page_range1);
    string += &ul_code_page_range1;
    let ul_code_page_range2 = format!("ulCodePageRange2 {}\n", self.ul_code_page_range2);
    string += &ul_code_page_range2;
    if self.version == 1 {
      return string;
    }
    let sx_height = format!("sxHeight {}\n", self.sx_height);
    string += &sx_height;
    let s_cap_height = format!("sCapHeight {}\n", self.s_cap_height);
    string += &s_cap_height;
    let us_default_char = format!("usDefaultChar {}\n", self.us_default_char);
    string += &us_default_char;
    let us_break_char = format!("usBreakChar {}\n", self.us_break_char);
    string += &us_break_char;
    let us_max_context = format!("usMaxContext {}\n", self.us_max_context);
    string += &us_max_context;
    if self.version < 5 {
      return string;
    }
    let us_lower_optical_point_size = format!("usLowerOpticalPointSize {}\n", self.us_lower_optical_point_size);
    string += &us_lower_optical_point_size;
    let us_upper_optical_point_size = format!("usUpperOpticalPointSize {}\n", self.us_upper_optical_point_size); 
    string += &us_upper_optical_point_size;
    string   
  }  
}

fn get_os2<R:Read + Seek>(mut file: R, offest: u32, length: u32) -> OS2 {
  let mut file = file;
  let mut buffer = vec![0; length as usize];
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  file.read_exact(&mut buffer).unwrap();
  let mut cursor = Cursor::new(buffer);
  let version = cursor.read_u16::<BigEndian>().unwrap();
  let x_avg_char_width = cursor.read_i16::<BigEndian>().unwrap();
  let us_weight_class = cursor.read_u16::<BigEndian>().unwrap();
  let us_width_class = cursor.read_u16::<BigEndian>().unwrap();
  let fs_type = cursor.read_u16::<BigEndian>().unwrap();
  let y_subscript_x_size = cursor.read_i16::<BigEndian>().unwrap();
  let y_subscript_y_size = cursor.read_i16::<BigEndian>().unwrap();
  let y_subscript_x_offset = cursor.read_i16::<BigEndian>().unwrap();
  let y_subscript_y_offset = cursor.read_i16::<BigEndian>().unwrap();
  let y_superscript_x_size = cursor.read_i16::<BigEndian>().unwrap();
  let y_superscript_y_size = cursor.read_i16::<BigEndian>().unwrap();
  let y_superscript_x_offset = cursor.read_i16::<BigEndian>().unwrap();
  let y_superscript_y_offset = cursor.read_i16::<BigEndian>().unwrap();
  let y_strikeout_size = cursor.read_i16::<BigEndian>().unwrap();
  let y_strikeout_position = cursor.read_i16::<BigEndian>().unwrap();
  let s_family_class = cursor.read_i16::<BigEndian>().unwrap();
  let mut panose = [0; 10];
  cursor.read_exact(&mut panose).unwrap();
  let ul_unicode_range1 = cursor.read_u32::<BigEndian>().unwrap();
  let ul_unicode_range2 = cursor.read_u32::<BigEndian>().unwrap();
  let ul_unicode_range3 = cursor.read_u32::<BigEndian>().unwrap();
  let ul_unicode_range4 = cursor.read_u32::<BigEndian>().unwrap();
  let mut ach_vend_id = [0; 4];
  cursor.read_exact(&mut ach_vend_id).unwrap();
  let fs_selection = cursor.read_u16::<BigEndian>().unwrap();
  let us_first_char_index = cursor.read_u16::<BigEndian>().unwrap();
  let us_last_char_index = cursor.read_u16::<BigEndian>().unwrap();
  let s_typo_ascender = cursor.read_i16::<BigEndian>().unwrap();
  let s_typo_descender = cursor.read_i16::<BigEndian>().unwrap();
  let s_typo_line_gap = cursor.read_i16::<BigEndian>().unwrap();
  let us_win_ascent = cursor.read_u16::<BigEndian>().unwrap();

  let mut us_win_descent = 0;
  let mut ul_code_page_range1 = 0;
  let mut ul_code_page_range2 = 0;
  if version >= 1 {
    us_win_descent = cursor.read_u16::<BigEndian>().unwrap();
    ul_code_page_range1 = cursor.read_u32::<BigEndian>().unwrap();
    ul_code_page_range2 = cursor.read_u32::<BigEndian>().unwrap();
  }

  let mut sx_height = 0;
  let mut s_cap_height = 0;
  let mut us_default_char = 0;
  let mut us_break_char = 0;
  let mut us_max_context = 0;

  if version >= 2 {
    sx_height = cursor.read_i16::<BigEndian>().unwrap();
    s_cap_height = cursor.read_i16::<BigEndian>().unwrap();
    us_default_char = cursor.read_u16::<BigEndian>().unwrap();
    us_break_char = cursor.read_u16::<BigEndian>().unwrap();
    us_max_context = cursor.read_u16::<BigEndian>().unwrap();
  }
  let mut us_lower_optical_point_size = 0;
  let mut us_upper_optical_point_size = 0;
if version >= 5 {
    us_lower_optical_point_size = cursor.read_u16::<BigEndian>().unwrap();
    us_upper_optical_point_size = cursor.read_u16::<BigEndian>().unwrap();
  }
  OS2 {
    version,
    x_avg_char_width,
    us_weight_class,
    us_width_class,
    fs_type,
    y_subscript_x_size,
    y_subscript_y_size,
    y_subscript_x_offset,
    y_subscript_y_offset,
    y_superscript_x_size,
    y_superscript_y_size,
    y_superscript_x_offset,
    y_superscript_y_offset,
    y_strikeout_size,
    y_strikeout_position,
    s_family_class,
    panose,
    ul_unicode_range1,
    ul_unicode_range2,
    ul_unicode_range3,
    ul_unicode_range4,
    ach_vend_id,
    fs_selection,
    us_first_char_index,
    us_last_char_index,
    s_typo_ascender,
    s_typo_descender,
    s_typo_line_gap,
    us_win_ascent,
    us_win_descent,
    ul_code_page_range1,
    ul_code_page_range2,
    sx_height,
    s_cap_height,
    us_default_char,
    us_break_char,
    us_max_context,
    us_lower_optical_point_size,
    us_upper_optical_point_size,
  }

}