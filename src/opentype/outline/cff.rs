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
    pub(crate) name: String,
    pub(crate) top_dict: Dict, // TopDict
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
}

impl CFF {
    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        _: u32,
    ) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset as u64))?;

        let header = Header::parse(reader)?;
        let name_index = Index::parse(reader)?;
        let name = String::from_utf8(name_index.data[0].clone())?;
        let top_dict_index = Index::parse(reader)?;
        let top_dict = Dict::parse(&top_dict_index.data[0])?;
        let n_glyphs = top_dict.get_i32(0, 15).unwrap() as usize;
        let encording_offset = top_dict.get_i32(0, 16); // none
        let global_subr_index_offset = top_dict.get_i32(12, 29); // none
        let fd_array_offset = top_dict.get_i32(12, 36).unwrap();
        let fd_select_offset = top_dict.get_i32(12, 37).unwrap();
        let charsets_offset = top_dict.get_i32(0, 15).unwrap();
        let char_strings_offset = top_dict.get_i32(0, 17).unwrap();
        #[cfg(debug_assertions)]
        {
            println!("n_glyphs: {}", n_glyphs);
            println!("encording: {:?}", encording_offset);
            println!("global_subr_index: {:?}", global_subr_index_offset);
            println!("fd_array: {:?}", fd_array_offset);
            println!("fd_select: {:?}", fd_select_offset);
            println!("charsets: {:?}", charsets_offset);
            println!("char_strings: {:?}", char_strings_offset);
        }

        let charsets_offset = charsets_offset as u32 + offset;

        let charsets = Charsets::new(reader, charsets_offset, n_glyphs as u32)?;
        let char_strings_offset = char_strings_offset as u32 + offset;
        let char_string = CharString::new(reader, char_strings_offset as u32)?;
        #[cfg(debug_assertions)]
        {
            println!("char_string: {:?}", char_string.data.data[0]);
        }
        // let fd_select = FDSelect::new(reader, fd_select_offset as u32 + offset, n_glyphs as u32)?;
        // println!("fd_select: {:?}", fd_select.fsds[0..10].to_vec());
        let private = top_dict.get_i32_array(0, 18);
        let private_dict = if let Some(private) = private {
            let private_dict_offset = private[1] as u32;
            let private_dict_offset = private_dict_offset as u32 + offset;
            reader.seek(SeekFrom::Start(private_dict_offset as u64))?;
            let private_dict_index = Index::parse(reader)?;
            let private_dict = Dict::parse(&private_dict_index.data[0])?;
            Some(private_dict)
        } else {
            None
        };

        Ok(Self {
            header,
            name,
            top_dict,
            strings: Vec::new(),
            charsets,
            char_string,
            // fd_Select: FDSelect,
            // fd_dict_index: FDDistIndex,
            private_dict,
        })
    }

    pub(crate) fn to_code(&self, gid: u32) -> String {
        //        let cid = self.charsets.sid[gid as usize];
        let data = &self.char_string.data.data[gid as usize];
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
        let mut i = 0;
        let mut string = String::new();
        let mut stacks: Vec<f64> = Vec::new();
        let mut width = self.top_dict.get_f64(0, 15).unwrap(); // nomarl width
        let mut first = true;
        while i < data.len() {
            let b0 = data[i];
            i += 1;

            match b0 {
                1 => {
                    // hstem |- y dy {dya dyb}* hstem (1) |
                    let mut command = "hstem".to_string();
                    y = stacks[0];
                    command += &format!(" {}", y);
                    let dy = stacks[1];
                    y += dy;
                    command += &format!(" {}", y);
                    let mut i = stacks.len() % 2 + 2;
                    while i + 1 < stacks.len() {
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", y);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", y);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;

                    stacks.truncate(stacks.len() - i + 1);
                }
                3 => {
                    // vstem |- v dx {dxa dxb}* vstem (3) |
                    let mut command = "vstem".to_string();
                    x = stacks[0];
                    command += &format!(" {}", x);
                    let dx = stacks[1];
                    x += dx;
                    command += &format!(" {}", x);
                    let mut i = stacks.len() % 2 + 2;
                    while i + 2 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", x);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", x);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    // stacks.len() - i..stacks(i) までの要素を削除
                    stacks.truncate(stacks.len() - i + 1);
                }
                18 => {
                    // hstemhm |- y dy {dya dyb}* hstemhm (18) |-
                    let mut command = "hstemhm".to_string();
                    y = stacks[0];
                    command += &format!(" {}", y);
                    let dy = stacks[1];
                    y += dy;
                    command += &format!(" {}", y);
                    let mut i = stacks.len() % 2 + 2;
                    while i < stacks.len() {
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", y);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", y);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1)
                }
                23 => {
                    // vstemhm |- x dx {dxa dxb}* vstemhm (23) |-
                    let mut command = "vstemhm".to_string();
                    x = stacks[0];
                    command += &format!(" {}", x);
                    let dx = stacks[1];
                    x += dx;
                    command += &format!(" {}", x);
                    let mut i = stacks.len() % 2 + 2;
                    while i < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", x);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", x);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                19 => {
                    // hintmask |- hintmask (19 + mask) |
                    let mask = data[i + 1];
                    i += 1;
                    let mut command = "hintmask".to_string();
                    command += &format!(" {:08b}", mask);
                    command += "\n";
                    string += &command;
                }
                20 => {
                    // cntrmask |- cntrmask (20 + mask) |-
                    let mask = data[i + 1];
                    i += 1;
                    let mut command = "cntrmask".to_string();
                    command += &format!(" {:08b}", mask);
                    command += "\n";
                    string += &command;
                }

                21 => {
                    // rmoveto |- dx1 dy1 rmoveto (21) |-
                    let dy = stacks.pop().unwrap();
                    let dx = stacks.pop().unwrap();
                    x += dx;
                    y += dy;
                    string += &format!("rmoveto {} {}\n", dx, dy);

                    if stacks.len() > 0 && first == true {
                        width += stacks.pop().unwrap();
                        first = false;
                        string += &format!("width {}\n", width);
                    }
                }
                22 => {
                    // hmoveto |- dx1 hmoveto (22) |-
                    let dy = stacks.pop().unwrap();
                    y += dy;
                    string += &format!("hmoveto {}\n", dy);
                    if stacks.len() > 0 && first == true {
                        width += stacks.pop().unwrap();
                        first = false;
                        string += &format!("width {}\n", width);
                    }
                    string += &format!("hmoveto {}\n", dy);
                }
                4 => {
                    // vmoveto |- dy1 vmoveto (4) |-
                    let dx = stacks.pop().unwrap();
                    x += dx;
                    string += &format!("vmoveto {}\n", dx);

                    if stacks.len() > 0 && first == true {
                        width += stacks.pop().unwrap();
                        first = false;
                        string += &format!("width {}\n", width);
                    }
                }
                5 => {
                    // rlineto |- {dxa dya}+ rlineto (5) |-
                    let mut command = "rlineto".to_string();
                    let mut i = 0;
                    while i + 1 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                6 => {
                    //  |- dx1 {dya dxb}* hlineto (6) |- odd
                    // |- {dxa dyb}+ hlineto (6) |-      even
                    let mut command = "hlineto".to_string();
                    let mut i = 0;
                    if stacks.len() % 2 == 1 {
                        let dx = stacks[i];
                        x += dx;
                        command += &format!(" dx1 {}", dx);
                        i += 1;
                    }
                    while i + 2 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                7 => {
                    // vlineto - dy1 {dxa dyb}* vlineto (7) |- odd
                    // |- {dya dxb}+ vlineto (7) |-  even
                    let mut command = "vlineto".to_string();
                    let mut i = 0;
                    if stacks.len() % 2 == 1 {
                        let dy = stacks[i];
                        y += dy;
                        command += &format!(" dy1 {}", dy);
                        i += 1;
                    }
                    while i + 1 < stacks.len() {
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                8 => {
                    // rrcurveto |- {dxa dya dxb dyb dxc dyc}+ rrcurveto (8) |-
                    let mut command = "rrcurveto".to_string();
                    let mut i = 0;
                    while i + 5 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                        let dxc = stacks[i];
                        x += dxc;
                        command += &format!(" {}", dxc);
                        i += 1;
                        let dyc = stacks[i];
                        y += dyc;
                        command += &format!(" {}", dyc);
                        i += 1;
                    }
                    stacks.truncate(stacks.len() - i + 1);
                }
                27 => {
                    //hhcurveto|- dy1? {dxa dxb dyb dxc}+ hhcurveto (27) |-
                    let mut command = "hhcurveto".to_string();
                    let mut i = 0;
                    if stacks.len() % 4 == 1 {
                        let dy = stacks[i];
                        y += dy;
                        command += &format!(" dy1 {}", dy);
                        i += 1;
                    }
                    while i + 3 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                        let dxc = stacks[i];
                        x += dxc;
                        command += &format!(" {}", dxc);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                31 => {
                    // hvcurveto |- dx1 dx2 dy2 dy3 {dya dxb dyb dxc dxd dxe dye dyf}* dxf?
                    //                hvcurveto (31) |-
                    // |- {dxa dxb dyb dyc dyd dxe dye dxf}+ dyf? hvcurveto (31) |-
                    let mut command = "hvcurveto".to_string();
                    let mut i = 0;
                    if stacks.len() % 8 == 4 || stacks.len() % 8 == 5 {
                        let dx = stacks[i];
                        x += dx;
                        command += &format!(" dx1 {}", dx);
                        i += 1;
                        let dx = stacks[i];
                        x += dx;
                        command += &format!(" dx2 {}", dx);
                        i += 1;
                        let dy = stacks[i];
                        y += dy;
                        command += &format!(" dy2 {}", dy);
                        i += 1;
                        let dy = stacks[i];
                        y += dy;
                        command += &format!(" dy3 {}", dy);
                        i += 1;
                    }
                    while i + 7 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                        let dyc = stacks[i];
                        y += dyc;
                        command += &format!(" {}", dyc);
                        i += 1;
                        let dyd = stacks[i];
                        y += dyd;
                        command += &format!(" {}", dyd);
                        i += 1;
                        let dxe = stacks[i];
                        x += dxe;
                        command += &format!(" {}", dxe);
                        i += 1;
                        let dye = stacks[i];
                        y += dye;
                        command += &format!(" {}", dye);
                        i += 1;
                        let dxf = stacks[i];
                        x += dxf;
                        command += &format!(" {}", dxf);
                        i += 1;
                    }
                    if i < stacks.len() {
                        let dyf = stacks[i];
                        y += dyf;
                        command += &format!(" dxf {}", dyf);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                24 => {
                    // rcurveline rcurveline |- {dxa dya dxb dyb dxc dyc}+ dxd dyd rcurveline (24) |-
                    let mut command = "rcurveline".to_string();
                    let mut i = 0;
                    while i + 6 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                        let dxc = stacks[i];
                        x += dxc;
                        command += &format!(" {}", dxc);
                        i += 1;
                        let dyc = stacks[i];
                        y += dyc;
                        command += &format!(" {}", dyc);
                        i += 1;
                    }
                    if i + 2 == stacks.len() {
                        let dxd = stacks[i];
                        x += dxd;
                        command += &format!(" dxd {}", dxd);
                        i += 1;
                        let dyd = stacks[i];
                        y += dyd;
                        command += &format!(" dyd {}", dyd);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                25 => {
                    // rlinecurve rlinecurve |- {dxa dya}+ dxb dyb dxc dyc dxd dyd rlinecurve (25) |-
                    let mut command = "rlinecurve".to_string();
                    let mut i = 0;
                    while i + 6 < stacks.len() {
                        let dxa = stacks[i];
                        x += dxa;
                        command += &format!(" {}", dxa);
                        i += 1;
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                    }
                    let dxb = stacks[i];
                    x += dxb;
                    command += &format!(" {}", dxb);
                    i += 1;
                    let dyb = stacks[i];
                    y += dyb;
                    command += &format!(" {}", dyb);
                    i += 1;
                    let dxc = stacks[i];
                    x += dxc;
                    command += &format!(" {}", dxc);
                    i += 1;
                    let dyc = stacks[i];
                    y += dyc;
                    command += &format!(" {}", dyc);
                    i += 1;
                    let dxd = stacks[i];
                    x += dxd;
                    command += &format!(" {}", dxd);
                    i += 1;
                    let dyd = stacks[i];
                    i += 1;
                    y += dyd;

                    command += &format!(" {}", dyd);
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                30 => {
                    // vhcurveto |- dy1 dx2 dy2 dx3 {dxa dxb dyb dyc dyd dxe dye dxf}* dyf?
                    // vhcurveto (30) |-
                    // |- {dya dxb dyb dxc dxd dxe dye dyf}+ dxf? vhcurveto (30) |-
                    let mut command = "vhcurveto".to_string();
                    let mut i = 0;
                    if stacks.len() % 8 == 4 || stacks.len() % 8 == 5 {
                        let dy = stacks[i];
                        y += dy;
                        command += &format!(" dy1 {}", dy);
                        i += 1;
                        let dx = stacks[i];
                        x += dx;
                        command += &format!(" dx2 {}", dx);
                        i += 1;
                        let dy = stacks[i];
                        y += dy;
                        command += &format!(" dy2 {}", dy);
                        i += 1;
                        let dx = stacks[i];
                        x += dx;
                        command += &format!(" dx3 {}", dx);
                        i += 1;
                    }
                    while i + 7 < stacks.len() {
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                        let dxc = stacks[i];
                        x += dxc;
                        command += &format!(" {}", dxc);
                        i += 1;
                        let dxd = stacks[i];
                        x += dxd;
                        command += &format!(" {}", dxd);
                        i += 1;
                        let dxe = stacks[i];
                        x += dxe;
                        command += &format!(" {}", dxe);
                        i += 1;
                        let dye = stacks[i];
                        y += dye;
                        command += &format!(" {}", dye);
                        i += 1;
                        let dxf = stacks[i];
                        x += dxf;
                        command += &format!(" {}", dxf);
                        i += 1;
                    }
                    if i < stacks.len() {
                        let dyf = stacks[i];
                        y += dyf;
                        command += &format!(" dxf {}", dyf);
                        i += 1;
                    }
                    command += "\n";
                    string += &command;
                    stacks.truncate(stacks.len() - i + 1);
                }
                26 => {
                    // vvcurveto |- dx1? {dya dxb dyb dyc}+ vvcurveto (26) |-
                    let mut command = "vvcurveto".to_string();
                    let mut i = 0;
                    if stacks.len() % 4 == 1 {
                        let dx = stacks[i];
                        x += dx;
                        command += &format!(" dx1 {}", dx);
                        i += 1;
                    }
                    while i + 3 < stacks.len() {
                        let dya = stacks[i];
                        y += dya;
                        command += &format!(" {}", dya);
                        i += 1;
                        let dxb = stacks[i];
                        x += dxb;
                        command += &format!(" {}", dxb);
                        i += 1;
                        let dyb = stacks[i];
                        y += dyb;
                        command += &format!(" {}", dyb);
                        i += 1;
                        let dyc = stacks[i];
                        y += dyc;
                        command += &format!(" {}", dyc);
                        i += 1;
                    }
                    command += "\n";
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
                            // flex |- dx1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 dx6 dy6 fd flex (12 35) |-
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
                            let dx1 = stacks.pop().unwrap();
                            x += dx1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            y += dy1 + dy2 + dy3 + dy4 + dy5 + dy6;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {} {} {} {} fd {}\n",
                                dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, dx6, dy6, fd
                            );
                            string += &command;
                        }
                        34 => {
                            // hflex |- dx1 dx2 dy2 dx3 dx4 dx5 dx6 hflex (12 34) |
                            let mut command = "hflex".to_string();
                            let dx6 = stacks.pop().unwrap();
                            let dx5 = stacks.pop().unwrap();
                            let dx4 = stacks.pop().unwrap();
                            let dx3 = stacks.pop().unwrap();
                            let dx2 = stacks.pop().unwrap();
                            let dx1 = stacks.pop().unwrap();
                            x += dx1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            command +=
                                &format!(" {} {} {} {} {} {}\n", dx1, dx2, dx3, dx4, dx5, dx6);
                            string += &command;
                        }
                        36 => {
                            // hflex1 |- dx1 dy1 dx2 dy2 dx3 dx4 dx5 dy5 dx6 hflex1 (12 36) |
                            let mut command = "hflex1".to_string();
                            let dx6 = stacks.pop().unwrap();
                            let dy5 = stacks.pop().unwrap();
                            let dx5 = stacks.pop().unwrap();
                            let dx4 = stacks.pop().unwrap();
                            let dx3 = stacks.pop().unwrap();
                            let dy2 = stacks.pop().unwrap();
                            let dx2 = stacks.pop().unwrap();
                            let dy1 = stacks.pop().unwrap();
                            let dx1 = stacks.pop().unwrap();
                            x += dx1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            y += dy1 + dy2 + dy5;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {}\n",
                                dx1, dy1, dx2, dy2, dx3, dx4, dx5, dy5, dx6
                            );
                            string += &command;
                        }
                        37 => {
                            // flex1 |- dx1 dy1 dx2 dy2 dx3 dy3 dx4 dy4 dx5 dy5 d6 flex1 (12 37) |-
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
                            let dx1 = stacks.pop().unwrap();
                            x += dx1 + dx2 + dx3 + dx4 + dx5 + dx6;
                            y += dy1 + dy2 + dy3 + dy4 + dy5 + dy6;
                            command += &format!(
                                " {} {} {} {} {} {} {} {} {} {} {} {}\n",
                                dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, dx6, dy6
                            );
                            string += &command;
                            stacks.clear();
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
                    let command = "callsubr\n".to_string();
                    string += &command;
                }
                29 => {
                    // callgsubr
                    let command = "callgsubr\n".to_string();
                    string += &command;
                }
                11 => {
                    // return
                    let command = "return\n".to_string();
                    string += &command;
                }

                32..=246 => {
                    let value = b0 as i32 - 139;
                    stacks.push(value as f64);
                }
                247..=250 => {
                    let b1 = data[i + 1];
                    let value = (b0 as i32 - 247) * 256 + b1 as i32 + 108;
                    stacks.push(value as f64);
                    i += 1;
                }
                251..=254 => {
                    let b1 = data[i + 1];
                    let value = -(b0 as i32 - 251) * 256 - b1 as i32 - 108;
                    stacks.push(value as f64);
                    i += 1;
                }
                255 => {
                    let b1 = data[i + 1];
                    let b2 = data[i + 2];
                    let b3 = data[i + 3];
                    let b4 = data[i + 4];
                    let value = i16::from_be_bytes([b1, b2]) as f64;
                    let frac = u16::from_be_bytes([b3, b4]) as f64;
                    let value = value + frac / 65536.0;
                    stacks.push(value);
                    i += 4;
                }
                _ => {
                    // 0,2,9,13,15,16,17 reserved
                }
            }
        }
        let string = format!("\nx {} y {} width {}\n {}", x, y, width, string);
        string
    }
}

/*
#[derive(Debug, Clone)]
pub(crate) struct FDSelect {
    fsds: Vec<u8>
}

impl FDSelect {
    pub(crate) fn new<R: BinaryReader>(reader: &mut R,offset: u32, n_glyphs: u32) -> Result<Self,Box<dyn Error>>{      reader.seek(SeekFrom::Start(offset as u64))?;
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
*/

#[derive(Debug, Clone)]
pub(crate) struct Charsets {
    n_glyphs: usize,
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
            n_glyphs: n_glyphs as usize,
            format,
            sid: Vec::new(),
        };

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

    fn parse_format2(&mut self) {
        todo!()
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
    pub(crate) count: u16,
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
        let mut operator = 0;
        let mut operands = Vec::new();
        while i < buffer.len() {
            if buffer.len() <= i {
                break;
            }
            let b = buffer[i];
            if b == 12 {
                operator = (b as u16) << 8 | buffer[i + 1] as u16;
                entries.insert(operator, operands);
                operands = Vec::new();
                i += 2;
            } else if b <= 21 {
                operator = b as u16;
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
        reader.seek(SeekFrom::Start(offset as u64))?;
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
            return Ok(Self {
                count,
                data: Vec::new(),
            });
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
                _ => {}
            }
        }

        let mut data = Vec::new();
        for i in 0..count {
            let start = offsets[i as usize] as usize;
            let end = offsets[i as usize + 1] as usize;
            let buf = r.read_bytes_as_vec(end - start)?;
            data.push(buf);
        }
        Ok(Self { count, data })
    }
}
