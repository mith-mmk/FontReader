// CFF is Adobe Type 1 font format, which is a compact binary format.
use bin_rs::reader::BinaryReader;
use std::{collections::HashMap, error::Error, io::SeekFrom};

use crate::fontreader::FontLayout;

// Compare this snippet from src/outline/cff.rs:

//
// // CFF is Adobe Type 1 font format, which is a compact binary format.
//
/*/
type Card8 = u8;
type Card16 = u16;
type OffSize = u8;
type Offset = u32; 1-4bytes
type SID = u16;
type Card32 = u32;
*/

#[derive(Debug, Clone)]
pub(crate) struct CFF {
    pub(crate) header: Header,
    pub(crate) names: Vec<String>,
    pub(crate) top_dict: Dict, // TopDict
    pub(crate) bbox: [f64; 4],
    #[cfg(feature = "cff2")]
    pub(crate) global_subr: Option<GlobalSubr>, // CFF2
    #[cfg(feature = "cff2")]
    pub(crate) variation_store: Vec<VariationStore>, // CFF2
    pub(crate) strings: Vec<String>,
    pub(crate) charsets: Charsets,
    pub(crate) char_string: CharString,
    // pub(crate) fd_Select: FDSelect,
    // pub(crate) fd_dict_index: FDDistIndex,
    pub(crate) private_dict: Option<PrivateDict>,
    pub(crate) gsubr: Option<CharString>,
    pub(crate) subr: Option<CharString>,
    pub(crate) default_width: f64,
    pub(crate) width: f64,
}

struct ParcePack {
    x: f64,
    y: f64,
    min_x: f64,
    width: Option<f64>,
    stacks: Box<Vec<f64>>,
    is_first: usize,
    hints: usize,
    commands: Box<Commands>,
}

enum Operation {
    M(f64, f64),
    C(f64, f64, f64, f64, f64, f64),
    L(f64, f64),
    Z,
}

struct Commands {
    operations: Vec<Operation>,
    commands: Vec<String>,
}

impl Commands {
    fn new() -> Self {
        Self {
            operations: Vec::new(),
            commands: Vec::new(),
        }
    }
}

impl CFF {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        _length: u32,
    ) -> Result<Self, Box<dyn Error>> {
        let offset = offset as u64;
        reader.seek(SeekFrom::Start(offset))?;
        let mut bbox = [0.0, 0.0, 1000.0, 1000.0];
        let mut width;
        let mut default_width = 0.0;

        let header = Header::parse(reader)?;
        let name_index = Index::parse(reader)?;
        let names = name_index
            .data
            .iter()
            .map(|name| String::from_utf8(name.clone()))
            .collect::<Result<Vec<String>, _>>()?;
        let top_dict_index = Index::parse(reader)?;
        let top_dict = Dict::parse(&top_dict_index.data[0])?;
        let string_index = if header.major == 1 {
            let index = Index::parse(reader)?;
            let mut strings = Vec::new();
            for data in &index.data {
                let string = String::from_utf8(data.clone())?;
                strings.push(string);
            }
            Some(strings)
        } else {
            None // CFF2
        };
        // Global Index Subr
        let gsbrtn = CharString::new(reader, 0)?;
        let gsubr = Some(gsbrtn);
        // cff2
        /*
        let variation_store = VariationStore::new(reader)?;
        */

        let n_glyphs = top_dict.get_i32(0, 15).unwrap() as usize;
        let encording_offset = top_dict.get_i32(0, 16); // none
        let fd_array_offset = top_dict.get_i32(12, 36);
        let fd_select_offset = top_dict.get_i32(12, 37);
        let opt_bbox = top_dict.get_f64_array(0, 5);
        width = top_dict.get_f64(12, 8).unwrap_or(0.0);
        default_width = width;
        if let Some(some_bbox) = opt_bbox {
            bbox = [some_bbox[0], some_bbox[1], some_bbox[2], some_bbox[3]];
        }
        let charsets_offset = top_dict.get_i32(0, 15).unwrap();
        let char_strings_offset = top_dict.get_i32(0, 17).unwrap();
        let vsstore_offset = top_dict.get_i32(0, 24); // none
        let ros = top_dict.get_i32_array(12, 30);
        let private = top_dict.get_i32_array(0, 18);
        #[cfg(debug_assertions)]
        {
            println!("header: {:?}", header);
            println!("name:",);
            for name in &names {
                println!("  {}", name);
            }
            println!("string_index: {:?}", string_index);
            println!("top_dict: {}", top_dict.to_string());
            println!("n_glyphs: {}", n_glyphs);
            println!("encording: {:?}", encording_offset);
            println!("fd_array: {:?}", fd_array_offset);
            println!("fd_select: {:?}", fd_select_offset);
            println!("charsets: {:?}", charsets_offset);
            println!("char_strings: {:?}", char_strings_offset);
            println!("vsstore: {:?}", vsstore_offset);
            println!("bbox: {:?}", bbox);
            println!("ROS: {:?}", ros);
            println!("private: {:?}", private);
        }

        // must CID = GID
        let charsets_offset = charsets_offset as u64 + offset;

        let charsets = Charsets::new(reader, charsets_offset, n_glyphs as u32)?;
        let char_strings_offset = char_strings_offset as u64 + offset;
        let char_string = CharString::new(reader, char_strings_offset)?;

        let private = top_dict.get_i32_array(0, 18);
        let mut subr = None;
        let private_dict = if let Some(private) = private {
            let _ = private[0] as u32; //
            let private_dict_offset = private[1] as u64 + offset;
            reader.seek(SeekFrom::Start(private_dict_offset as u64))?;
            let buffer = reader.read_bytes_as_vec(private[0] as usize)?;
            let private_dict = Dict::parse(&buffer)?;
            default_width = private_dict.get_f64(0, 20).unwrap_or(0.0);
            width = private_dict.get_f64(0, 21).unwrap_or(width);
            #[cfg(debug_assertions)]
            {
                println!("private_dict: {}", private_dict.to_string());
                println!("defaultWidthX: {:?}", private_dict.get_f64(0, 20));
                println!("normalWidthX: {:?}", private_dict.get_f64(0, 21));
            }
            if let Some(sub_offset) = private_dict.get_i32(0, 19) {
                let subr_offset = sub_offset as u64 + private_dict_offset;
                let subrtn = CharString::new(reader, subr_offset)?;
                subr = Some(subrtn)
            }

            Some(private_dict)
        } else {
            None
        };
        if ros.is_some() {
            let fd_array_offset = (fd_array_offset.unwrap() as u64 + offset) as u64;
            reader.seek(SeekFrom::Start(fd_array_offset))?;
            let fd_arrays = Index::parse(reader)?;
            #[cfg(debug_assertions)]
            {
                println!("fd_arrays: {:?}", fd_arrays);
            }
            let font_dict = Dict::parse(&fd_arrays.data[0])?;
            #[cfg(debug_assertions)]
            {
                println!("font_dict:\n{}", font_dict.to_string());
            }

            if let Some(private) = font_dict.get_i32_array(0, 18) {
                let private_dict_offset = private[1] as u64 + offset;
                reader.seek(SeekFrom::Start(private_dict_offset))?;
                let buffer = reader.read_bytes_as_vec(private[0] as usize);
                let private_dict = Dict::parse(&buffer?)?;
                #[cfg(debug_assertions)]
                {
                    println!("private_dict: {}", private_dict.to_string());
                    println!("defaultWidthX: {:?}", private_dict.get_f64(0, 20));
                    println!("nominalWidthX: {:?}", private_dict.get_f64(0, 21));
                }
                default_width = private_dict.get_f64(0, 20).unwrap_or(default_width);
                width = private_dict.get_f64(0, 21).unwrap_or(width);

                let subr_offset = private_dict.get_i32(0, 19);
                if let Some(subr_offset) = subr_offset {
                    let subr_offset = subr_offset as u64 + private_dict_offset;
                    let subrtn = CharString::new(reader, subr_offset)?;
                    subr = Some(subrtn);
                }
            }
            // let fd_select = FDSelect::new(reader, fd_select_offset.unwrap() as u32 + offset, n_glyphs);
            // println!("fd_select: {:?}", fd_select);
        }

        Ok(Self {
            header,
            names,
            top_dict,
            strings: Vec::new(),
            charsets,
            char_string,
            bbox,
            // fd_Select: FDSelect,
            // fd_dict_index: FDDistIndex,
            private_dict,
            gsubr,
            subr,
            width,
            default_width,
        })
    }

    pub fn to_code(&self, gid: usize, layout: &FontLayout) -> String {
        #[cfg(debug_assertions)]
        {
            let cid = self.charsets.sid[gid as usize];
            println!("gid {} cid {}", gid, cid);
        }
        let data = &self.char_string.data.data[gid as usize];
        self.parse_data(gid, data, 24.0, "pt", layout, 0.0, 0.0, false)
    }

    fn parse(&self, data: &[u8], parce_data: &mut ParcePack) -> Option<()> {
        let mut i = 0;
        // w? {hs* vs* cm* hm* mt subpath}? {mt subpath}* endchar
        while i < data.len() {
            let b0 = data[i];
            i += 1;
            #[cfg(debug_assertions)]
            {
                // let command = format!("{} {:?}",b0, parce_data.stacks);
                // parce_data.commands.as_mut().commands.push(command);
            }

            match b0 {
                1 => {
                    // hstem |- y dy {dya dyb}* hstem (1) |
                    let mut command = "hstem".to_string();
                    let mut args = Vec::new();
                    args.push(parce_data.stacks.pop()?);
                    args.push(parce_data.stacks.pop()?);

                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }

                    parce_data.hints += args.len();

                    let mut y = args.pop()?;
                    command += &format!(" {}", y);
                    if 1 <= args.len() {
                        let dy = args.pop()?;
                        y += dy;
                        command += &format!(" {}", y);
                    }
                    while 2 <= args.len() {
                        let dya = args.pop()?;
                        y += dya;
                        command += &format!(" {}", y);
                        let dyb = args.pop()?;
                        y += dyb;
                        command += &format!(" {}", y);
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                3 => {
                    // vstem |- x dx {dxa dxb}* vstem (3) |

                    let mut args = Vec::new();
                    args.push(parce_data.stacks.pop()?);
                    args.push(parce_data.stacks.pop()?);
                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    parce_data.hints += args.len();

                    let mut command = "vstem".to_string();
                    let mut x = args.pop()?;
                    command += &format!(" {}", x);
                    let dx = args.pop()?;
                    command += &format!(" {}", dx);
                    while 2 <= args.len() {
                        let dxa = args.pop()?;
                        x += dxa;
                        command += &format!(" {}", x);
                        let dxb = args.pop()?;
                        x += dxb;
                        command += &format!(" {}", x);
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                18 => {
                    // hstemhm |- y dy {dya dyb}* hstemhm (18) |-
                    let mut args = Vec::new();
                    args.push(parce_data.stacks.pop()?);
                    args.push(parce_data.stacks.pop()?);
                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    parce_data.hints += args.len();

                    let mut command = "hstemhm".to_string();
                    let mut y = args.pop()?;
                    command += &format!(" {}", y);
                    let dy = args.pop()?;
                    y += dy;
                    command += &format!(" {}", y);
                    while 2 <= args.len() {
                        let dya = args.pop()?;
                        y += dya;
                        command += &format!(" {}", y);
                        let dyb = args.pop()?;
                        y += dyb;
                        command += &format!(" {}", y);
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                23 => {
                    // vstemhm |- x dx {dxa dxb}* vstemhm (23) |-
                    let mut args = Vec::new();
                    args.push(parce_data.stacks.pop()?);
                    args.push(parce_data.stacks.pop()?);
                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    parce_data.hints += args.len();
                    let mut command = "vstemhm".to_string();
                    let mut x = args.pop()?;
                    command += &format!(" {}", x);
                    let dx = args.pop()?;
                    x += dx;
                    command += &format!(" {}", x);
                    while 2 <= args.len() {
                        let dxa = args.pop()?;
                        x += dxa;
                        command += &format!(" {}", x);
                        let dxb = args.pop()?;
                        x += dxb;
                        command += &format!(" {}", x);
                        parce_data.hints += 2;
                    }
                    command += "\n";
                    parce_data.commands.as_mut().commands.push(command);
                }
                19 => {
                    // hintmask |- hintmask (19 + mask) |
                    /* pop vstemhm */
                    if parce_data.is_first == 0 && parce_data.stacks.len() >= 2 {
                        let mut args = Vec::new();
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        while 2 <= parce_data.stacks.len() {
                            args.push(parce_data.stacks.pop()?);
                            args.push(parce_data.stacks.pop()?);
                        }
                        parce_data.hints += args.len();

                        let mut command = "vstemhm".to_string();
                        let mut x = args.pop()?;
                        command += &format!(" {}", x);
                        let dx = args.pop()?;
                        x += dx;
                        command += &format!(" {}", x);
                        while 2 <= args.len() {
                            let dxa = args.pop()?;
                            x += dxa;
                            command += &format!(" {}", x);
                            let dxb = args.pop()?;
                            x += dxb;
                            command += &format!(" {}", x);
                        }
                        command += "\n";
                        parce_data.commands.as_mut().commands.push(command);
                    }

                    let len = (parce_data.hints / 2 + 7) / 8;
                    let mut command = "hintmask".to_string();
                    command += &format!(" {}", parce_data.hints / 2);
                    for j in 0..len {
                        let mask = data[i + j];
                        command += &format!(" {:08b} {}", mask, mask);
                    }
                    i += len;
                    parce_data.commands.as_mut().commands.push(command);
                }
                20 => {
                    // cntrmask |- cntrmask (20 + mask) |-
                    /* pop vstemhm */
                    if parce_data.is_first == 0 && parce_data.stacks.len() >= 2 {
                        let mut args = Vec::new();
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        while 2 <= parce_data.stacks.len() {
                            args.push(parce_data.stacks.pop()?);
                            args.push(parce_data.stacks.pop()?);
                        }
                        parce_data.hints += args.len();
                        let mut command = "vstemhm".to_string();
                        let mut x = args.pop()?;
                        command += &format!(" {}", x);
                        let dx = args.pop()?;
                        x += dx;
                        command += &format!(" {}", x);
                        while 2 <= args.len() {
                            let dxa = args.pop()?;
                            x += dxa;
                            command += &format!(" {}", x);
                            let dxb = args.pop()?;
                            x += dxb;
                            command += &format!(" {}", x);
                        }
                        command += "\n";
                        parce_data.commands.as_mut().commands.push(command);
                    }

                    let len = (parce_data.hints / 2 + 7) / 8;
                    let mut command = format! {"cntrmask {} {}",parce_data.hints / 2, len};
                    for j in 0..len {
                        let mask = data[i + j];
                        command += &format!(" {:08b}", mask);
                    }
                    i += len;
                    parce_data.commands.as_mut().commands.push(command);
                }

                21 => {
                    // rmoveto |- dx1 dy1 rmoveto (21) |-
                    if parce_data.is_first > 0 {
                        parce_data.commands.as_mut().operations.push(Operation::Z);
                    }

                    let dy = parce_data.stacks.pop()?;
                    let dx = parce_data.stacks.pop()?;
                    parce_data.x += dx;
                    parce_data.y += dy;
                    parce_data.min_x = parce_data.min_x.min(parce_data.x);
                    let command = format!("rmoveto {} {}", dx, dy);
                    parce_data.commands.as_mut().commands.push(command);
                    parce_data
                        .commands
                        .as_mut()
                        .operations
                        .push(Operation::M(parce_data.x, parce_data.y));

                    if 1 <= parce_data.stacks.len() && parce_data.is_first == 0 {
                        parce_data.width = parce_data.stacks.pop();
                        let width = parce_data.width;
                        parce_data
                            .commands
                            .as_mut()
                            .commands
                            .push(format!("width {:?}", width));
                    }
                    parce_data.is_first += 1;
                }
                22 => {
                    // |- dx1 hmoveto (22) |-
                    if parce_data.is_first > 0 {
                        parce_data.commands.as_mut().operations.push(Operation::Z);
                    }
                    let dx = parce_data.stacks.pop()?;
                    parce_data.x += dx;
                    parce_data.min_x = parce_data.min_x.min(parce_data.x);
                    let command = format!("hmoveto {}", dx);
                    parce_data.commands.as_mut().commands.push(command);
                    parce_data
                        .commands
                        .as_mut()
                        .operations
                        .push(Operation::M(parce_data.x, parce_data.y));
                    if 1 <= parce_data.stacks.len() && parce_data.is_first == 0 {
                        parce_data.width = parce_data.stacks.pop();
                        let width = parce_data.width;
                        parce_data
                            .commands
                            .as_mut()
                            .commands
                            .push(format!("width {:?}", width));
                    }
                    parce_data.is_first += 1;
                }
                4 => {
                    // |- dy1 vmoveto (4) |-
                    if parce_data.is_first > 0 {
                        parce_data.commands.as_mut().operations.push(Operation::Z);
                    }
                    let dy = parce_data.stacks.pop()?;
                    parce_data.y += dy;
                    let command = format!("vmoveto {}\n", dy);
                    parce_data.commands.as_mut().commands.push(command);
                    parce_data
                        .commands
                        .as_mut()
                        .operations
                        .push(Operation::M(parce_data.x, parce_data.y));

                    if 1 <= parce_data.stacks.len() && parce_data.is_first == 0 {
                        parce_data.width = parce_data.stacks.pop();
                        let width = parce_data.width;
                        parce_data
                            .commands
                            .as_mut()
                            .commands
                            .push(format!("width {:?}", width));
                    }
                    parce_data.is_first += 1;
                }
                5 => {
                    // rlineto |- {dxa dya}+ rlineto (5) |-
                    let mut args = Vec::new();
                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }

                    let mut command = "rlineto".to_string();
                    while 2 <= args.len() {
                        let dxa = args.pop()?;
                        let dya = args.pop()?;
                        parce_data.x += dxa;
                        parce_data.min_x = parce_data.min_x.min(parce_data.x);
                        parce_data.y += dya;
                        command += &format!(" {}", dxa);
                        command += &format!(" {}", dya);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::L(parce_data.x, parce_data.y));
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                6 => {
                    //  - dx1 {dya dxb}* hlineto (6) |
                    // |- {dxa dyb}+ hlineto (6) |-      even
                    let mut args = Vec::new();
                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 1 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                    }

                    let mut command = "hlineto".to_string();
                    if args.len() % 2 == 1 {
                        let dx1 = args.pop()?;
                        parce_data.x += dx1;
                        parce_data.min_x = parce_data.min_x.min(parce_data.x);
                        command += &format!(" dx1 {}", dx1);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::L(parce_data.x, parce_data.y));
                        while 2 <= args.len() {
                            let dya = args.pop()?;
                            let dxb = args.pop()?;

                            parce_data.y += dya;
                            command += &format!(" {}", dya);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));

                            parce_data.x += dxb;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            command += &format!(" {}", dxb);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));
                        }
                    } else {
                        while 2 <= args.len() {
                            let dxa = args.pop()?;
                            let dyb = args.pop()?;

                            parce_data.x += dxa;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            command += &format!(" {}", dxa);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));

                            parce_data.y += dyb;
                            command += &format!(" {}", dyb);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));
                        }
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                7 => {
                    // |- dy1 {dxa dyb}* vlineto (7) |-
                    // |- {dya dxb}+ vlineto (7) |-
                    let mut args = Vec::new();
                    while 2 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 1 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                    }

                    let mut command = "vlineto".to_string();

                    if args.len() % 2 == 1 {
                        let dy1 = args.pop()?;
                        parce_data.y += dy1;
                        command += &format!(" dy1 {}", dy1);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::L(parce_data.x, parce_data.y));
                        while 2 <= args.len() {
                            let dxa = args.pop()?;
                            let dyb = args.pop()?;
                            parce_data.x += dxa;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            command += &format!(" dxa {}", dxa);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));

                            parce_data.y += dyb;
                            command += &format!(" dyb {}", dyb);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));
                        }
                    } else {
                        while 2 <= args.len() {
                            let dya = args.pop()?;
                            let dxb = args.pop()?;

                            parce_data.y += dya;
                            command += &format!(" {}", dya);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));

                            parce_data.x += dxb;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            command += &format!(" {}", dxb);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::L(parce_data.x, parce_data.y));
                        }
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                8 => {
                    // |- {dxa dya dxb dyb dxc dyc}+ rrcurveto (8) |-
                    let mut args = Vec::new();
                    while 6 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);

                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }

                    let mut command = "rrcurveto".to_string();
                    while 6 <= args.len() {
                        let dxa = args.pop()?;
                        let dya = args.pop()?;
                        let dxb = args.pop()?;
                        let dyb = args.pop()?;
                        let dxc = args.pop()?;
                        let dyc = args.pop()?;

                        parce_data.x += dxa;
                        parce_data.y += dya;
                        command += &format!(" {}", dxa);
                        command += &format!(" {}", dya);
                        let xa = parce_data.x;
                        let ya = parce_data.y;

                        parce_data.x += dxb;
                        parce_data.y += dyb;
                        command += &format!(" {}", dxb);
                        command += &format!(" {}", dyb);
                        let xb = parce_data.x;
                        let yb = parce_data.y;

                        parce_data.x += dxc;
                        parce_data.y += dyc;
                        let xc = parce_data.x;
                        let yc = parce_data.y;
                        command += &format!(" {}", dxc);
                        command += &format!(" {}", dyc);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::C(xa, ya, xb, yb, xc, yc));
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                27 => {
                    // |- dy1? {dxa dxb dyb dxc}+ hhcurveto (27) |-
                    let mut args = Vec::new();
                    while 4 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 1 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                    }

                    let mut command = "hhcurveto".to_string();
                    if args.len() % 4 == 1 {
                        // dy1?
                        let dy1 = args.pop()?;
                        command += &format!(" dy1 {}", dy1);
                        parce_data.y += dy1;
                    }

                    while 4 <= args.len() {
                        // dxa dxb dyb dxc
                        let dxa = args.pop()?;
                        let dxb = args.pop()?;
                        let dyb = args.pop()?;
                        let dxc = args.pop()?;

                        parce_data.x += dxa;
                        let xa = parce_data.x;
                        let ya = parce_data.y;

                        command += &format!(" dxa {}", dxa);

                        parce_data.x += dxb;
                        parce_data.y += dyb;
                        let xb = parce_data.x;
                        let yb = parce_data.y;

                        command += &format!(" dxb {} dyb {}", dxb, dyb);

                        parce_data.x += dxc;
                        parce_data.min_x = parce_data.min_x.min(parce_data.x);
                        // parce_data.y += dy1;
                        let xc = parce_data.x;
                        let yc = parce_data.y;

                        command += &format!(" dxc {}", dxc);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::C(xa, ya, xb, yb, xc, yc));
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                31 => {
                    // |- dx1 dx2 dy2 dy3 {dya dxb dyb dxc dxd dxe dye dyf}* dxf? hvcurveto (31) |-
                    // |- {dxa dxb dyb dyc dyd dxe dye dxf}+ dyf? hvcurveto (31) |-

                    let mut args = Vec::new();
                    while 8 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);

                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 4 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 1 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                    }

                    let mut command = "hvcurveto".to_string();
                    if args.len() % 8 >= 4 {
                        // dx1 dx2 dy2 dy3
                        let dx1 = args.pop()?;
                        let dx2 = args.pop()?;
                        let dy2 = args.pop()?;
                        let dy3 = args.pop()?;

                        parce_data.x += dx1;
                        let xa = parce_data.x;
                        let ya = parce_data.y;
                        command += &format!(" dx1 {}", dx1);

                        parce_data.x += dx2;
                        parce_data.y += dy2;
                        command += &format!(" dx2 {}", dx2);
                        command += &format!(" dy2 {}", dy2);
                        let xb = parce_data.x;
                        let yb = parce_data.y;

                        parce_data.y += dy3;
                        let xc = parce_data.x;
                        let yc = parce_data.y;
                        command += &format!(" dy3 {}", dy3);

                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::C(xa, ya, xb, yb, xc, yc));
                        let mut lp = false;
                        while 8 <= args.len() {
                            // dya dxb dyb dxc dxd dxe dye dyf
                            lp = true;
                            let dya = args.pop()?;
                            let dxb = args.pop()?;
                            let dyb = args.pop()?;
                            let dxc = args.pop()?;
                            let dxd = args.pop()?;
                            let dxe = args.pop()?;
                            let dye = args.pop()?;
                            let dyf = args.pop()?;

                            parce_data.y += dya;
                            command += &format!(" {}", dya);
                            let xa = parce_data.x;
                            let ya = parce_data.y;
                            parce_data.x += dxb;
                            parce_data.y += dyb;
                            let xb = parce_data.x;
                            let yb = parce_data.y;
                            command += &format!(" dxa {}", dxb);
                            command += &format!(" dxb {}", dyb);

                            parce_data.x += dxc;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            let xc = parce_data.x;
                            let yc = parce_data.y;
                            command += &format!(" dxc {}", dxc);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));

                            parce_data.x += dxd;

                            let xd = parce_data.x;
                            let yd = parce_data.y;
                            command += &format!(" dxd {}", dxd);
                            parce_data.x += dxe;
                            parce_data.y += dye;
                            let xe = parce_data.x;
                            let ye = parce_data.y;
                            command += &format!(" dxe {}", dxe);
                            command += &format!(" dye {}", dye);

                            parce_data.y += dyf;
                            let xf = parce_data.x;
                            let yf = parce_data.y;
                            command += &format!(" dyf {}", dyf);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));
                        }
                        if 1 <= args.len() {
                            let dxf = args.pop()?;
                            parce_data.x += dxf;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            let xf = parce_data.x;
                            command += &format!(" dxf {}", dxf);
                            let op = parce_data.commands.as_mut().operations.pop()?;
                            if let Operation::C(xd, yd, xe, ye, _, yf) = op {
                                parce_data
                                    .commands
                                    .as_mut()
                                    .operations
                                    .push(Operation::C(xd, yd, xe, ye, xf, yf));
                            } else {
                                parce_data.commands.as_mut().operations.push(op);
                            }
                        }
                    } else {
                        while 8 <= args.len() {
                            let dxa = args.pop()?;
                            let dxb = args.pop()?;
                            let dyb = args.pop()?;
                            let dyc = args.pop()?;
                            let dyd = args.pop()?;
                            let dxe = args.pop()?;
                            let dye = args.pop()?;
                            let dxf = args.pop()?;

                            parce_data.x += dxa;
                            let xa = parce_data.x;
                            let ya = parce_data.y;
                            command += &format!(" dxa {}", dxa);

                            parce_data.x += dxb;
                            parce_data.y += dyb;
                            let xb = parce_data.x;
                            let yb = parce_data.y;
                            command += &format!(" dxb {}", dxb);
                            command += &format!(" dyb {}", dyb);

                            parce_data.y += dyc;
                            let xc = parce_data.x;
                            let yc = parce_data.y;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));
                            command += &format!(" dyc {}", dyc);

                            parce_data.y += dyd;
                            let xd = parce_data.x;
                            let yd = parce_data.y;
                            command += &format!(" dyd {}", dyd);
                            parce_data.x += dxe;
                            parce_data.y += dye;
                            let xe = parce_data.x;
                            let ye = parce_data.y;
                            command += &format!(" dxe {}", dxe);
                            command += &format!(" dye {}", dye);
                            parce_data.x += dxf;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            let xf = parce_data.x;
                            let yf = parce_data.y;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));
                            command += &format!(" dxf {}", dxf);
                        }
                        if 1 <= args.len() {
                            let dyf = args.pop()?;
                            parce_data.y += dyf;
                            let yf = parce_data.y;
                            command += &format!(" dyf {}", dyf);
                            let op = parce_data.commands.as_mut().operations.pop()?;
                            if let Operation::C(xd, yd, xe, ye, xf, _) = op {
                                parce_data
                                    .commands
                                    .as_mut()
                                    .operations
                                    .push(Operation::C(xd, yd, xe, ye, xf, yf));
                            } else {
                                parce_data.commands.as_mut().operations.push(op);
                            }
                        }
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                24 => {
                    // rcurveline rcurveline |- {dxa dya dxb dyb dxc dyc}+ dxd dyd rcurveline (24) |-
                    let mut args = Vec::new();
                    args.push(parce_data.stacks.pop()?);
                    args.push(parce_data.stacks.pop()?);
                    while 6 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    let mut command = "rcurveline".to_string();
                    while 8 <= args.len() {
                        let dxa = args.pop()?;
                        let dya = args.pop()?;
                        parce_data.x += dxa;
                        parce_data.y += dya;
                        let xa = parce_data.x;
                        let ya = parce_data.y;
                        command += &format!(" {}", dxa);
                        command += &format!(" {}", dya);
                        let dxb = args.pop()?;
                        let dyb = args.pop()?;
                        parce_data.x += dxb;
                        parce_data.y += dyb;
                        let xb = parce_data.x;
                        let yb = parce_data.y;
                        command += &format!(" {}", dxb);
                        command += &format!(" {}", dyb);
                        let dxc = args.pop()?;
                        let dyc = args.pop()?;
                        parce_data.x += dxc;
                        parce_data.min_x = parce_data.min_x.min(parce_data.x);
                        parce_data.y += dyc;
                        let xc = parce_data.x;
                        let yc = parce_data.y;
                        command += &format!(" {}", dxc);
                        command += &format!(" {}", dyc);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::C(xa, ya, xb, yb, xc, yc));
                    }
                    let dxd = args.pop()?;
                    let dyd = args.pop()?;
                    parce_data.x += dxd;
                    parce_data.y += dyd;
                    let xx = parce_data.x;
                    parce_data.min_x = parce_data.min_x.min(parce_data.x);
                    let yy = parce_data.y;
                    command += &format!(" dxd {}", dxd);
                    command += &format!(" dyd {}", dyd);
                    parce_data
                        .commands
                        .as_mut()
                        .operations
                        .push(Operation::L(xx, yy));
                    parce_data.commands.as_mut().commands.push(command);
                }
                25 => {
                    // rlinecurve rlinecurve |- {dxa dya}+ dxb dyb dxc dyc dxd dyd rlinecurve (25) |-
                    let mut args = Vec::new();
                    while 8 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    let d1 = parce_data.stacks.pop()?;
                    let d2 = parce_data.stacks.pop()?;
                    let d3 = parce_data.stacks.pop()?;
                    let d4 = parce_data.stacks.pop()?;
                    let d5 = parce_data.stacks.pop()?;
                    let d6 = parce_data.stacks.pop()?;
                    args.push(d1);
                    args.push(d2);
                    args.push(d3);
                    args.push(d4);
                    args.push(d5);
                    args.push(d6);
                    let mut command = "rlinecurve".to_string();
                    while 8 <= args.len() {
                        let dxa = args.pop()?;
                        let dya = args.pop()?;
                        parce_data.x += dxa;
                        parce_data.min_x = parce_data.min_x.min(parce_data.x);
                        parce_data.y += dya;
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::L(parce_data.x, parce_data.y));
                        command += &format!(" dxa {} dya {}", dxa, dya);
                    }
                    let dxb = args.pop()?;
                    let dyb = args.pop()?;
                    let dxc = args.pop()?;
                    let dyc = args.pop()?;
                    let dxd = args.pop()?;
                    let dyd = args.pop()?;

                    parce_data.x += dxb;
                    parce_data.y += dyb;
                    let xa = parce_data.x;
                    let ya = parce_data.y;
                    command += &format!(" {}", dxb);
                    command += &format!(" {}", dyb);
                    parce_data.x += dxc;
                    parce_data.y += dyc;
                    let xb = parce_data.x;
                    let yb = parce_data.y;
                    command += &format!(" {}", dxc);
                    command += &format!(" {}", dyc);

                    parce_data.x += dxd;
                    parce_data.min_x = parce_data.min_x.min(parce_data.x);
                    parce_data.y += dyd;
                    let xc = parce_data.x;
                    let yc = parce_data.y;
                    command += &format!(" {}", dxd);
                    command += &format!(" {}", dyd);
                    parce_data
                        .commands
                        .as_mut()
                        .operations
                        .push(Operation::C(xa, ya, xb, yb, xc, yc));
                    parce_data.commands.as_mut().commands.push(command);
                }
                30 => {
                    // - dy1 dx2 dy2 dx3 {dxa dxb dyb dyc dyd dxe dye dxf}* dyf? vhcurveto (30) |-
                    // |- {dya dxb dyb dxc dxd dxe dye dyf}+ dxf? vhcurveto (30) |
                    let mut args = Vec::new();

                    while 8 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);

                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 4 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 1 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                    }
                    let mut command = "vhcurveto".to_string();
                    // <!-- vhcurveto dy1 -77 dx2 -6 dy2 -39 dx3 -14 dyf -19 -->
                    if args.len() % 8 >= 4 {
                        // - dy1 dx2 dy2 dx3 |-
                        let dy1 = args.pop()?;
                        parce_data.y += dy1;
                        let xa = parce_data.x;
                        let ya = parce_data.y;
                        command += &format!(" dy1 {}", dy1);

                        let dx2 = args.pop()?;
                        let dy2 = args.pop()?;
                        parce_data.x += dx2;
                        parce_data.y += dy2;
                        let xb = parce_data.x;
                        let yb = parce_data.y;
                        command += &format!(" dx2 {}", dx2);
                        command += &format!(" dy2 {}", dy2);

                        let dx3 = args.pop()?;
                        parce_data.x += dx3;
                        let xc = parce_data.x;
                        let yc = parce_data.y;
                        command += &format!(" dx3 {}", dx3);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::C(xa, ya, xb, yb, xc, yc));
                        while 8 <= args.len() {
                            let dxa = args.pop()?;
                            let dxb = args.pop()?;
                            let dyb = args.pop()?;
                            let dyc = args.pop()?;

                            parce_data.x += dxa;
                            let xa = parce_data.x;
                            let ya = parce_data.y;
                            command += &format!(" {}", dxa);

                            parce_data.x += dxb;
                            parce_data.y += dyb;
                            let xb = parce_data.x;
                            let yb = parce_data.y;
                            command += &format!(" {}", dxb);
                            command += &format!(" {}", dyb);

                            parce_data.y += dyc;
                            let xc = parce_data.x;
                            let yc = parce_data.y;
                            command += &format!(" {}", dyc);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));

                            let dyd = args.pop()?;
                            parce_data.y += dyd;
                            let xd = parce_data.x;
                            let yd = parce_data.y;
                            command += &format!(" {}", dyd);

                            let dxe = args.pop()?;
                            let dye = args.pop()?;
                            parce_data.x += dxe;
                            parce_data.y += dye;
                            let xe = parce_data.x;
                            let ye = parce_data.y;
                            command += &format!(" {}", dxe);
                            command += &format!(" {}", dye);

                            let dxf = args.pop()?;
                            parce_data.x += dxf;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            let xf = parce_data.x;
                            let yf = parce_data.y;
                            command += &format!(" {}", dxf);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));
                        }
                        if 1 <= args.len() {
                            let dyf = args.pop()?;
                            parce_data.y += dyf;
                            let yf = parce_data.y;
                            command += &format!(" dyf {}", dyf);
                            let op = parce_data.commands.as_mut().operations.pop()?;
                            if let Operation::C(xd, yd, xe, ye, xf, _) = op {
                                parce_data
                                    .commands
                                    .as_mut()
                                    .operations
                                    .push(Operation::C(xd, yd, xe, ye, xf, yf));
                            } else {
                                parce_data.commands.as_mut().operations.push(op);
                            }
                        }
                    } else {
                        while 8 <= args.len() {
                            // {dya dxb dyb dxc dxd dxe dye dyf}+ dxf?
                            let dya = args.pop()?;
                            let dxb = args.pop()?;
                            let dyb = args.pop()?;
                            let dxc = args.pop()?;

                            parce_data.y += dya;
                            let xa = parce_data.x;
                            let ya = parce_data.y;
                            command += &format!(" {}", dya);

                            parce_data.x += dxb;
                            parce_data.y += dyb;
                            let xb = parce_data.x;
                            let yb = parce_data.y;
                            command += &format!(" {}", dxb);
                            command += &format!(" {}", dyb);

                            parce_data.x += dxc;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            let xc = parce_data.x;
                            let yc = parce_data.y;
                            command += &format!(" {}", dxc);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));

                            let dxd = args.pop()?;
                            parce_data.x += dxd;
                            let xd = parce_data.x;
                            let yd = parce_data.y;
                            command += &format!(" {}", dxd);

                            let dxe = args.pop()?;
                            let dye = args.pop()?;
                            parce_data.x += dxe;
                            parce_data.y += dye;
                            let xe = parce_data.x;
                            let ye = parce_data.y;
                            command += &format!(" {}", dxe);
                            command += &format!(" {}", dye);

                            let dyf = args.pop()?;
                            parce_data.y += dyf;
                            let xf = parce_data.x;
                            let yf = parce_data.y;
                            command += &format!(" {}", dyf);
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));
                        }
                        if 1 <= args.len() {
                            let dxf = args.pop()?;
                            parce_data.x += dxf;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            let xf = parce_data.x;
                            command += &format!(" dxf {}", dxf);
                            let op = parce_data.commands.as_mut().operations.pop()?;
                            if let Operation::C(xd, yd, xe, ye, _, yf) = op {
                                parce_data
                                    .commands
                                    .as_mut()
                                    .operations
                                    .push(Operation::C(xd, yd, xe, ye, xf, yf));
                            } else {
                                parce_data.commands.as_mut().operations.push(op);
                            }
                        }
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }
                26 => {
                    // |- dx1? {dya dxb dyb dyc}+ vvcurveto (26) |-
                    let mut args = Vec::new();
                    while 4 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                        args.push(parce_data.stacks.pop()?);
                    }
                    if 1 <= parce_data.stacks.len() {
                        args.push(parce_data.stacks.pop()?);
                    }
                    let mut command = "vvcurveto".to_string();
                    if args.len() % 4 == 1 {
                        //dx1
                        let dx1 = args.pop()?;
                        parce_data.x += dx1;
                        command += &format!(" dx1 {}", dx1);
                    }
                    while 4 <= args.len() {
                        // dya dxb dyb dyc
                        let dya = args.pop()?;
                        let dxb = args.pop()?;
                        let dyb = args.pop()?;
                        let dyc = args.pop()?;

                        parce_data.y += dya;
                        let xa = parce_data.x;
                        let ya = parce_data.y;
                        command += &format!(" {}", dya);

                        parce_data.y += dyb;
                        parce_data.x += dxb;
                        let xb = parce_data.x;
                        let yb = parce_data.y;
                        command += &format!(" {}", dxb);
                        command += &format!(" {}", dyb);

                        // parce_data.x += dx1;
                        parce_data.y += dyc;
                        let xc = parce_data.x;
                        let yc = parce_data.y;
                        command += &format!(" {}", dyc);
                        parce_data
                            .commands
                            .as_mut()
                            .operations
                            .push(Operation::C(xa, ya, xb, yb, xc, yc));
                    }
                    parce_data.commands.as_mut().commands.push(command);
                }

                14 => {
                    if 1 <= parce_data.stacks.len() && parce_data.is_first == 0 {
                        parce_data.width = parce_data.stacks.pop();
                        let width = parce_data.width;
                        parce_data
                            .commands
                            .as_mut()
                            .commands
                            .push(format!("width {:?}", width));
                    }
                    parce_data
                        .commands
                        .as_mut()
                        .commands
                        .push("endchar".to_string());
                    parce_data.commands.as_mut().operations.push(Operation::Z);
                    return Some(());
                }
                12 => {
                    let b1 = data[i];
                    i += 1;
                    match b1 {
                        35 => {
                            // flex |- dx1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 dx6 dy6 fd flex (12 35) |-
                            let mut command = "flex".to_string();
                            let fd = parce_data.stacks.pop()?;
                            let dy6 = parce_data.stacks.pop()?;
                            let dx6 = parce_data.stacks.pop()?;
                            let dy5 = parce_data.stacks.pop()?;
                            let dx5 = parce_data.stacks.pop()?;
                            let dy4 = parce_data.stacks.pop()?;
                            let dx4 = parce_data.stacks.pop()?;
                            let dy3 = parce_data.stacks.pop()?;
                            let dx3 = parce_data.stacks.pop()?;
                            let dy2 = parce_data.stacks.pop()?;
                            let dx2 = parce_data.stacks.pop()?;
                            let dy1 = parce_data.stacks.pop()?;
                            let dx1 = parce_data.stacks.pop()?;
                            let mut xx = parce_data.x;
                            let mut yy = parce_data.y;
                            xx += dx1;
                            yy += dy1;
                            let xa = xx;
                            let ya = yy;
                            xx += dx2;
                            yy += dy2;
                            let xb = xx;
                            let yb = yy;
                            xx += dx3;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            yy += dy3;
                            let xc = xx;
                            let yc = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));
                            xx += dx4;
                            yy += dy4;
                            let xd = xx;
                            let yd = yy;
                            xx += dx5;
                            yy += dy5;
                            let xe = xx;
                            let ye = yy;
                            xx += dx6;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            yy += dy6;
                            let xf = xx;
                            let yf = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));

                            parce_data.x += xx;
                            parce_data.y += yy;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {} {} {} {} fd {}\n",
                                dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, dx6, dy6, fd
                            );
                        }
                        34 => {
                            // hflex |- dy1 dx2 dy2 dx3 dx4 dx5 dx6 hflex (12 34) |
                            let mut command = "hflex".to_string();
                            let dx6 = parce_data.stacks.pop()?;
                            let dx5 = parce_data.stacks.pop()?;
                            let dx4 = parce_data.stacks.pop()?;
                            let dx3 = parce_data.stacks.pop()?;
                            let dy2 = parce_data.stacks.pop()?;
                            let dx2 = parce_data.stacks.pop()?;
                            let dy1 = parce_data.stacks.pop()?;
                            let mut xx = parce_data.x;
                            let mut yy = parce_data.y;
                            yy += dy1;
                            let xa = xx;
                            let ya = yy;
                            xx += dx2;
                            yy += dy2;
                            let xb = xx;
                            let yb = yy;
                            xx += dx3;
                            let xc = xx;
                            let yc = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));
                            xx += dx4;
                            let xd = xx;
                            let yd = yy;
                            xx += dx5;
                            let xe = xx;
                            let ye = yy;
                            xx += dx6;
                            let xf = xx;
                            let yf = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));

                            parce_data.x = xx;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            parce_data.y = yy;

                            command += &format!(
                                " {} {} {} {} {} {} {}\n",
                                dy1, dx2, dy2, dx3, dx4, dx5, dx6
                            );
                            parce_data.commands.as_mut().commands.push(command);
                        }
                        36 => {
                            // hflex1 |- dx1 dy1 dx2 dy2 dx3 dx4 dx5 dy5 dx6 hflex1 (12 36) |
                            let mut command = "hflex1".to_string();
                            let dx6 = parce_data.stacks.pop()?;
                            let dy5 = parce_data.stacks.pop()?;
                            let dx5 = parce_data.stacks.pop()?;
                            let dx4 = parce_data.stacks.pop()?;
                            let dx3 = parce_data.stacks.pop()?;
                            let dy2 = parce_data.stacks.pop()?;
                            let dx2 = parce_data.stacks.pop()?;
                            let dy1 = parce_data.stacks.pop()?;
                            let dx1 = parce_data.stacks.pop()?;
                            let mut xx = parce_data.x;
                            let mut yy = parce_data.y;
                            xx += dx1;
                            yy += dy1;
                            let xa = xx;
                            let ya = yy;
                            xx += dx2;
                            yy += dy2;
                            let xb = xx;
                            let yb = yy;
                            xx += dx3;
                            let xc = xx;
                            let yc = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));
                            xx += dx4;
                            let xd = xx;
                            let yd = yy;
                            xx += dx5;
                            yy += dy5;
                            let xe = xx;
                            let ye = yy;
                            xx += dx6;
                            let xf = xx;
                            let yf = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));

                            parce_data.x = xx;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            parce_data.y = yy;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {}\n",
                                dx1, dy1, dx2, dy2, dx3, dx4, dx5, dy5, dx6
                            );
                            parce_data.commands.as_mut().commands.push(command);
                        }
                        37 => {
                            // flex1 |- dx1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 d6 flex1 (12 37) |-
                            let mut command = "flex1".to_string();
                            let d6 = parce_data.stacks.pop()?;
                            let dy5 = parce_data.stacks.pop()?;
                            let dx5 = parce_data.stacks.pop()?;
                            let dy4 = parce_data.stacks.pop()?;
                            let dx4 = parce_data.stacks.pop()?;
                            let dy3 = parce_data.stacks.pop()?;
                            let dx3 = parce_data.stacks.pop()?;
                            let dy2 = parce_data.stacks.pop()?;
                            let dx2 = parce_data.stacks.pop()?;
                            let dy1 = parce_data.stacks.pop()?;
                            let dx1 = parce_data.stacks.pop()?;
                            let mut xx = parce_data.x;
                            let mut yy = parce_data.y;
                            xx += dx1;
                            yy += dy1;
                            let xa = xx;
                            let ya = yy;
                            xx += dx2;
                            yy += dy2;
                            let xb = xx;
                            let yb = yy;
                            xx += dx3;
                            yy += dy3;
                            let xc = xx;
                            let yc = yy;
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xa, ya, xb, yb, xc, yc));
                            xx += dx4;
                            yy += dy4;
                            let xd = xx;
                            let yd = yy;
                            xx += dx5;
                            yy += dy5;
                            let xe = xx;
                            let ye = yy;
                            let _x = dx1 + dx2 + dx3 + dx4 + dx5;
                            let _y = dy1 + dy2 + dy3 + dy4 + dy5;
                            if _x < _y {
                                xx += d6;
                            } else {
                                yy += d6;
                            }
                            let xf = xx;
                            let yf = yy;

                            parce_data.x = xx;
                            parce_data.min_x = parce_data.min_x.min(parce_data.x);
                            parce_data.y = yy;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {} {} {}\n",
                                dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, d6
                            );
                            parce_data
                                .commands
                                .as_mut()
                                .operations
                                .push(Operation::C(xd, yd, xe, ye, xf, yf));
                        }

                        19 => {
                            // abs
                            let number = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("abs {}", number));
                            parce_data.stacks.push(number.abs());
                        }
                        10 => {
                            // add
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("add {} {}", num1, num2));
                            parce_data.stacks.push(num1 + num2);
                        }
                        11 => {
                            // sub
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("sub {} {}", num1, num2));
                            parce_data.stacks.push(num1 - num2);
                        }
                        12 => {
                            // div
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("div {} {}", num1, num2));
                            parce_data.stacks.push(num1 / num2);
                        }
                        14 => {
                            // neg
                            let num = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("neg {}", num));
                            parce_data.stacks.push(-num);
                        }
                        23 => {
                            // random
                            // random 0.0 - 1.0
                            // need rand crate
                            // let num = rand::random::<f64>();
                            let num = 0.5;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("random {}", num));
                            parce_data.stacks.push(num);
                        }
                        24 => {
                            // mul
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("mul {} {}", num1, num2));
                            parce_data.stacks.push(num1 * num2);
                        }
                        26 => {
                            // sqrt
                            let num = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("sqrt {}", num));
                            parce_data.stacks.push(num.sqrt());
                        }
                        18 => {
                            // drop
                            parce_data.stacks.pop();
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push("drop".to_string());
                        }
                        29 => {
                            // index
                            let index = parce_data.stacks.pop()?;
                            let num = parce_data.stacks[parce_data.stacks.len() - index as usize];
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("index {} {}", index, num));
                            parce_data.stacks.push(num);
                        }
                        30 => {
                            // roll
                            let index = parce_data.stacks.pop()?;
                            let count = parce_data.stacks.pop()?;
                            let mut new_stacks = Vec::new();
                            for _ in 0..count as usize {
                                let num = parce_data.stacks.pop()?;
                                parce_data.stacks.push(num);
                            }
                            for _ in 0..count as usize {
                                let num = new_stacks.pop()?;
                                parce_data.stacks.push(num);
                            }
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("roll {} {}", index, count));
                        }
                        27 => {
                            // dup
                            let num = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("dup {}", num));
                            parce_data.stacks.push(num);
                            parce_data.stacks.push(num);
                        }

                        20 => {
                            // put
                            let index = parce_data.stacks.pop()?;
                            let num = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("put {} {}", index, num));
                            parce_data.stacks[index as usize] = num;
                        }
                        21 => {
                            // get
                            let index = parce_data.stacks.pop()?;
                            let num = parce_data.stacks[index as usize];
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("get {} {}", index, num));
                            parce_data.stacks.push(num);
                        }
                        3 => {
                            // and
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("and {} {}", num1, num2));
                            let num = if num1 == 0.0 || num2 == 0.0 { 0 } else { 1 };
                            parce_data.stacks.push(num as f64);
                        }
                        4 => {
                            // or
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("or {} {}", num1, num2));
                            let num = if num1 == 0.0 && num2 == 0.0 { 0 } else { 1 };
                            parce_data.stacks.push(num as f64);
                        }
                        5 => {
                            // not
                            let num = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("not {}", num));
                            let num = if num == 0.0 { 1 } else { 0 };
                            parce_data.stacks.push(num as f64);
                        }
                        15 => {
                            // eq
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("eq {} {}", num1, num2));
                            let num = if num1 == num2 { 1 } else { 0 };
                            parce_data.stacks.push(num as f64);
                        }
                        22 => {
                            // if else
                            let num2 = parce_data.stacks.pop()?;
                            let num1 = parce_data.stacks.pop()?;
                            let res2 = parce_data.stacks.pop()?;
                            let res1 = parce_data.stacks.pop()?;
                            parce_data
                                .commands
                                .as_mut()
                                .commands
                                .push(format!("ifelse {} {} {} {}", num1, num2, res1, res2));
                            let num = if num1 > num2 { res1 } else { res2 };
                            parce_data.stacks.push(num);
                        }

                        _ => { // reserved
                        }
                    }
                }
                10 => {
                    // call callsubr
                    let mut command = "callsubr".to_string();
                    let num = parce_data.stacks.pop()? as isize;
                    if let Some(subr) = self.subr.as_ref() {
                        let no = if subr.data.data.len() <= 1238 {
                            num + 107
                        } else if subr.data.data.len() <= 33899 {
                            num + 1131
                        } else {
                            num + 32768
                        };
                        command += &format!(" {} {}\n", no, num);
                        parce_data.commands.as_mut().commands.push(command);
                        let data = &subr.data.data[no as usize];
                        self.parse(data, parce_data)?;
                    } else {
                        #[cfg(debug_assertions)]
                        {
                            println!("no subr");
                        }
                        break;
                    }
                }
                29 => {
                    // callgsubr
                    let mut command = "callgsubr\n".to_string();
                    let num = parce_data.stacks.pop()? as isize;
                    if let Some(subr) = self.gsubr.as_ref() {
                        let no = if subr.data.data.len() <= 1238 {
                            num + 107
                        } else if subr.data.data.len() <= 33899 {
                            num + 1131
                        } else {
                            num + 32768
                        };
                        command += &format!(" {} {}\n", no, num);
                        parce_data.commands.as_mut().commands.push(command);
                        let data = &subr.data.data[no as usize];
                        self.parse(data, parce_data)?;
                    } else {
                        #[cfg(debug_assertions)]
                        {
                            println!("no subr");
                        }
                        break;
                    }
                }
                11 => {
                    // return
                    let command = "return\n".to_string();
                    parce_data.commands.as_mut().commands.push(command);
                    return Some(());
                }
                28 => {
                    let b0 = data[i];
                    let b1 = data[i + 1];
                    let value = i16::from_be_bytes([b0, b1]) as i32;
                    parce_data.stacks.push(value as f64);
                    i += 2;
                }
                32..=246 => {
                    let value = b0 as i32 - 139;
                    parce_data.stacks.push(value as f64);
                }
                247..=250 => {
                    let b1 = data[i];
                    let value = (b0 as i32 - 247) * 256 + b1 as i32 + 108;
                    parce_data.stacks.push(value as f64);
                    i += 1;
                }
                251..=254 => {
                    let b1 = data[i];
                    let value = -(b0 as i32 - 251) * 256 - b1 as i32 - 108;
                    parce_data.stacks.push(value as f64);
                    i += 1;
                }
                255 => {
                    if data.len() - i >= 4 {
                        let b1 = data[i];
                        let b2 = data[i + 1];
                        let b3 = data[i + 2];
                        let b4 = data[i + 3];
                        let value = i16::from_be_bytes([b1, b2]) as f64;
                        let frac = u16::from_be_bytes([b3, b4]) as f64;
                        let value = value + frac / 65536.0;
                        parce_data.stacks.push(value);
                        i += 4;
                    }
                }
                _ => {
                    let command = format!("unknown {}", b0);
                    parce_data.commands.as_mut().commands.push(command);
                }
            }
        }
        parce_data.commands.as_mut().operations.push(Operation::Z);
        Some(())
    }

    pub(crate) fn to_svg(
        &self,
        gid: usize,
        fontsize: f64,
        fontunit: &str,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> Result<String, std::io::Error> {
        if gid >= self.char_string.data.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("gid {} is not found", gid),
            ));
        }
        let data = &self.char_string.data.data[gid as usize];
        Ok(self.parse_data(gid, &data, fontsize, fontunit, layout, sx, sy, true))
    }

    fn get_svg_header(
        &self,
        gid: usize,
        parce_data: &ParcePack,
        fontsize: f64,
        fontunit: &str,
        layout: &FontLayout,
    ) -> String {
        match layout {
            FontLayout::Horizontal(layout) => {
                let lsb = layout.lsb as f64;
                let descender = layout.descender;
                let accender = layout.accender;
                let line_gap = layout.line_gap;
                let advance_width = layout.advance_width as f64;
                // let width = self.width + parce_data.width;
                let height = self.bbox[3] - self.bbox[1] as f64;
                let y_scale = (height + line_gap as f64) / height;
                let width = advance_width / height * fontsize;
                let h = format!("{}{}", fontsize, fontunit);
                let w = format!("{}{}", width / y_scale, fontunit);
                let self_width = if let Some(width) = parce_data.width {
                    self.width + width
                } else {
                    self.default_width
                };

                let mut svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" ".to_string();
                svg += &format!(
                    " width=\"{}\" height=\"{}\" viewbox=\"{} {} {} {}\">\n",
                    w,
                    h,
                    0, // parce_data.min_x,
                    self.bbox[1],
                    self_width,
                    (self.bbox[3] - self.bbox[1]) * y_scale
                );
                #[cfg(debug_assertions)]
                {
                    svg += &format!(
                        "<!-- gid {} width {} {} {} height {} -->\n",
                        gid, self.width, self_width, parce_data.min_x, height
                    );
                    svg += &format!("<!-- bbox {:?} -->\n", self.bbox);
                    svg += &format!(
                        "<!-- advance_width {} accender {} descender {} line_gap {} {} -->\n",
                        layout.advance_width, accender, descender, line_gap, y_scale
                    );
                }
                svg
            }
            FontLayout::Vertical(layout) => {
                let descender = layout.descender;
                let accender = layout.accender;
                let line_gap = layout.line_gap;
                let advance_height = layout.advance_height as f64;
                // let width = self.width + parce_data.width;
                let height = self.bbox[3] - self.bbox[1] as f64;
                let width = (descender + accender) as f64 / advance_height * fontsize;
                let h = format!("{}{}", fontsize, fontunit);
                let w = format!("{}{}", width, fontunit);
                let self_width = if let Some(width) = parce_data.width {
                    self.width + width
                } else {
                    self.default_width
                };

                let mut svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" ".to_string();
                svg += &format!(
                    " width=\"{}\" height=\"{}\" viewbox=\"{} {} {} {}\">\n",
                    w,
                    h,
                    0, // parce_data.min_x,
                    self.bbox[1],
                    self_width,
                    (self.bbox[3] - self.bbox[1])
                );
                #[cfg(debug_assertions)]
                {
                    svg += &format!(
                        "<!-- gid {} width {} {} {} height {} -->\n",
                        gid, self.width, self_width, parce_data.min_x, height
                    );
                    svg += &format!("<!-- bbox {:?} -->\n", self.bbox);
                    svg += &format!(
                        "<!-- advance_height {} accender {} descender {} line_gap {}-->\n",
                        layout.advance_height, accender, descender, line_gap
                    );
                }
                svg
            }
            FontLayout::Unknown => "".to_string(),
        }
    }

    fn parse_data(
        &self,
        gid: usize,
        data: &[u8],
        fontsize: f64,
        fontunit: &str,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
        is_svg: bool,
    ) -> String {
        let commands = Box::new(Commands::new());
        let stacks = Box::new(Vec::with_capacity(48)); // CFF1 max stack depth 48
        let mut parce_data = ParcePack {
            x: 0.0,
            y: 0.0,
            hints: 0,
            min_x: 0.0,
            width: None,
            commands,
            stacks,
            is_first: 0,
        };

        self.parse(data, &mut parce_data);
        let commands = &parce_data.commands;
        if is_svg {
            let mut svg = self.get_svg_header(gid, &parce_data, fontsize, fontunit, layout);
            let y_pos = self.bbox[3] + self.bbox[1];
            svg += "<path d=\"";
            for operation in commands.operations.iter() {
                match operation {
                    Operation::M(x, y) => {
                        svg += &format!("M {} {}\n", x + sx, y_pos - y + sy);
                    }
                    Operation::L(x, y) => {
                        svg += &format!("L {} {}\n", x + sx, y_pos - y + sy);
                    }
                    Operation::C(xa, ya, xb, yb, xc, yc) => {
                        svg += &format!(
                            "C {} {} {} {} {} {}\n",
                            xa + sx,
                            y_pos - ya + sy,
                            xb + sx,
                            y_pos - yb + sy,
                            xc + sx,
                            y_pos - yc + sy
                        );
                    }
                    Operation::Z => {
                        svg += "Z\n";
                    }
                }
            }
            svg += "\"/></svg>";
            svg
        } else {
            let mut string = String::new();
            for command in commands.commands.iter() {
                string += &format!("{}\n", command);
            }
            string
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FDSelect {
    fsds: Vec<u8>,
}

impl FDSelect {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        n_glyphs: usize,
    ) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.seek(SeekFrom::Start(offset as u64))?;
        let format = reader.read_u8()?;
        let mut fsds = Vec::new();
        match format {
            0 => {
                for _ in 0..n_glyphs {
                    let fsd = reader.read_u8()?;
                    fsds.push(fsd);
                }
            }
            3 => {
                let n_ranges = reader.read_u16_be()?;
                let mut last_gid = 0;
                let is_first_gid = reader.read_u16_be()?;
                for _ in 0..n_ranges {
                    let fd = reader.read_u8()?;
                    let sentinel_gid = reader.read_u16_be()?;
                    for _ in last_gid..sentinel_gid {
                        fsds.push(fd);
                    }
                    last_gid = is_first_gid;
                }
            }
            _ => return Err("Illegal format".into()),
        }
        Ok(Self { fsds })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Charsets {
    format: u8,
    sid: Vec<u16>,
}

impl Charsets {
    fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
        n_glyphs: u32,
    ) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset)).unwrap();
        let format = reader.read_u8().unwrap();
        let mut charsets = Self {
            format,
            sid: Vec::new(),
        };
        charsets.sid.push(0);

        match format {
            0 => charsets.parse_format0(reader, n_glyphs)?,
            1..=2 => charsets.parse_format1(reader, n_glyphs)?,
            _ => return Err("Illegal format".into()),
        }
        Ok(charsets)
    }

    fn parse_format0<R: BinaryReader>(
        &mut self,
        reader: &mut R,
        n_glyphs: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut i = 1;
        for _ in 0..n_glyphs as usize - 1 {
            let sid = reader.read_u16_be()?;
            self.sid.push(sid);
            i += 2;
        }
        Ok(())
    }

    fn parse_format1<R: BinaryReader>(
        &mut self,
        reader: &mut R,
        n_glyphs: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut i = 1;
        while i < n_glyphs as usize - 1 {
            let mut sid = reader.read_u16_be()?;
            let n_left = if self.format == 1 {
                reader.read_u8()? as usize
            } else {
                reader.read_u16_be()? as usize
            };
            for _ in 0..=n_left {
                self.sid.push(sid);
                i += 1;
                sid += 1;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Operand {
    Integer(i32),
    Real(f64),
}

pub(crate) fn operand_encoding(b: &[u8]) -> Result<(Operand, usize), Box<dyn Error>> {
    if b.is_empty() {
        return Err("empty".into());
    }
    let b0 = b[0];
    if (32..=246).contains(&b0) {
        return Ok((Operand::Integer(b0 as i32 - 139), 1));
    }
    if (247..=250).contains(&b0) {
        if b.len() < 2 {
            return Err("buffer shotage".into());
        }
        let b1 = b[1];
        return Ok((
            Operand::Integer((b0 as i32 - 247) * 256 + b1 as i32 + 108),
            2,
        ));
    }
    if (251..=254).contains(&b0) {
        if b.len() < 2 {
            return Err("buffer shotage".into());
        }
        let b1 = b[1];
        return Ok((
            Operand::Integer(-(b0 as i32 - 251) * 256 - b1 as i32 - 108),
            2,
        ));
    }
    if b0 == 28 {
        if b.len() < 3 {
            return Err("buffer shotage".into());
        }
        let value = i16::from_be_bytes([b[1], b[2]]) as i32;
        return Ok((Operand::Integer(value), 3));
    }
    if b0 == 29 {
        if b.len() < 5 {
            return Err("buffer shotage".into());
        }
        let value = i32::from_be_bytes([b[1], b[2], b[3], b[4]]);
        return Ok((Operand::Integer(value), 5));
    }
    if b0 == 30 {
        let mut r = Vec::new();
        let mut x = 1;
        for i in 1..b.len() {
            let b = b[i];

            x += 1;
            let r0 = b >> 4;
            let r1 = b & 0x0f;
            match r0 {
                0 => r.push('0'),
                1 => r.push('1'),
                2 => r.push('2'),
                3 => r.push('3'),
                4 => r.push('4'),
                5 => r.push('5'),
                6 => r.push('6'),
                7 => r.push('7'),
                8 => r.push('8'),
                9 => r.push('9'),
                0xa => r.push('.'),
                0xb => r.push('E'),
                0xc => {
                    r.push('E');
                    r.push('-');
                }
                0xd => {}
                0xe => r.push('-'),
                0xf => {
                    break;
                }
                _ => {}
            }
            match r1 {
                0 => r.push('0'),
                1 => r.push('1'),
                2 => r.push('2'),
                3 => r.push('3'),
                4 => r.push('4'),
                5 => r.push('5'),
                6 => r.push('6'),
                7 => r.push('7'),
                8 => r.push('8'),
                9 => r.push('9'),
                0xa => r.push('.'),
                0xb => r.push('E'),
                0xc => {
                    r.push('E');
                    r.push('-');
                }
                0xd => {}
                0xe => r.push('-'),
                0xf => {
                    break;
                }
                _ => {}
            }
        }
        let str = r.iter().collect::<String>();
        match str.parse::<f64>() {
            Ok(f64value) => return Ok((Operand::Real(f64value), x)),
            Err(_) => return Err("Illegal value".into()),
        }
    }
    Err("Illegal value".into())
}

#[derive(Debug, Clone)]
pub(crate) struct Header {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) hdr_size: u8,
    pub(crate) off_size: u8,
    #[cfg(feature = "cff2")]
    pub(crate) top_dict_index_offset: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Index {
    pub(crate) data: Vec<Vec<u8>>,
}

type PrivateDict = Dict;

#[derive(Debug, Clone)]
pub(crate) struct Dict {
    pub(crate) entries: HashMap<u16, Vec<Operand>>,
}

impl Dict {
    pub(crate) fn parse(buffer: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut entries = HashMap::new();
        let mut i = 0;
        let mut operands = Vec::new();
        while i < buffer.len() {
            if buffer.len() <= i {
                break;
            }
            let b = buffer[i];
            if b == 12 {
                let operator = (b as u16) << 8 | buffer[i + 1] as u16;
                entries.insert(operator, operands);
                operands = Vec::new();
                i += 2;
            } else if b <= 27 {
                let operator = b as u16;
                entries.insert(operator, operands);
                operands = Vec::new();
                i += 1;
            }
            if i >= buffer.len() {
                break;
            }

            let (operand, len) = operand_encoding(&buffer[i..])?;
            operands.push(operand);
            i += len;
        }

        Ok(Self { entries })
    }

    pub(crate) fn get(&self, key: u16) -> Option<Vec<Operand>> {
        self.entries.get(&key).cloned()
    }

    pub(crate) fn get_sid(&self, key1: u8, key2: u8) -> Option<i32> {
        self.get_i32(key1, key2)
    }

    pub(crate) fn get_i32(&self, key1: u8, key2: u8) -> Option<i32> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                if operands.len() != 1 {
                    return None;
                }
                match operands[0] {
                    Operand::Integer(value) => Some(value),
                    Operand::Real(value) => Some(value as i32),
                    _ => None,
                }
            }
            None => None,
        }
    }

    pub fn get_f64(&self, key1: u8, key2: u8) -> Option<f64> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                if operands.len() != 1 {
                    return None;
                }
                match operands[0] {
                    Operand::Integer(value) => Some(value as f64),
                    Operand::Real(value) => Some(value),
                    _ => None,
                }
            }
            None => None,
        }
    }

    pub(crate) fn get_i32_array(&self, key1: u8, key2: u8) -> Option<Vec<i32>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                let mut r = Vec::new();
                for operand in operands {
                    match operand {
                        Operand::Integer(value) => r.push(*value),
                        Operand::Real(value) => r.push(*value as i32),
                        _ => return None,
                    }
                }
                Some(r)
            }
            None => None,
        }
    }

    pub(crate) fn get_f64_array(&self, key1: u8, key2: u8) -> Option<Vec<f64>> {
        let key = (key1 as u16) << 8 | key2 as u16;
        match self.entries.get(&key) {
            Some(operands) => {
                let mut r = Vec::new();
                for operand in operands {
                    match operand {
                        Operand::Integer(value) => r.push(*value as f64),
                        Operand::Real(value) => r.push(*value),
                        _ => return None,
                    }
                }
                Some(r)
            }
            None => None,
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = String::new();
        for (key, operands) in &self.entries {
            if *key < 256 {
                string += &format!("{} [", key);
            } else {
                string += &format!("{} {} [", key >> 8, key & 0xff);
            }
            for operand in operands {
                match operand {
                    Operand::Integer(value) => string += &format!("{},", value),
                    Operand::Real(value) => string += &format!("{},", value),
                }
            }
            string += "]\n";
        }
        string
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CharString {
    pub(crate) data: Index,
}

impl CharString {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u64,
    ) -> Result<Self, Box<dyn Error>> {
        if offset > 0 {
            reader.seek(SeekFrom::Start(offset))?;
        }
        let index = Index::parse(reader)?;
        Ok(Self { data: index })
    }
}

impl Header {
    pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn Error>> {
        let major = r.read_u8()?; // CFF = 1 / CFF2 = 2
        let minor = r.read_u8()?;
        let hdr_size = r.read_u8()?; // CFF = 4 / CFF2 = 5
        let off_size = // CFF only
        if major == 1 {
            r.read_u8()?
        } else {
            #[cfg(not(feature = "cff2"))]
            r.skip_ptr(hdr_size as usize - 3)?;
            0
        };
        #[cfg(feature = "cff2")]
        if major == 2 {
            let top_dict_index_offset = r.read_u32_be()?;
            return Ok(Self {
                major,
                minor,
                hdr_size,
                off_size,
                top_dict_index_offset,
            });
        }
        Ok(Self {
            major,
            minor,
            hdr_size,
            off_size,
            #[cfg(feature = "cff2")]
            top_dict_index_offset: 0,
        })
    }
}

impl Index {
    pub(crate) fn parse<R: BinaryReader>(r: &mut R) -> Result<Self, Box<dyn Error>> {
        let count = r.read_u16_be()?;
        if count == 0 {
            return Ok(Self { data: Vec::new() });
        }
        let off_size = r.read_u8()?;

        let mut offsets = Vec::new();
        for _ in 0..count + 1 {
            match off_size {
                1 => offsets.push(r.read_u8()? as u32),
                2 => offsets.push(r.read_u16_be()? as u32),
                3 => {
                    let b0 = r.read_u8()?;
                    let b1 = r.read_u16_be()?;
                    offsets.push(((b0 as u32) << 16) + (b1 as u32));
                }
                4 => offsets.push(r.read_u32_be()?),
                _ => {
                    return Err("Illegal offset size".into());
                }
            }
        }

        let mut data = Vec::new();
        for i in 0..count {
            let start = offsets[i as usize] as usize;
            let end = offsets[i as usize + 1] as usize;
            let buf = r.read_bytes_as_vec(end - start)?;
            data.push(buf);
        }
        Ok(Self { data })
    }
}
