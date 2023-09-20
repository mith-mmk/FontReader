// CFF is Adobe Type 1 font format, which is a compact binary format.

use std::{collections::HashMap, error::Error, io::SeekFrom};

// Compare this snippet from src/outline/cff.rs:
use bin_rs::reader::BinaryReader;

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
type SID = u16;

#[derive(Debug, Clone)]
pub(crate) struct CFF {
    pub(crate) header: Header,
    pub(crate) names: Vec<String>,
    pub(crate) top_dict: Dict, // TopDict
    pub(crate) bbox: [f64;4],
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
}

impl CFF {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        _length: u32,
    ) -> Result<Self, Box<dyn Error>> {

        reader.seek(SeekFrom::Start(offset as u64))?;
        let mut bbox = [0.0 ,0.0, 1000.0, 1000.0];

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
                None    // CFF2
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
        if let Some(some_bbox) = opt_bbox {
            bbox = [some_bbox[0],  some_bbox[1], some_bbox[2], some_bbox[3]];
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
        let charsets_offset = charsets_offset as u32 + offset;

        let charsets = Charsets::new(reader, charsets_offset, n_glyphs as u32)?;
        let char_strings_offset = char_strings_offset as u32 + offset;
        let char_string = CharString::new(reader, char_strings_offset as u32)?;


        let private = top_dict.get_i32_array(0, 18);
        let mut subr = None;
        let private_dict = if let Some(private) = private {
            let _ = private[0] as u32; //
            let private_dict_offset = private[1] as u32 + offset;
            reader.seek(SeekFrom::Start(private_dict_offset as u64))?;
            let buffer = reader.read_bytes_as_vec(private[0] as usize)?;
            let private_dict = Dict::parse(&buffer)?;
            #[cfg(debug_assertions)]
            {
                println!("private_dict: {}", private_dict.to_string());
            }
            if let Some(sub_offset) = private_dict.get_i32(0, 19) {
                let subr_offset = sub_offset as u32 + private_dict_offset;
                let subrtn = CharString::new(reader, subr_offset)?;
                subr = Some(subrtn)
            }

            Some(private_dict)
        } else {
            None
        };
        if ros.is_some() {
            let fd_array_offset = (fd_array_offset.unwrap() as u32 + offset) as u64;
            reader.seek(SeekFrom::Start(fd_array_offset))?;
            let fd_arrays = Index::parse(reader)?;
            let font_dict = Dict::parse(&fd_arrays.data[0])?;
            #[cfg(debug_assertions)]
            {
                println!("font_dict:\n{}", font_dict.to_string());
            }

            if let Some(private) = font_dict.get_i32_array(0, 18) {
                let private_dict_offset = private[1] as u32 + offset as u32;
                reader.seek(SeekFrom::Start(private_dict_offset as u64))?;
                let buffer = reader.read_bytes_as_vec(private[0] as usize);
                let private_dict = Dict::parse(&buffer?)?;
                #[cfg(debug_assertions)]
                {
                    println!("private_dict: {}", private_dict.to_string());
                }
                let subr_offset = private_dict.get_i32(0, 19);
                if let Some(subr_offset) = subr_offset {
                    let subr_offset = subr_offset as u32 + private_dict_offset as u32;
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
        })
    }

    pub fn set_bbox(&mut self, min_x: f64, min_y: f64, max_x:f64, max_y :f64) {
        self.bbox = [min_x, min_y, max_x, max_y];
    }

    pub fn to_code(&self, gid: usize, width: f64) -> String {
        let cid = self.charsets.sid[gid as usize];
        println!("gid {} cid {}",gid, cid);
        let data = &self.char_string.data.data[gid as usize];
        let width = if width == 0.0 {
            self.top_dict.get_f64(0, 15).unwrap() // nomarl width
        } else {
            width as f64
        };
        self.parse_data(data, width, &mut Vec::new(), false)
    }

    fn parse_data(
        &self,
        data: &[u8],
        width: f64,
        stacks: &mut Vec<f64>,
        is_svg: bool,
    ) -> String {

        let mut width = width;
        //        let cid = self.charsets.sid[gid as usize];
        /*
           0..=11 =>  operators
           12 => escape get next byte
           13..=18 => operators
           19 => hintmask
           20 => cntrmask
           21..=27 => operators
           28 => number get next 2 bytes
           29..=31 => operators
           32..=246 => number - 139
           247..=250 => number (b0 - 247) * 256 + b1 + 108
           251..=254 => number -(b0 - 251) * 256 - b1 - 108
           255 => real number get next 4bytes 16dot16
        */
        let mut x = 0.0;
        let mut y = 0.0;
        let accender = self.bbox[3];
        let mut string = String::new();
        let mut svg = String::new();
        // let mut stacks: Vec<f64> = Vec::new();
        let mut first = if width == 0.0 { false } else { true };
        let mut hints = 0;
        let mut i = 0;
        // println!("data.len() = {}, {}", data.len(), i);
        while i < data.len() {
            let b0 = data[i];
            i += 1;
            match b0 {
                1 => {
                    // hstem |- y dy {dya dyb}* hstem (1) |
                    let mut command = "hstem".to_string();
                    let mut args = Vec::new();
                    args.push(stacks.pop().unwrap());
                    args.push(stacks.pop().unwrap());

                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }

                    y = args.pop().unwrap();
                    command += &format!(" {}", y);
                    if 1 <= args.len() {
                        let dy = args.pop().unwrap();
                        y += dy;
                        command += &format!(" {}", y);
                    }
                    while 2 <= args.len() {
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!(" {}", y);
                        let dyb = args.pop().unwrap();
                        y += dyb;
                        command += &format!(" {}", y);
                    }
                    command += "\n";
                    string += &command;
                }
                3 => {
                    // vstem |- v dx {dxa dxb}* vstem (3) |

                    let mut args = Vec::new();
                    args.push(stacks.pop().unwrap());
                    args.push(stacks.pop().unwrap());
                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }
                    hints += args.len();

                    let mut command = "vstem".to_string();
                    let mut x = args.pop().unwrap();
                    command += &format!(" {}", x);
                    if 1 <= args.len() {
                        let dx = args.pop().unwrap();
                        x += dx;
                        command += &format!(" {}", x);
                    }
                    while 2 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;
                        command += &format!(" {}", x);
                        let dxb = args.pop().unwrap();
                        x += dxb;
                        command += &format!(" {}", x);
                    }
                    command += "\n";
                    string += &command;
                }
                18 => {
                    // hstemhm |- y dy {dya dyb}* hstemhm (18) |-
                    let mut args = Vec::new();
                    args.push(stacks.pop().unwrap());
                    args.push(stacks.pop().unwrap());
                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }
                    hints += args.len();

                    let mut command = "hstemhm".to_string();
                    let mut y = args.pop().unwrap();
                    command += &format!(" {}", y);
                    let dy = args.pop().unwrap();
                    y += dy;
                    command += &format!(" {}", y);
                    while 2 <= args.len() {
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!(" {}", y);
                        let dyb = args.pop().unwrap();
                        y += dyb;
                        command += &format!(" {}", y);
                    }
                    command += "\n";
                    string += &command;
                }
                23 => {
                    // vstemhm |- x dx {dxa dxb}* vstemhm (23) |-
                    let mut args = Vec::new();
                    args.push(stacks.pop().unwrap());
                    args.push(stacks.pop().unwrap());
                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }
                    hints += args.len();
                    let mut command = "vstemhm".to_string();
                    let mut x = args.pop().unwrap();
                    command += &format!(" {}", x);
                    let dx = args.pop().unwrap();
                    x += dx;
                    command += &format!(" {}", x);
                    while 2 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;
                        command += &format!(" {}", x);
                        let dxb = args.pop().unwrap();
                        x += dxb;
                        command += &format!(" {}", x);
                        hints += 2;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i);
                }
                19 => {
                    // hintmask |- hintmask (19 + mask) |
                    let len = (hints + 7) / 8;
                    let mut command = "hintmask".to_string();
                    for j in 0..len {
                        let mask = data[i + j];
                        command += &format!(" {:08b}", mask);
                    }
                    i += len;
                    command += "\n";
                    string += &command;
                }
                20 => {
                    // cntrmask |- cntrmask (20 + mask) |-
                    let len = (hints + 7) / 8;
                    let mut command = "cntrmask".to_string();
                    for j in 0..len {
                        let mask = data[i + j];
                        command += &format!(" {:08b}", mask);
                    }
                    i += len;
                    command += "\n";
                    string += &command;
                }

                21 => {
                    // rmoveto |- dy1 dy1 rmoveto (21) |-
                    if !first {
                        svg += "Z\n";
                    }
                    let dy = stacks.pop().unwrap();
                    let dx = stacks.pop().unwrap();
                    x += dx;
                    y += dy;
                    string += &format!("rmoveto {} {}\n", dx, dy);
                    svg += &format!("M {} {}\n", x, accender - y);

                    if 1 <= stacks.len() && first == true {
                        width += stacks.pop().unwrap();
                        first = false;
                        string += &format!("width {}\n", width);
                    }
                }
                22 => {
                    // hmoveto |- dy1 hmoveto (22) |-
                    if !first {
                        svg += "Z\n";
                    }
                    let dx = stacks.pop().unwrap();
                    x += dx;
                    string += &format!("hmoveto {}\n", dx);
                    svg += &format!("M {} {}\n", x, accender - y);
                    if 1 <= stacks.len() && first == true {
                        width += stacks.pop().unwrap();
                        first = false;
                        string += &format!("width {}\n", width);
                    }
                    string += &format!("hmoveto {}\n", dx);
                }
                4 => {
                    // vmoveto |- dy1 vmoveto (4) |-
                    if !first {
                        svg += "Z\n";
                    }
                    let dy = stacks.pop().unwrap();
                    y += dy;
                    string += &format!("vmoveto {}\n", dy);
                    svg += &format!("M {} {}\n", x, accender - y);

                    if 1 <= stacks.len() && first == true {
                        width += stacks.pop().unwrap();
                        first = false;
                        string += &format!("width {}\n", width);
                    }
                }
                5 => {
                    // rlineto |- {dxa dya}+ rlineto (5) |-
                    let mut args = Vec::new();
                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }

                    let mut command = "rlineto".to_string();
                    while 2 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;
                        command += &format!(" {}", dxa);
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!(" {}", dya);
                        svg += &format!("L {} {}\n", x, accender - y);
                    }
                    command += "\n";
                    string += &command;
                }
                6 => {
                    //  |- dy1 {dya dxb}* hlineto (6) |- odd
                    // |- {dxa dyb}+ hlineto (6) |-      even
                    let mut args = Vec::new();
                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }
                    if 1 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                    }

                    let mut command = "hlineto".to_string();
                    if args.len() % 2 == 1 {
                        let dy1 = args.pop().unwrap();
                        x += dy1;
                        command += &format!(" dy1 {}", dy1);
                        svg += &format!("L {} {}\n", x, accender - y);
                        while 2 <= args.len() {
                            let dya = args.pop().unwrap();
                            y += dya;
                            command += &format!(" {}", dya);
                            svg += &format!("L {} {}\n", x, accender - y);
                            let dxb = args.pop().unwrap();
                            x += dxb;
                            command += &format!(" {}", dxb);
                            svg += &format!("L {} {}\n", x, accender - y);
                        }
                    } else {
                        while 2 <= args.len() {
                            let dxa = args.pop().unwrap();
                            x += dxa;
                            command += &format!(" {}", dxa);
                            svg += &format!("L {} {}\n", x, accender - y);
                            let dyb = args.pop().unwrap();
                            y += dyb;
                            command += &format!(" {}", dyb);
                            svg += &format!("L {} {}\n", x, accender - y);
                        }
                    }

                    command += "\n";
                    string += &command;
                }
                7 => {
                    // vlineto - dy1 {dxa dyb}* vlineto (7) |- odd
                    // |- {dya dxb}+ vlineto (7) |-  even
                    let mut args = Vec::new();
                    while 2 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                    }
                    if 1 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                    }

                    let mut command = "vlineto".to_string();

                    if args.len() % 2 == 1 {
                        let dy1 = args.pop().unwrap();
                        y += dy1;
                        command += &format!(" dy1 {}", dy1);
                        svg += &format!("L {} {}", x, accender - y);
                        while 2 <= args.len() {
                            let dxa = args.pop().unwrap();
                            x += dxa;
                            command += &format!(" {}", dxa);
                            svg += &format!("L {} {}\n", x, accender - y);
                            let dyb = args.pop().unwrap();
                            y += dyb;
                            command += &format!(" {}", dyb);
                            svg += &format!("L {} {}\n", x, accender - y);
                        }
                    } else {
                        while 2 <= args.len() {
                            let dya = args.pop().unwrap();
                            y += dya;
                            command += &format!(" {}", dya);
                            svg += &format!("L {} {}\n", x, accender - y);
                            let dxb = args.pop().unwrap();
                            x += dxb;
                            command += &format!(" {}", dxb);
                            svg += &format!("L {} {}\n", x, accender - y);
                        }
                    }

                    command += "\n";
                    string += &command;
                }
                8 => {
                    // rrcurveto |- {dxa dya dxb dyb dxc dyc}+ rrcurveto (8) |-
                    let mut args = Vec::new();
                    while 6 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        let d5 = stacks.pop().unwrap();
                        let d6 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                        args.push(d5);
                        args.push(d6);
                    }

                    let mut command = "rrcurveto".to_string();
                    while 6 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;
                        command += &format!(" {}", dxa);
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!(" {}", dya);
                        svg += &format!("C {} {}", x, accender - y); // P(a)

                        let dxb = args.pop().unwrap();
                        x += dxb;
                        command += &format!(" {}", dxb);
                        let dyb = args.pop().unwrap();
                        y += dyb;
                        command += &format!(" {}", dyb);

                        svg += &format!(" {} {}", x, accender - y); // P(b)
                        let dxc = args.pop().unwrap();
                        x += dxc;
                        command += &format!(" {}", dxc);
                        let dyc = args.pop().unwrap();
                        y += dyc;
                        command += &format!(" {}", dyc);
                        svg += &format!(" {} {}\n", x, accender - y); // P(c)
                    }
                }
                27 => {
                    //hhcurveto|- dy1? {dxa dxb dyb dxc}+ hhcurveto (27) |-
                    let mut args = Vec::new();
                    while 4 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                    }
                    if 1 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                    }

                    let mut command = "hhcurveto".to_string();
                    if args.len() % 4 == 1 {
                        let dy1 = args.pop().unwrap();
                        command += &format!(" dy1 {}", dy1);
                        y += dy1;
                        // svg += &format!("L {} {}", x, accender - y);
                    }
                    while 4 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;

                        command += &format!(" {}", dxa);
                        svg += &format!("C {} {}", x, accender - y); // P(a)

                        let dxb = args.pop().unwrap();
                        x += dxb;
                        command += &format!(" {}", dxb);
                        let dyb = args.pop().unwrap();
                        y += dyb;
                        command += &format!(" {}", dyb);
                        svg += &format!(" {} {}", x, accender - y); // P(b)

                        let dxc = args.pop().unwrap();
                        x += dxc;

                        command += &format!(" {}", dxc);
                        svg += &format!(" {} {}\n", x, accender - y); // P(c)
                    }
                    command += "\n";
                    string += &command;
                }
                31 => {
                    // hvcurveto |- dy1 dx2 dy2 dy3 {dya dxb dyb dxc dxd dxe dye dyf}* dxf?
                    //                hvcurveto (31) |-
                    // |- {dxa dxb dyb dyc dyd dxe dye dxf}+ dyf? hvcurveto (31) |-
                    let mut args = Vec::new();
                    let mut tmp = String::new();
                    while 8 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        let d5 = stacks.pop().unwrap();
                        let d6 = stacks.pop().unwrap();
                        let d7 = stacks.pop().unwrap();
                        let d8 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                        args.push(d5);
                        args.push(d6);
                        args.push(d7);
                        args.push(d8);
                    }
                    if 4 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                    }
                    if 1 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                    }

                    let mut command = "hvcurveto".to_string();
                    if args.len() % 8 >= 4 {
                        let dy1 = args.pop().unwrap();
                        x += dy1;
                        command += &format!(" dy1 {}", dy1);
                        let dx2 = args.pop().unwrap();
                        x += dx2;
                        svg += &format!("C{} {}", x, accender - y); // P(a)

                        command += &format!(" dx2 {}", dx2);
                        let dy2 = args.pop().unwrap();
                        y += dy2;
                        command += &format!(" dy2 {}", dy2);
                        svg += &format!(" {} {}", x, accender - y); // P(b)

                        let dy3 = args.pop().unwrap();
                        y += dy3;
                        command += &format!(" dy3 {}", dy3);
                        svg += &format!(" {} {}\n", x, accender - y); // P(c)
                        let mut lp = false;
                        while 8 <= args.len() {
                            svg += &format!("{}", tmp);
                            lp = true;
                            let dya = args.pop().unwrap();
                            y += dya;
                            command += &format!(" {}", dya);
                            svg += &format!("C {} {}", x, accender - y); // P(a)
                            let dxb = args.pop().unwrap();
                            x += dxb;
                            command += &format!(" {}", dxb);
                            let dyb = args.pop().unwrap();
                            y += dyb;
                            command += &format!(" {}", dyb);
                            svg += &format!(" {} {}", x, accender - y); // P(b)
                            let dxc = args.pop().unwrap();
                            x += dxc;
                            command += &format!(" {}", dxc);
                            svg += &format!(" {} {}\n", x, accender - y); // P(c)
                            let dxd = args.pop().unwrap();
                            x += dxd;
                            command += &format!(" {}", dxd);
                            svg += &format!("C {} {}", x, accender - y); // P(d)
                            let dxe = args.pop().unwrap();
                            x += dxe;
                            command += &format!(" {}", dxe);
                            let dye = args.pop().unwrap();
                            y += dye;
                            command += &format!(" {}", dye);
                            svg += &format!(" {} {}", x, accender - y); // P(e)

                            let dyf = args.pop().unwrap();
                            y += dyf;
                            command += &format!(" {}", dyf);
                            tmp = format!(" {} {}\n", x, accender - y); // P(f)
                        }
                        if lp {
                            if 1 <= args.len() {
                                let dxf = args.pop().unwrap();
                                x += dxf;
                                command += &format!(" dxf {}", dxf);
                                svg += &format!(" {} {}\n", x, accender - y); // P(f)
                            } else {
                                svg += &format!("{}", tmp);
                            }
                        }
                    } else {
                        while 8 <= args.len() {
                            svg += &format!("{}", tmp);
                            let dxa = args.pop().unwrap();
                            x += dxa;
                            command += &format!(" {}", dxa);
                            svg += &format!("C {} {}", x, accender - y); // P(a)
                            let dxb = args.pop().unwrap();
                            x += dxb;
                            command += &format!(" {}", dxb);
                            let dyb = args.pop().unwrap();
                            y += dyb;
                            command += &format!(" {}", dyb);
                            svg += &format!(" {} {}", x, accender - y); // P(b)
                            let dyc = args.pop().unwrap();
                            y += dyc;
                            command += &format!(" {}", dyc);
                            svg += &format!(" {} {}\n", x, accender - y); // P(c)
                            let dyd = args.pop().unwrap();
                            y += dyd;
                            command += &format!(" {}", dyd);
                            svg += &format!("C {} {}", x, accender - y); // P(d)
                            let dxe = args.pop().unwrap();
                            x += dxe;
                            command += &format!(" {}", dxe);
                            let dye = args.pop().unwrap();
                            y += dye;
                            command += &format!(" {}", dye);
                            svg += &format!(" {} {}", x, accender - y); // P(e)

                            let dxf = args.pop().unwrap();
                            x += dxf;
                            command += &format!(" {}", dxf);
                            tmp = format!(" {} {}\n", x, accender - y); // P(f)
                        }
                        if 1 <= args.len() {
                            let dyf = args.pop().unwrap();
                            y += dyf;
                            command += &format!(" dyf {}", dyf);
                            svg += &format!(" {} {}\n", x, accender - y); // P(f)
                        } else {
                            svg += &format!("{}", tmp);
                        }
                    }

                    command += "\n";
                    string += &command;
                }
                24 => {
                    // rcurveline rcurveline |- {dxa dya dxb dyb dxc dyc}+ dxd dyd rcurveline (24) |-
                    let mut args = Vec::new();
                    args.push(stacks.pop().unwrap());
                    args.push(stacks.pop().unwrap());
                    while 6 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        let d5 = stacks.pop().unwrap();
                        let d6 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                        args.push(d5);
                        args.push(d6);
                    }
                    let mut command = "rcurveline".to_string();
                    while 8 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;
                        command += &format!(" {}", dxa);
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!(" {}", dya);
                        svg += &format!("C {} {}", x, accender - y); // P(a)
                        let dxb = args.pop().unwrap();
                        x += dxb;
                        command += &format!(" {}", dxb);
                        let dyb = args.pop().unwrap();
                        y += dyb;
                        command += &format!(" {}", dyb);
                        svg += &format!(" {} {}", x, accender - y); // P(b)
                        let dxc = args.pop().unwrap();
                        x += dxc;
                        command += &format!(" {}", dxc);
                        let dyc = args.pop().unwrap();
                        y += dyc;
                        command += &format!(" {}", dyc);
                        svg += &format!(" {} {}\n", x, accender - y); // P(c)
                    }
                    let dxd = args.pop().unwrap();
                    x += dxd;
                    command += &format!(" dxd {}", dxd);
                    let dyd = args.pop().unwrap();
                    y += dyd;
                    command += &format!(" dyd {}", dyd);
                    svg += &format!("L {} {}\n", x, accender - y); // add line
                    command += "\n";
                    string += &command;
                }
                25 => {
                    // rlinecurve rlinecurve |- {dxa dya}+ dxb dyb dxc dyc dxd dyd rlinecurve (25) |-
                    let mut args = Vec::new();
                    while 8 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                        args.push(stacks.pop().unwrap());
                    }
                    let d1 = stacks.pop().unwrap();
                    let d2 = stacks.pop().unwrap();
                    let d3 = stacks.pop().unwrap();
                    let d4 = stacks.pop().unwrap();
                    let d5 = stacks.pop().unwrap();
                    let d6 = stacks.pop().unwrap();
                    args.push(d1);
                    args.push(d2);
                    args.push(d3);
                    args.push(d4);
                    args.push(d5);
                    args.push(d6);
                    let mut command = "rlinecurve".to_string();
                    while 8 <= args.len() {
                        let dxa = args.pop().unwrap();
                        x += dxa;
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!("dxa {} dya {}", dxa, dya);
                        svg += &format!("L {} {}\n", x, accender - y); // P(a)
                    }
                    let dxb = args.pop().unwrap();
                    x += dxb;
                    command += &format!(" {}", dxb);
                    let dyb = args.pop().unwrap();
                    y += dyb;
                    command += &format!(" {}", dyb);
                    svg += &format!("C {} {}", x, accender - y); // P(b)
                    let dxc = args.pop().unwrap();
                    x += dxc;
                    command += &format!(" {}", dxc);
                    let dyc = args.pop().unwrap();
                    y += dyc;
                    command += &format!(" {}", dyc);
                    svg += &format!(" {} {}", x, accender - y); // P(c)

                    let dxd = args.pop().unwrap();
                    x += dxd;
                    command += &format!(" {}", dxd);
                    let dyd = args.pop().unwrap();
                    y += dyd;
                    command += &format!(" {}", dyd);
                    svg += &format!(" {} {}\n", x, accender - y); // P(d)

                    command += "\n";
                    string += &command;
                }
                30 => {
                    // vhcurveto |- dy1 dx2 dy2 dx3 {dxa dxb dyb dyc dyd dxe dye dxf}* dyf?
                    // vhcurveto (30) |-
                    // |- {dya dxb dyb dxc dxd dxe dye dyf}+ dxf? vhcurveto (30) |-
                    let mut args = Vec::new();
                    let mut tmp = "".to_string();

                    while 8 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        let d5 = stacks.pop().unwrap();
                        let d6 = stacks.pop().unwrap();
                        let d7 = stacks.pop().unwrap();
                        let d8 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                        args.push(d5);
                        args.push(d6);
                        args.push(d7);
                        args.push(d8);
                    }
                    if 4 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                    }
                    if 1 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                    }
                    let mut command = "vhcurveto".to_string();
                    if args.len() % 8 >= 4 {
                        let dy1 = args.pop().unwrap();
                        y += dy1;
                        command += &format!(" dy1 {}", dy1);
                        svg += &format!("C {} {}", x, accender - y); // P(a)

                        let dx2 = args.pop().unwrap();
                        x += dx2;
                        command += &format!(" dx2 {}", dx2);
                        let dy2 = args.pop().unwrap();
                        y += dy2;
                        command += &format!(" dy2 {}", dy2);
                        svg += &format!(" {} {}", x, accender - y); // P(b)

                        let dx3 = args.pop().unwrap();
                        x += dx3;
                        command += &format!(" dx3 {}", dx3);
                        svg += &format!(" {} {}\n", x, accender - y); // P(c)
                        let mut lp = false;
                        while 8 <= args.len() {
                            lp = true;
                            svg += &tmp;
                            let dxa = args.pop().unwrap();
                            x += dxa;
                            command += &format!(" {}", dxa);
                            svg += &format!("C {} {}", x, accender - y); // P(a)

                            let dxb = args.pop().unwrap();
                            x += dxb;
                            command += &format!(" {}", dxb);
                            let dyb = args.pop().unwrap();
                            y += dyb;
                            command += &format!(" {}", dyb);

                            svg += &format!(" {} {}", x, accender - y); // P(b)

                            let dyc = args.pop().unwrap();
                            y += dyc;
                            command += &format!(" {}", dyc);
                            svg += &format!(" {} {}\n", x, accender - y); // P(c)

                            let dyd = args.pop().unwrap();
                            y += dyd;
                            command += &format!(" {}", dyd);
                            svg += &format!("C {} {}", x, accender - y); // P(d)

                            let dxe = args.pop().unwrap();
                            x += dxe;
                            command += &format!(" {}", dxe);
                            let dye = args.pop().unwrap();
                            y += dye;
                            command += &format!(" {}", dye);
                            svg += &format!(" {} {}", x, accender - y); // P(e)

                            let dxf = args.pop().unwrap();
                            x += dxf;
                            command += &format!(" {}", dxf);
                            tmp = format!(" {} {}\n", x, accender - y); // P(f)
                        }
                        if lp {
                            if 1 <= args.len() {
                                let dyf = args.pop().unwrap();
                                y += dyf;
                                command += &format!(" dyf {}", dyf);
                                svg += &format!(" {} {}\n", x, accender - y); // P(f)
                            } else {
                                svg += &format!("{}", tmp);
                            }
                        }
                    } else {
                        while 8 <= args.len() {
                            svg += &tmp;
                            // {dya dxb dyb dxc dxd dxe dye dyf}+ dxf?
                            let dya = args.pop().unwrap();
                            y += dya;
                            command += &format!(" {}", dya);
                            svg += &format!("C {} {}", x, accender - y); // P(a)

                            let dxb = args.pop().unwrap();
                            x += dxb;
                            command += &format!(" {}", dxb);
                            let dyb = args.pop().unwrap();
                            y += dyb;
                            command += &format!(" {}", dyb);

                            svg += &format!(" {} {}", x, accender - y); // P(b)

                            let dxc = args.pop().unwrap();
                            x += dxc;
                            command += &format!(" {}", dxc);
                            svg += &format!(" {} {}\n", x, accender - y); // P(c)

                            let dxd = args.pop().unwrap();
                            x += dxd;
                            command += &format!(" {}", dxd);

                            svg += &format!("C {} {}", x, accender - y); // P(d)
                            let dxe = args.pop().unwrap();
                            x += dxe;

                            command += &format!(" {}", dxe);
                            let dye = args.pop().unwrap();
                            y += dye;
                            command += &format!(" {}", dye);

                            svg += &format!(" {} {}", x, accender - y); // P(e)
                            let dyf = args.pop().unwrap();
                            y += dyf;
                            command += &format!(" {}", dyf);
                            tmp = format!(" {} {}", x, accender - y); // P(f)
                        }
                        if 1 <= args.len() {
                            let dxf = args.pop().unwrap();
                            x += dxf;
                            command += &format!(" dxf {}", dxf);
                            svg += &format!(" {} {}\n", x, accender - y); // P(f)
                        } else {
                            svg += &tmp;
                        }
                    }

                    command += "\n";
                    string += &command;
                }
                26 => {
                    // vvcurveto |- dx1? {dya dxb dyb dyc}+ vvcurveto (26) |-
                    let mut args = Vec::new();
                    while 4 <= stacks.len() {
                        let d1 = stacks.pop().unwrap();
                        let d2 = stacks.pop().unwrap();
                        let d3 = stacks.pop().unwrap();
                        let d4 = stacks.pop().unwrap();
                        args.push(d1);
                        args.push(d2);
                        args.push(d3);
                        args.push(d4);
                    }
                    if 1 <= stacks.len() {
                        args.push(stacks.pop().unwrap());
                    }
                    let mut command = "vvcurveto".to_string();
                    if args.len() % 4 == 1 {
                        let dx1 = args.pop().unwrap();
                        x += dx1;
                        command += &format!(" dx1 {}", dx1);
                        // svg += &format!("L {} {}", x, accender - y); // P1
                    }
                    while 4 <= args.len() {
                        let dya = args.pop().unwrap();
                        y += dya;
                        command += &format!(" {}", dya);
                        svg += &format!("C {} {}", x, accender - y); // P(a)
                        let dxb = args.pop().unwrap();
                        x += dxb;
                        command += &format!(" {}", dxb);
                        let dyb = args.pop().unwrap();
                        y += dyb;
                        command += &format!(" {}", dyb);
                        svg += &format!(" {} {}", x, accender - y); // P(b)

                        let dyc = args.pop().unwrap();
                        y += dyc;
                        command += &format!(" {}", dyc);
                        svg += &format!(" {} {}\n", x, accender - y); // P(c)

                    }
                    command += "\n";
                    string += &command;
                }

                28 => {
                    let b1 = data[i];
                    let value = i16::from_be_bytes([b0, b1]) as i32;
                    stacks.push(value as f64);
                    i += 1;
                }
                14 => {
                    // endchar – endchar (14) |–
                    break;
                }
                12 => {
                    let b1 = data[i];
                    i += 1;
                    match b1 {
                        35 => {
                            // flex |- dy1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 dx6 dy6 fd flex (12 35) |-
                            let mut command = "flex".to_string();
                            let fd = stacks.pop().unwrap();
                            let dy6 = stacks.pop().unwrap();
                            let dx6 = stacks.pop().unwrap();
                            let dy5 = stacks.pop().unwrap();
                            let dx5 = stacks.pop().unwrap();
                            let dy4 = stacks.pop().unwrap();
                            let dx4 = stacks.pop().unwrap();
                            let dy3 = stacks.pop().unwrap();
                            let dx3 = stacks.pop().unwrap();
                            let dy2 = stacks.pop().unwrap();
                            let dx2 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let mut xx = x;
                            let mut yy = y;
                            xx += dy1;
                            yy += dy1;
                            svg += &format!("C {} {}", x, accender - y); // P(a)
                            xx += dx2;
                            yy += dy2;
                            svg += &format!(" {} {}", xx, accender - yy); // P(b)
                            xx += dx3;
                            yy += dy3;
                            svg += &format!(" {} {}", xx, accender - yy); // P(c)
                            xx += dx4;
                            yy += dy4;
                            svg += &format!("C {} {}", xx, accender - yy); // P(d)
                            xx += dx5;
                            yy += dy5;
                            svg += &format!(" {} {}", xx, accender - yy); // P(e)
                            xx += dx6;
                            yy += dy6;
                            svg += &format!(" {} {}", xx, accender - yy); // P(f)

                            x += dy1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            y += dy1 + dy2 + dy3 + dy4 + dy5 + dy6;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {} {} {} {} fd {}\n",
                                dy1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, dx6, dy6, fd
                            );
                            string += &command;
                        }
                        34 => {
                            // hflex |- dy1 dx2 dy2 dx3 dx4 dx5 dx6 hflex (12 34) |
                            let mut command = "hflex".to_string();
                            let dx6 = stacks.pop().unwrap();
                            let dx5 = stacks.pop().unwrap();
                            let dx4 = stacks.pop().unwrap();
                            let dx3 = stacks.pop().unwrap();
                            let dx2 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let mut xx = x;
                            let yy = y;
                            xx += dy1;
                            svg += &format!("C {} {}", xx, accender - yy); // P(a)
                            xx += dx2;
                            svg += &format!(" {} {}", xx, accender - yy); // P(b)
                            xx += dx3;
                            svg += &format!(" {} {}", xx, accender - yy); // P(c)
                            xx += dx4;
                            svg += &format!("C {} {}", xx, accender - yy); // P(d)
                            xx += dx5;
                            svg += &format!(" {} {}", xx, accender - yy); // P(e)
                            xx += dx6;
                            svg += &format!(" {} {}", xx, accender - yy); // P(f)

                            x += dy1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            command +=
                                &format!(" {} {} {} {} {} {}\n", dy1, dx2, dx3, dx4, dx5, dx6);
                            string += &command;
                        }
                        36 => {
                            // hflex1 |- dy1 dy1 dx2 dy2 dx3 dx4 dx5 dy5 dx6 hflex1 (12 36) |
                            let mut command = "hflex1".to_string();
                            let dx6 = stacks.pop().unwrap();
                            let dy5 = stacks.pop().unwrap();
                            let dx5 = stacks.pop().unwrap();
                            let dx4 = stacks.pop().unwrap();
                            let dx3 = stacks.pop().unwrap();
                            let dy2 = stacks.pop().unwrap();
                            let dx2 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let mut xx = x;
                            let mut yy = y;
                            xx += dy1;
                            yy += dy1;
                            svg += &format!("C {} {}", xx, accender - yy); // P(a)
                            xx += dx2;
                            yy += dy2;
                            svg += &format!(" {} {}", xx, accender - yy); // P(b)
                            xx += dx3;
                            svg += &format!(" {} {}", xx, accender - yy); // P(c)
                            xx += dx4;
                            svg += &format!("C {} {}", xx, accender - yy); // P(d)
                            xx += dx5;
                            yy += dy5;
                            svg += &format!(" {} {}", xx, accender - yy); // P(e)
                            xx += dx6;
                            svg += &format!(" {} {}", xx, accender - yy); // P(f)

                            x += dy1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            y += dy1 + dy2 + dy5;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {}\n",
                                dy1, dy1, dx2, dy2, dx3, dx4, dx5, dy5, dx6
                            );
                            string += &command;
                        }
                        37 => {
                            // flex1 |- dy1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 d6 flex1 (12 37) |-
                            let mut command = "flex1".to_string();
                            let dy6 = stacks.pop().unwrap();
                            let dx6 = stacks.pop().unwrap();
                            let dy5 = stacks.pop().unwrap();
                            let dx5 = stacks.pop().unwrap();
                            let dy4 = stacks.pop().unwrap();
                            let dx4 = stacks.pop().unwrap();
                            let dy3 = stacks.pop().unwrap();
                            let dx3 = stacks.pop().unwrap();
                            let dy2 = stacks.pop().unwrap();
                            let dx2 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let mut xx = x;
                            let mut yy = y;
                            xx += dy1;
                            yy += dy1;
                            svg += &format!("C {} {}", xx, accender - yy); // P(a)
                            xx += dx2;
                            yy += dy2;
                            svg += &format!(" {} {}", xx, accender - yy); // P(b)
                            xx += dx3;
                            yy += dy3;
                            svg += &format!(" {} {}", xx, accender - yy); // P(c)
                            xx += dx4;
                            yy += dy4;
                            svg += &format!("C {} {}", xx, accender - yy); // P(d)
                            xx += dx5;
                            yy += dy5;
                            svg += &format!(" {} {}", xx, accender - yy); // P(e)
                            xx += dx6;
                            yy += dy6;
                            svg += &format!(" {} {}", xx, accender - yy); // P(f)

                            x += dy1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            y += dy1 + dy2 + dy3 + dy4 + dy5 + dy6;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {} {} {} {}\n",
                                dy1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, dx6, dy6
                            );
                            string += &command;
                        }

                        19 => {
                            // abs
                            let number = stacks.pop().unwrap();
                            string += &format!("abs {}\n", number);
                            stacks.push(number.abs());
                        }
                        10 => {
                            // add
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("add {} {}\n", num1, num2);
                            stacks.push(num1 + num2);
                        }
                        11 => {
                            // sub
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("sub {} {}\n", num1, num2);
                            stacks.push(num1 - num2);
                        }
                        12 => {
                            // div
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("div {} {}\n", num1, num2);
                            stacks.push(num1 / num2);
                        }
                        14 => {
                            // neg
                            let num = stacks.pop().unwrap();
                            string += &format!("neg {}\n", num);
                            stacks.push(-num);
                        }
                        23 => {
                            // random
                            // random 0.0 - 1.0
                            // need rand crate
                            // let num = rand::random::<f64>();
                            let num = 0.5;
                            string += &format!("random\n");
                            stacks.push(num);
                        }
                        24 => {
                            // mul
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("mul {} {}\n", num1, num2);
                            stacks.push(num1 * num2);
                        }
                        26 => {
                            // sqrt
                            let num = stacks.pop().unwrap();
                            string += &format!("sqrt {}\n", num);
                            stacks.push(num.sqrt());
                        }
                        18 => {
                            // drop
                            stacks.pop();
                            string += &format!("drop\n");
                        }
                        29 => {
                            // index
                            let index = stacks.pop().unwrap();
                            let num = stacks[stacks.len() - index as usize];
                            string += &format!("index {}\n", index);
                            stacks.push(num);
                        }
                        30 => {
                            // roll
                            let index = stacks.pop().unwrap();
                            let count = stacks.pop().unwrap();
                            let mut new_stacks = Vec::new();
                            for _ in 0..count as usize {
                                let num = stacks.pop().unwrap();
                                new_stacks.push(num);
                            }
                            for _ in 0..count as usize {
                                let num = new_stacks.pop().unwrap();
                                stacks.push(num);
                            }
                            string += &format!("roll {} {}\n", index, count);
                        }
                        27 => {
                            // dup
                            let num = stacks.pop().unwrap();
                            string += &format!("dup {}\n", num);
                            stacks.push(num);
                            stacks.push(num);
                        }

                        20 => {
                            // put
                            let index = stacks.pop().unwrap();
                            let num = stacks.pop().unwrap();
                            string += &format!("put {} {}\n", index, num);
                            stacks[index as usize] = num;
                        }
                        21 => {
                            // get
                            let index = stacks.pop().unwrap();
                            let num = stacks[index as usize];
                            string += &format!("get {} {}\n", index, num);
                            stacks.push(num);
                        }
                        3 => {
                            // and
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("and {} {}\n", num1, num2);
                            let num = if num1 == 0.0 || num2 == 0.0 { 0 } else { 1 };
                            stacks.push(num as f64);
                        }
                        4 => {
                            // or
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("or {} {}\n", num1, num2);
                            let num = if num1 == 0.0 && num2 == 0.0 { 0 } else { 1 };
                            stacks.push(num as f64);
                        }
                        5 => {
                            // not
                            let num = stacks.pop().unwrap();
                            string += &format!("not {}\n", num);
                            let num = if num == 0.0 { 1 } else { 0 };
                            stacks.push(num as f64);
                        }
                        15 => {
                            // eq
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            string += &format!("eq {} {}\n", num1, num2);
                            let num = if num1 == num2 { 1 } else { 0 };
                            stacks.push(num as f64);
                        }
                        22 => {
                            // if else
                            let num2 = stacks.pop().unwrap();
                            let num1 = stacks.pop().unwrap();
                            let res2 = stacks.pop().unwrap();
                            let res1 = stacks.pop().unwrap();
                            string += &format!("ifelse {} {} {} {}\n", num1, num2, res1, res2);
                            let num = if num1 > num2 { res1 } else { res2 };
                            stacks.push(num);
                        }

                        _ => { // reserved
                        }
                    }
                }
                10 => {
                    // call callsubr
                    let mut command = "callsubr\n".to_string();
                    let mut num = stacks.pop().unwrap() as isize;
                    if let Some(subr) = self.subr.as_ref() {
                        let len = subr.data.data.len();
                        if len <= 1238 {
                            num += 107
                        } else if len <= 33899 {
                            num += 1131
                        } else {
                            num += 32768
                        }

                        command += &format!("{}\n", num);
                        string += &command;
                        let data = &subr.data.data[num as usize];
                        let subroutine = self.parse_data(&data, width, stacks, is_svg);
                        string += &subroutine;
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
                    let mut num = stacks.pop().unwrap() as isize;
                    if let Some(subr) = self.gsubr.as_ref() {
                        let len = subr.data.data.len();
                        if len <= 1238 {
                            num += 107
                        } else if len <= 33899 {
                            num += 1131
                        } else {
                            num += 32768
                        }
                        command += &format!("{}\n", num);
                        string += &command;
                        let data = &subr.data.data[num as usize];
                        let subroutine = self.parse_data(&data, width,  stacks, is_svg);
                        string += &subroutine;
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
                    string += &command;
                    return string;
                }
                32..=246 => {
                    let value = b0 as i32 - 139;
                    stacks.push(value as f64);
                }
                247..=250 => {
                    let b1 = data[i];
                    let value = (b0 as i32 - 247) * 256 + b1 as i32 + 108;
                    stacks.push(value as f64);
                    i += 1;
                }
                251..=254 => {
                    let b1 = data[i];
                    let value = -(b0 as i32 - 251) * 256 - b1 as i32 - 108;
                    stacks.push(value as f64);
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
                        stacks.push(value);
                        i += 4;
                    }
                }
                _ => {
                    // 0,2,9,13,15,16,17 reserved
                }
            }
        }
        svg += "Z";
        let x_min = self.bbox[0];
        let height = self.bbox[3] - self.bbox[1];

        let viewbox = format!("viewBox=\"{} {} {} {}\"", x_min, 0, width - x_min, height);

        let mut svg2 = format!("<svg width=\"270.9375pt\" height=\"240pt\" {} xmlns=\"http://www.w3.org/2000/svg\" fill=\"none\" stroke=\"black\" stroke-width=\"10pt\">\n", viewbox);
        svg2 += "<path d=\"";
        svg2 += &svg;
        svg2 += "\" />\n</svg>";
        svg = svg2;

        let string = format!("\nx {} y {} width {}\n {}\n{}\n", x, y, width, string, svg);
        string
    }
}


#[derive(Debug, Clone)]
pub(crate) struct FDSelect {
    fsds: Vec<u8>
}

impl FDSelect {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R,offset: u32, n_glyphs: usize) -> Result<Self,Box<dyn Error>>{      reader.seek(SeekFrom::Start(offset as u64))?;
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
                let first_gid = reader.read_u16_be()?;
                for _ in 0..n_ranges {
                    let fd = reader.read_u8()?;
                    let sentinel_gid = reader.read_u16_be()?;
                    for _ in last_gid..sentinel_gid {
                        fsds.push(fd);
                    }
                    last_gid = first_gid;
                }
            }
            _ => {return Err("Illegal format".into()) }
        }
        Ok(Self {
            fsds
        })
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
        offset: u32,
        n_glyphs: u32,
    ) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset as u64)).unwrap();
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
        offset: u32,
    ) -> Result<Self, Box<dyn Error>> {
        if offset > 0 {
            reader.seek(SeekFrom::Start(offset as u64))?;
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
