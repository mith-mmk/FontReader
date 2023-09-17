// Glyph Data
use std::{fmt, io::SeekFrom};

use bin_rs::reader::BinaryReader;

use super::loca;
/*
int16	numberOfContours	If the number of contours is greater than or equal to zero, this is a simple glyph. If negative, this is a composite glyph — the value -1 should be used for composite glyphs.
int16	xMin	Minimum x for coordinate data.
int16	yMin	Minimum y for coordinate data.
int16	xMax	Maximum x for coordinate data.
int16	yMax	Maximum y for coordinate data.
*/

#[derive(Debug, Clone)]

pub(crate) struct GLYF {
    pub(crate) griphs: Box<Vec<Glyph>>,
}

#[derive(Debug, Clone)]
pub struct Glyph {
    pub glyphs: Box<Vec<u8>>,
    pub offset: u32,
    pub length: u32,
}

#[derive(Debug, Clone)]
pub struct ParsedGlyph {
    pub number_of_contours: i16,
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
    pub offset: u32,
    pub length: u32,
    pub end_pts_of_contours: Vec<usize>,
    pub instructions: Vec<u8>,
    // bit 0 on curve -> true if on curve else off curve
    // bit 1 x short -> X data is 1byte if flag else 2byte
    // bit 2 y short -> Y data is 1byte if flag else 2byte
    // bit 3 repeat -> repeat flag
    // bit 4 x same -> X data is 0byte, same as previous
    // bit 5 y same -> Y data is 0byte, same as previous
    // bit 6 overlap simple -> overlap simple flag, also this flag is false using evenodd, but true using nonzero
    //                         However, there is no problem if you use nonzero.
    // bit 7 reserved
    // bit 8 reserved
    pub flags: Vec<u8>,
    pub xs: Vec<i16>,
    pub ys: Vec<i16>,
    pub on_curves: Vec<bool>,
}

impl fmt::Display for Glyph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Glyph {
    pub fn parse(&self) -> ParsedGlyph {
        if self.length < 10 {
            return ParsedGlyph {
                number_of_contours: 0,
                x_min: 0,
                y_min: 0,
                x_max: 0,
                y_max: 0,
                offset: self.offset,
                length: self.length,
                end_pts_of_contours: Vec::new(),
                instructions: Vec::new(),
                flags: Vec::new(),
                xs: Vec::new(),
                ys: Vec::new(),
                on_curves: Vec::new(),
            };
        }
        let buf = self.glyphs.clone();
        let mut offset = 0;
        let high_byte = buf[offset] as u16;
        let low_byte = buf[offset + 1] as u16;
        let number_of_contours = ((high_byte << 8) + low_byte) as i16;
        offset += 2;
        let high_byte = buf[offset] as u16;
        let low_byte = buf[offset + 1] as u16;
        let x_min = ((high_byte << 8) + low_byte) as i16;
        offset += 2;
        let high_byte = buf[offset] as u16;
        let low_byte = buf[offset + 1] as u16;
        let y_min = ((high_byte << 8) + low_byte) as i16;
        offset += 2;
        let high_byte = buf[offset] as u16;
        let low_byte = buf[offset + 1] as u16;
        let x_max = ((high_byte << 8) + low_byte) as i16;
        offset += 2;
        let high_byte = buf[offset] as u16;
        let low_byte = buf[offset + 1] as u16;
        let y_max = ((high_byte << 8) + low_byte) as i16;
        offset += 2;

        let mut instructions = Vec::new();
        let mut flags = Vec::new();
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let mut on_curves = Vec::new();
        let mut coutour = Vec::new();

        if number_of_contours >= 0 {
            for _ in 0..number_of_contours as usize {
                let high_byte = self.glyphs[offset] as u16;
                let low_byte = self.glyphs[offset + 1] as u16;
                offset += 2;
                let number = (high_byte << 8) + low_byte;
                coutour.push(number as usize);
            }
            let last_end_pts_of_contours = coutour[number_of_contours as usize - 1] + 1;
            let high_byte = self.glyphs[offset] as u16;
            let low_byte = self.glyphs[offset + 1] as u16;
            let instruction_length = (high_byte << 8) + low_byte;
            offset += 2;

            for _ in 0..instruction_length {
                let instruction = self.glyphs[offset];
                instructions.push(instruction);
                offset += 1;
            }
            let mut i = 0;
            while i < last_end_pts_of_contours {
                // flags
                let flag = self.glyphs[offset];
                offset += 1;
                flags.push(flag);
                let mut repeat = 0;
                if flag & 0x08 != 0 {
                    repeat = self.glyphs[offset];
                    offset += 1;
                }
                for _ in 0..repeat {
                    flags.push(flag);
                    i += 1;
                }
                i += 1;
            }
            for flag in flags.iter() {
                let on_curve = flag & 0x01 != 0;
                on_curves.push(on_curve);
            }

            i = 0;
            for flag in flags.iter() {
                let mut x = 0;
                if flag & 0x2 != 0 {
                    let byte = self.glyphs[offset];
                    offset += 1;
                    if flag & 0x10 != 0 {
                        x += byte as i16;
                    } else {
                        x -= byte as i16;
                    }
                } else if flag & 0x10 == 0 {
                    let hi_byte = self.glyphs[offset] as u16;
                    let lo_byte = self.glyphs[offset + 1] as u16;
                    offset += 2;
                    let byte = (hi_byte << 8) + lo_byte;
                    x = byte as i16;
                }
                xs.push(x);
            }
            for flag in flags.iter() {
                let mut y = 0;
                if flag & 0x4 != 0 {
                    let byte = self.glyphs[offset];
                    offset += 1;
                    if flag & 0x20 != 0 {
                        y += byte as i16;
                    } else {
                        y -= byte as i16;
                    }
                } else if flag & 0x20 == 0 {
                    let hi_byte = self.glyphs[offset] as u16;
                    let lo_byte = self.glyphs[offset + 1] as u16;
                    offset += 2;
                    let byte = (hi_byte << 8) + lo_byte;
                    y = byte as i16;
                }
                ys.push(y);
            }
        } else {
            // TODO: composite glyph
        }

        ParsedGlyph {
            number_of_contours,
            x_min,
            y_min,
            x_max,
            y_max,
            offset: self.offset,
            length: self.length,
            end_pts_of_contours: coutour,
            instructions,
            flags,
            xs,
            ys,
            on_curves,
        }
    }

    pub(crate) fn get_svg_heder(
        &self,
        fonsize: f32,
        fontunit: &str,
        layout: &crate::fontreader::HorizontalLayout,
    ) -> String {
        let parsed = self.parse();
        Self::get_svg_header_from_parsed(&parsed, fonsize, fontunit, layout)
    }

    pub(crate) fn get_svg_header_from_parsed(
        parsed: &ParsedGlyph,
        fontsize: f32,
        fontunit: &str,
        layout: &crate::fontreader::HorizontalLayout,
    ) -> String {
        let rsb = (layout.advance_width - parsed.x_max as isize) as i16;
        let x_min = parsed.x_min - layout.lsb as i16;
        let x_max = parsed.x_max + rsb;
        let y_max = layout.accender - layout.descender + layout.line_gap;
        let y_min = if y_max > (parsed.y_max - parsed.y_min) as isize {
            0
        } else {
            y_max - (parsed.y_max - parsed.y_min) as isize
        };
        let height = fontsize;
        let width = x_min as f32 + x_max as f32;
        let width = width * height / y_max as f32;

        let height_str = format!("{}{}", height, fontunit);
        let width_str = format!("{}{}", width, fontunit);
        let mut svg = format!("<svg width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" xmlns=\"http://www.w3.org/2000/svg\">", width_str, height_str, x_min, y_min, x_max, y_max);
        #[cfg(debug_assertions)]
        {
            let rsb = (layout.advance_width - parsed.x_max as isize) as i16;
            svg += &format!(
                "<!-- lsb {} accender {} descender {} line_gap {} avance width {}-->",
                layout.lsb,
                layout.accender,
                layout.descender,
                layout.line_gap,
                layout.advance_width
            );
            svg += &format!(
                "<!-- x min {} y min {} x max {} y max {} -->",
                parsed.x_min, parsed.y_min, parsed.x_max, parsed.y_max
            );
            svg += &format!(
                "<!-- offset {} length {} lsb {} advanced width {} rsb {} -->",
                parsed.offset, parsed.length, layout.lsb, layout.advance_width, rsb
            );
        }
        svg
    }

    pub(crate) fn get_svg_path(&self, layout: &crate::fontreader::HorizontalLayout) -> String {
        let parsed = self.parse();
        Self::get_svg_path_parsed(&parsed, layout)
    }

    pub(crate) fn get_svg_path_parsed(
        parsed: &ParsedGlyph,
        layout: &crate::fontreader::HorizontalLayout,
    ) -> String {
        let y_max = layout.accender as i16 + layout.line_gap as i16;
        let mut svg = String::new();
        #[cfg(debug_assertions)]
        {
            svg += "<!-- ";
            let mut i = 0;
            for byte in &parsed.instructions {
                if i % 16 == 0 {
                    svg += "\n";
                }
                i += 1;
                svg += &format!("{:02x} ", byte);
            }
            svg += " -->\n";
        }
        svg += "<path d=\"";
        let mut pos = 0;
        let mut befor_on_curve = false;
        let mut path_start = true;
        let mut x = 0;
        let mut y = 0;
        let mut start_x = 0;
        let mut start_y = 0;

        for i in 0..parsed.flags.len() {
            x += parsed.xs[i];
            y += parsed.ys[i];
            let on_curve = parsed.on_curves[i];
            let next_x;
            let next_y;
            if parsed.end_pts_of_contours[pos] == i || i == parsed.flags.len() - 1 {
                next_x = start_x;
                next_y = start_y;
            } else {
                next_x = x + parsed.xs[i + 1];
                next_y = y + parsed.ys[i + 1];
            }
            let next_on_curve = if i + 1 < parsed.on_curves.len() {
                parsed.on_curves[i + 1]
            } else {
                true
            };
            if path_start {
                if on_curve {
                    start_x = x;
                    start_y = y;
                } else {
                    start_x = (x + next_x) / 2;
                    start_y = (y + next_y) / 2;
                }
                if i != 0 {
                    svg += "Z ";
                }

                svg += &format!("M{} {}", start_x, y_max - start_y);
                path_start = false;
            } else if on_curve {
                if befor_on_curve {
                    svg += &format!("L{} {}", x, y_max - y);
                } else {
                    // Q px py x y or T x y was writed
                }
            } else if befor_on_curve {
                if next_on_curve {
                    svg += &format!("Q{} {} {} {}", x, y_max - y, next_x, y_max - next_y);
                } else {
                    // next off curve
                    svg += &format!(
                        "Q{} {} {} {}",
                        x,
                        y_max - y,
                        (x + next_x) / 2,
                        y_max - (y + next_y) / 2
                    );
                }
            } else {
                // befor off curve
                if next_on_curve {
                    svg += &format!("T{} {}", next_x, y_max - next_y);
                } else {
                    // next off curve
                    svg += &format!(
                        "Q{} {} {} {}",
                        x,
                        y_max - y,
                        (x + next_x) / 2,
                        y_max - (y + next_y) / 2
                    );
                }
            }
            if i >= parsed.end_pts_of_contours[pos] {
                pos += 1;
                path_start = true;
            }
            befor_on_curve = on_curve;
        }
        svg += "Z\"/>";
        svg
    }

    pub fn to_svg(
        &self,
        fonsize: f32,
        fontunit: &str,
        layout: &crate::fontreader::HorizontalLayout,
    ) -> String {
        let parsed = self.parse();
        let mut svg = Self::get_svg_header_from_parsed(&parsed, fonsize, fontunit, layout);

        // heightを後ろから読み出す
        svg += &Self::get_svg_path_parsed(&parsed, layout);
        svg += "\n</svg>";
        svg
    }

    pub fn to_string(&self) -> String {
        let parsed = self.parse();
        let mut string = "glyph\n".to_string();

        string += &format!("Number of Contours {}\n", parsed.number_of_contours);
        string += &format!("x_min {}\n", parsed.x_min);
        string += &format!("x_max {}\n", parsed.x_max);
        string += &format!("y_max {}\n", parsed.y_max);
        string += &format!("y_min {}\n", parsed.y_min);
        string += &format!("offset {}\n", parsed.offset);
        string += &format!("length {}\n", parsed.length);

        let length = self.glyphs.len();
        string += &format!("buffer length {}\n", length);

        if length == 0 {
            string += "empty glyph\n";
            return string;
        }

        // instructions
        for (i, byte) in parsed.instructions.iter().enumerate() {
            if i % 16 == 0 {
                string += "\n";
            }
            string += &format!("{:02x}", byte);
        }
        string += "\n";

        let mut pos = 0;
        for i in 0..parsed.end_pts_of_contours.len() {
            string += &format!(
                "{} end_pts_of_contours {}\n",
                i, parsed.end_pts_of_contours[i]
            );
        }
        let mut x = 0;
        let mut y = 0;

        for i in 0..parsed.flags.len() {
            let dx = parsed.xs[i];
            let dy = parsed.ys[i];
            x += dx;
            y += dy;
            string += &format!(
                "{:2} flag {} {:6} {} {} {} {}\n",
                i, pos, parsed.on_curves[i], x, y, dx, dy
            );
            if i >= parsed.end_pts_of_contours[pos] {
                pos += 1;
            }
        }

        string
    }
}

impl fmt::Display for GLYF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl GLYF {
    pub(crate) fn new<R: BinaryReader>(
        file: &mut R,
        offset: u32,
        length: u32,
        loca: &loca::LOCA,
    ) -> Self {
        get_glyf(file, offset, length, loca)
    }

    pub fn get_glyph(&self, index: usize) -> Option<&Glyph> {
        self.griphs.get(index)
    }

    pub(crate) fn to_string(&self) -> String {
        let mut string = "glyf\n".to_string();
        string += &format!("number of glyphs {}\n", self.griphs.len());
        let max_number = 10;
        for (i, glyph) in self.griphs.iter().enumerate() {
            string += &format!("glyph {}\n", i);
            string += &glyph.to_string();
            if max_number < i {
                break;
            }
        }
        string
    }
}

fn get_glyf<R: BinaryReader>(file: &mut R, offset: u32, _length: u32, loca: &loca::LOCA) -> GLYF {
    let loca = loca.clone();
    file.seek(SeekFrom::Start(offset as u64)).unwrap();
    let offsets = loca.offsets.clone();
    let mut glyphs = Vec::new();
    for i in 0..offsets.len() - 1 {
        let offset = offsets[i];
        let length = offsets[i + 1] - offset;
        let glyph = get_glyph(file, offset, length);
        glyphs.push(glyph);
    }
    GLYF {
        griphs: Box::new(glyphs),
    }
}

fn get_glyph<R: BinaryReader>(file: &mut R, offset: u32, length: u32) -> Glyph {
    if length == 0 {
        return Glyph {
            offset,
            length,
            glyphs: Box::<Vec<u8>>::default(),
        };
    }

    let glyphs = file.read_bytes_as_vec(length as usize).unwrap();
    Glyph {
        glyphs: Box::new(glyphs),
        offset,
        length,
    }
}
