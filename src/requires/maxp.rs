use std::{io::{Cursor, SeekFrom, Read, Seek}, fmt};

use byteorder::{BigEndian, ReadBytesExt};


#[derive(Debug, Clone)]
pub(crate) struct MAXP {
  pub(crate) version: u32,
  pub(crate) num_glyphs: u16,
  // 1.0
  pub(crate) max_points: u16,
  pub(crate) max_contours: u16,
  pub(crate) max_composite_points: u16,
  pub(crate) max_composite_contours: u16,
  pub(crate) max_zones: u16,
  pub(crate) max_twilight_points: u16,
  pub(crate) max_storage: u16,
  pub(crate) max_function_defs: u16,
  pub(crate) max_instruction_defs: u16,
  pub(crate) max_stack_elements: u16,
  pub(crate) max_size_of_instructions: u16,
  pub(crate) max_component_elements: u16,
  pub(crate) max_component_depth: u16
}

impl fmt::Display for MAXP {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_string())
  }
}

impl MAXP {
  pub(crate) fn new<R:Read + Seek>(file: R, offest: u32, length: u32) -> Self {
    get_maxp(file, offest, length)
  }

  pub(crate) fn to_string(&self) -> String {
    let mut string = "maxp\n".to_string();
    let major = self.version >> 16;
    let minor = self.version & 0xFFFF;
    let version = format!("Version {:X}.{:04X}\n", major, minor);
    string += &version;
    let num_glyphs = format!("Number of Glyphs {}\n", self.num_glyphs);
    string += &num_glyphs;
    let max_points = format!("Max Points {}\n", self.max_points);
    string += &max_points;
    let max_contours = format!("Max Contours {}\n", self.max_contours);
    string += &max_contours;
    let max_composite_points = format!("Max Composite Points {}\n", self.max_composite_points);
    string += &max_composite_points;
    let max_composite_contours = format!("Max Composite Contours {}\n", self.max_composite_contours);
    string += &max_composite_contours;
    let max_zones = format!("Max Zones {}\n", self.max_zones);
    string += &max_zones;
    let max_twilight_points = format!("Max Twilight Points {}\n", self.max_twilight_points);
    string += &max_twilight_points;
    let max_storage = format!("Max Storage {}\n", self.max_storage);
    string += &max_storage;
    let max_function_defs = format!("Max Function Defs {}\n", self.max_function_defs);
    string += &max_function_defs;
    let max_instruction_defs = format!("Max Instruction Defs {}\n", self.max_instruction_defs);
    string += &max_instruction_defs;
    let max_stack_elements = format!("Max Stack Elements {}\n", self.max_stack_elements);
    string += &max_stack_elements;
    let max_size_of_instructions = format!("Max Size of Instructions {}\n", self.max_size_of_instructions);
    string += &max_size_of_instructions;
    let max_component_elements = format!("Max Component Elements {}\n", self.max_component_elements);
    string += &max_component_elements;
    let max_component_depth = format!("Max Component Depth {}\n", self.max_component_depth);
    string += &max_component_depth;
    string
  }
}


fn get_maxp<R: Read + Seek>(file: R, offest: u32, length: u32) -> MAXP{
  let mut file = file;
  file.seek(SeekFrom::Start(offest as u64)).unwrap();
  let mut buf = vec![0; length as usize];
  file.read_exact(&mut buf).unwrap();
  let mut cursor = Cursor::new(buf);
  let version = cursor.read_u32::<BigEndian>().unwrap();
  let num_glyphs = cursor.read_u16::<BigEndian>().unwrap();
  if version == 0x00005000 {
    return MAXP{
      version,
      num_glyphs,
      max_points: 0,
      max_contours: 0,
      max_composite_points: 0,
      max_composite_contours: 0,
      max_zones: 0,
      max_twilight_points: 0,
      max_storage: 0,
      max_function_defs: 0,
      max_instruction_defs: 0,
      max_stack_elements: 0,
      max_size_of_instructions: 0,
      max_component_elements: 0,
      max_component_depth: 0,
    }
  }

  let max_points = cursor.read_u16::<BigEndian>().unwrap();
  let max_contours = cursor.read_u16::<BigEndian>().unwrap();
  let max_composite_points = cursor.read_u16::<BigEndian>().unwrap();
  let max_composite_contours = cursor.read_u16::<BigEndian>().unwrap();
  let max_zones = cursor.read_u16::<BigEndian>().unwrap();
  let max_twilight_points = cursor.read_u16::<BigEndian>().unwrap();
  let max_storage = cursor.read_u16::<BigEndian>().unwrap();
  let max_function_defs = cursor.read_u16::<BigEndian>().unwrap();
  let max_instruction_defs = cursor.read_u16::<BigEndian>().unwrap();
  let max_stack_elements = cursor.read_u16::<BigEndian>().unwrap();
  let max_size_of_instructions = cursor.read_u16::<BigEndian>().unwrap();
  let max_component_elements = cursor.read_u16::<BigEndian>().unwrap();
  let max_component_depth = cursor.read_u16::<BigEndian>().unwrap();
  MAXP {
    version,
    num_glyphs,
    max_points,
    max_contours,
    max_composite_points,
    max_composite_contours,
    max_zones,
    max_twilight_points,
    max_storage,
    max_function_defs,
    max_instruction_defs,
    max_stack_elements,
    max_size_of_instructions,
    max_component_elements,
    max_component_depth,
  }


}