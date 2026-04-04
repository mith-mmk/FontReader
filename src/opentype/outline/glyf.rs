// Glyph Data
use std::{fmt, io::SeekFrom};

use bin_rs::reader::BinaryReader;

use crate::fontreader::{FontLayout, PathCommand};

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

const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const ROUND_XY_TO_GRID: u16 = 0x0004;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;
const SCALED_COMPONENT_OFFSET: u16 = 0x0800;

#[derive(Debug, Clone, Copy)]
struct CompositeTransform {
    xx: f64,
    xy: f64,
    yx: f64,
    yy: f64,
    dx: f64,
    dy: f64,
}

impl Default for CompositeTransform {
    fn default() -> Self {
        Self {
            xx: 1.0,
            xy: 0.0,
            yx: 0.0,
            yy: 1.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
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
    fn empty_parsed(&self) -> ParsedGlyph {
        ParsedGlyph {
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
        }
    }

    pub fn parse(&self) -> ParsedGlyph {
        if self.length < 10 {
            return self.empty_parsed();
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
        fonsize: f64,
        fontunit: &str,
        layout: &FontLayout,
    ) -> String {
        let parsed = self.parse();
        Self::get_svg_header_from_parsed(&parsed, fonsize, fontunit, layout)
    }

    pub(crate) fn get_svg_header_from_parsed(
        parsed: &ParsedGlyph,
        fontsize: f64,
        fontunit: &str,
        layout: &FontLayout,
    ) -> String {
        match layout {
            FontLayout::Horizontal(layout) => {
                let rsb = (layout.advance_width - parsed.x_max as isize) as i16;
                let x_min = parsed.x_min - layout.lsb as i16;
                let x_max = parsed.x_max + rsb;
                //        let y_max = layout.accender - layout.descender + layout.line_gap;
                let y_max = layout.accender - layout.descender + layout.line_gap;
                let y_min = if y_max > (parsed.y_max - parsed.y_min) as isize {
                    0
                } else {
                    y_max - (parsed.y_max - parsed.y_min) as isize
                };
                let height = fontsize;
                let width = x_min as f64 + x_max as f64;
                let width = width * height / y_max as f64;

                let height_str = format!("{}{}", height, fontunit);
                let width_str = format!("{}{}", width, fontunit);
                let svg = {
                    let base = format!(
                        "<svg width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" xmlns=\"http://www.w3.org/2000/svg\">",
                        width_str, height_str, x_min, y_min, x_max, y_max
                    );
                    #[cfg(debug_assertions)]
                    {
                        let rsb = (layout.advance_width - parsed.x_max as isize) as i16;
                        format!(
                            "{}<!-- lsb {} accender {} descender {} line_gap {} avance width {}--><!-- x min {} y min {} x max {} y max {} --><!-- offset {} length {} lsb {} advanced width {} rsb {} -->",
                            base,
                            layout.lsb,
                            layout.accender,
                            layout.descender,
                            layout.line_gap,
                            layout.advance_width,
                            parsed.x_min,
                            parsed.y_min,
                            parsed.x_max,
                            parsed.y_max,
                            parsed.offset,
                            parsed.length,
                            layout.lsb,
                            layout.advance_width,
                            rsb
                        )
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        base
                    }
                };
                svg
            }
            FontLayout::Vertical(layout) => {
                // let bsb = (layout.advance_height - parsed.y_max as isize) as i16;
                let x_min = 0;
                let y_min = layout.tsb;
                let descender = if layout.descender < 0 {
                    layout.descender
                } else {
                    -layout.descender
                };
                let x_max = layout.accender - descender;
                let y_max = layout.advance_height - layout.tsb;
                let height = fontsize;

                let height_str = format!("{}{}", height, fontunit);
                let width_str = format!("{}{}", height, fontunit);
                let svg = {
                    let base = format!(
                        "<svg width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" xmlns=\"http://www.w3.org/2000/svg\">\n",
                        width_str, height_str, x_min, y_min, x_max, y_max
                    );
                    #[cfg(debug_assertions)]
                    {
                        format!(
                            "{}<!-- x min {} y min {} x max {} y max {} -->\n<!-- layout height {} accender {}  descender {} tsb {}-->\n",
                            base,
                            parsed.x_min,
                            parsed.y_min,
                            parsed.x_max,
                            parsed.y_max,
                            layout.advance_height,
                            layout.accender,
                            layout.descender,
                            layout.tsb
                        )
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        base
                    }
                };
                svg
            }
            FontLayout::Unknown => "".to_string(),
        }
    }

    fn build_contours(parsed: &ParsedGlyph) -> Vec<Vec<(i16, i16, bool)>> {
        let mut contours = Vec::new();
        let mut contour_start = 0usize;
        let mut x = 0i16;
        let mut y = 0i16;

        for &contour_end in &parsed.end_pts_of_contours {
            let mut contour = Vec::new();
            for index in contour_start..=contour_end {
                x += parsed.xs[index];
                y += parsed.ys[index];
                contour.push((x, y, parsed.on_curves[index]));
            }
            contours.push(contour);
            contour_start = contour_end + 1;
        }

        contours
    }

    fn midpoint(a: (i16, i16), b: (i16, i16)) -> (i16, i16) {
        ((a.0 + b.0) / 2, (a.1 + b.1) / 2)
    }

    #[allow(dead_code)]
    pub(crate) fn get_svg_path(&self, layout: &FontLayout) -> String {
        let parsed = self.parse();
        Self::get_svg_path_parsed(&parsed, layout, 0.0, 0.0)
    }

    pub(crate) fn get_svg_path_parsed(
        parsed: &ParsedGlyph,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> String {
        let y_max = match layout {
            FontLayout::Horizontal(layout) => layout.accender as i16 + layout.line_gap as i16,
            FontLayout::Vertical(layout) => layout.accender as i16 - layout.descender as i16,
            FontLayout::Unknown => 0,
        };

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
        for contour in Self::build_contours(parsed) {
            if contour.is_empty() {
                continue;
            }

            let first = contour[0];
            let last = contour[contour.len() - 1];
            let start = if first.2 {
                (first.0, first.1)
            } else if last.2 {
                (last.0, last.1)
            } else {
                Self::midpoint((last.0, last.1), (first.0, first.1))
            };

            svg += &format!(
                "M{} {} ",
                start.0 + sx as i16,
                y_max - (start.1 + sy as i16)
            );

            let mut index = if first.2 { 1 } else { 0 };
            while index < contour.len() {
                let point = contour[index];
                if point.2 {
                    svg += &format!(
                        "L{} {} ",
                        point.0 + sx as i16,
                        y_max - (point.1 + sy as i16)
                    );
                    index += 1;
                } else {
                    let next = contour[(index + 1) % contour.len()];
                    let end = if next.2 {
                        index += 2;
                        (next.0, next.1)
                    } else {
                        index += 1;
                        Self::midpoint((point.0, point.1), (next.0, next.1))
                    };
                    svg += &format!(
                        "Q{} {} {} {} ",
                        point.0 + sx as i16,
                        y_max - (point.1 + sy as i16),
                        end.0 + sx as i16,
                        y_max - (end.1 + sy as i16)
                    );
                }
            }
            svg += "Z ";
        }
        svg += "\"/>";
        svg
    }

    pub fn to_path_commands(&self, layout: &FontLayout, sx: f64, sy: f64) -> Vec<PathCommand> {
        let parsed = self.parse();
        Self::to_path_commands_parsed(&parsed, layout, sx, sy)
    }

    pub fn to_path_commands_parsed(
        parsed: &ParsedGlyph,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> Vec<PathCommand> {
        let y_max = match layout {
            FontLayout::Horizontal(layout) => layout.accender as i16 + layout.line_gap as i16,
            FontLayout::Vertical(layout) => layout.accender as i16 - layout.descender as i16,
            FontLayout::Unknown => 0,
        };

        let mut commands = Vec::new();
        for contour in Self::build_contours(parsed) {
            if contour.is_empty() {
                continue;
            }

            let first = contour[0];
            let last = contour[contour.len() - 1];
            let start = if first.2 {
                (first.0, first.1)
            } else if last.2 {
                (last.0, last.1)
            } else {
                Self::midpoint((last.0, last.1), (first.0, first.1))
            };

            commands.push(PathCommand::MoveTo {
                x: (start.0 + sx as i16) as f64,
                y: (y_max - (start.1 + sy as i16)) as f64,
            });

            let mut index = if first.2 { 1 } else { 0 };
            while index < contour.len() {
                let point = contour[index];
                if point.2 {
                    commands.push(PathCommand::LineTo {
                        x: (point.0 + sx as i16) as f64,
                        y: (y_max - (point.1 + sy as i16)) as f64,
                    });
                    index += 1;
                } else {
                    let next = contour[(index + 1) % contour.len()];
                    let end = if next.2 {
                        index += 2;
                        (next.0, next.1)
                    } else {
                        index += 1;
                        Self::midpoint((point.0, point.1), (next.0, next.1))
                    };
                    commands.push(PathCommand::QuadTo {
                        cx: (point.0 + sx as i16) as f64,
                        cy: (y_max - (point.1 + sy as i16)) as f64,
                        x: (end.0 + sx as i16) as f64,
                        y: (y_max - (end.1 + sy as i16)) as f64,
                    });
                }
            }
            commands.push(PathCommand::ClosePath);
        }
        commands
    }

    pub fn to_svg(
        &self,
        fonsize: f64,
        fontunit: &str,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> String {
        let parsed = self.parse();
        let mut svg = Self::get_svg_header_from_parsed(&parsed, fonsize, fontunit, layout);

        // heightを後ろから読み出す
        svg += &Self::get_svg_path_parsed(&parsed, layout, sx, sy);
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

    #[allow(dead_code)]
    pub(crate) fn parse_glyph(&self, index: usize) -> Option<ParsedGlyph> {
        self.parse_glyph_recursive(index, 0)
    }

    pub(crate) fn parse_glyph_with_variation<F>(
        &self,
        index: usize,
        vary: &F,
    ) -> Option<ParsedGlyph>
    where
        F: Fn(usize, &ParsedGlyph) -> Option<ParsedGlyph>,
    {
        self.parse_glyph_recursive_with_variation(index, 0, vary)
    }

    pub fn to_path_commands(
        &self,
        index: usize,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> Vec<PathCommand> {
        let mut commands = self.to_path_commands_recursive(index, layout, 0);
        if sx != 0.0 || sy != 0.0 {
            commands = translate_path_commands(&commands, sx, sy);
        }
        commands
    }

    pub(crate) fn get_svg_path(
        &self,
        index: usize,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> String {
        let commands = self.to_path_commands(index, layout, sx, sy);
        if commands.is_empty() {
            return "<path d=\"\"/>".to_string();
        }
        format!("<path d=\"{}\"/>", path_commands_to_svg_path(&commands))
    }

    pub fn to_svg(
        &self,
        index: usize,
        fonsize: f64,
        fontunit: &str,
        layout: &FontLayout,
        sx: f64,
        sy: f64,
    ) -> String {
        let Some(glyph) = self.get_glyph(index) else {
            return String::new();
        };
        let parsed = glyph.parse();
        let mut svg = Glyph::get_svg_header_from_parsed(&parsed, fonsize, fontunit, layout);
        svg += &self.get_svg_path(index, layout, sx, sy);
        svg += "\n</svg>";
        svg
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

    fn to_path_commands_recursive(
        &self,
        index: usize,
        layout: &FontLayout,
        depth: usize,
    ) -> Vec<PathCommand> {
        if depth > 16 {
            return Vec::new();
        }

        let Some(glyph) = self.get_glyph(index) else {
            return Vec::new();
        };
        let parsed = glyph.parse();
        if parsed.number_of_contours >= 0 {
            return Glyph::to_path_commands_parsed(&parsed, layout, 0.0, 0.0);
        }

        self.composite_to_path_commands(glyph, layout, depth)
    }

    #[allow(dead_code)]
    fn parse_glyph_recursive(&self, index: usize, depth: usize) -> Option<ParsedGlyph> {
        self.parse_glyph_recursive_with_variation(index, depth, &|_, _| None)
    }

    fn parse_glyph_recursive_with_variation<F>(
        &self,
        index: usize,
        depth: usize,
        vary: &F,
    ) -> Option<ParsedGlyph>
    where
        F: Fn(usize, &ParsedGlyph) -> Option<ParsedGlyph>,
    {
        if depth > 16 {
            return None;
        }

        let glyph = self.get_glyph(index)?;
        let parsed = glyph.parse();
        let parsed = if parsed.number_of_contours >= 0 {
            parsed
        } else {
            self.parse_composite_glyph_with_variation(glyph, depth, vary)?
        };

        Some(vary(index, &parsed).unwrap_or(parsed))
    }

    fn parse_composite_glyph_with_variation<F>(
        &self,
        glyph: &Glyph,
        depth: usize,
        vary: &F,
    ) -> Option<ParsedGlyph>
    where
        F: Fn(usize, &ParsedGlyph) -> Option<ParsedGlyph>,
    {
        let mut offset = 10usize;
        let mut points = Vec::<(i16, i16, bool)>::new();
        let mut end_pts_of_contours = Vec::new();
        let mut instructions = Vec::new();

        while offset + 4 <= glyph.glyphs.len() {
            let flags = read_u16(&glyph.glyphs, &mut offset);
            let component_index = read_u16(&glyph.glyphs, &mut offset) as usize;
            let args_are_words = flags & ARG_1_AND_2_ARE_WORDS != 0;
            let args_are_xy_values = flags & ARGS_ARE_XY_VALUES != 0;

            let (arg1, arg2) = if args_are_words {
                if offset + 4 > glyph.glyphs.len() {
                    break;
                }
                (
                    read_i16(&glyph.glyphs, &mut offset) as f64,
                    read_i16(&glyph.glyphs, &mut offset) as f64,
                )
            } else {
                if offset + 2 > glyph.glyphs.len() {
                    break;
                }
                (
                    read_i8(&glyph.glyphs, &mut offset) as f64,
                    read_i8(&glyph.glyphs, &mut offset) as f64,
                )
            };

            let mut transform = CompositeTransform::default();
            if args_are_xy_values {
                transform.dx = arg1;
                transform.dy = arg2;
            } else {
                return None;
            }

            if flags & WE_HAVE_A_SCALE != 0 {
                let scale = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.xx = scale;
                transform.yy = scale;
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                transform.xx = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.yy = read_f2dot14(&glyph.glyphs, &mut offset);
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                transform.xx = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.yx = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.xy = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.yy = read_f2dot14(&glyph.glyphs, &mut offset);
            }

            if flags & SCALED_COMPONENT_OFFSET != 0 {
                let dx = transform.xx * transform.dx + transform.xy * transform.dy;
                let dy = transform.yx * transform.dx + transform.yy * transform.dy;
                transform.dx = dx;
                transform.dy = dy;
            }
            if flags & ROUND_XY_TO_GRID != 0 {
                transform.dx = transform.dx.round();
                transform.dy = transform.dy.round();
            }

            let component =
                self.parse_glyph_recursive_with_variation(component_index, depth + 1, vary)?;
            for contour in Glyph::build_contours(&component) {
                if contour.is_empty() {
                    continue;
                }

                for (x, y, on_curve) in contour {
                    let (x, y) = transform_outline_point(x as f64, y as f64, transform);
                    points.push((round_f64_to_i16(x), round_f64_to_i16(y), on_curve));
                }
                end_pts_of_contours.push(points.len() - 1);
            }

            if flags & MORE_COMPONENTS == 0 {
                if flags & WE_HAVE_INSTRUCTIONS != 0 && offset + 2 <= glyph.glyphs.len() {
                    let instruction_len = read_u16(&glyph.glyphs, &mut offset) as usize;
                    let end = offset
                        .saturating_add(instruction_len)
                        .min(glyph.glyphs.len());
                    instructions.extend_from_slice(&glyph.glyphs[offset..end]);
                }
                break;
            }
        }

        Some(parsed_glyph_from_points(
            glyph.offset,
            glyph.length,
            instructions,
            points,
            end_pts_of_contours,
        ))
    }

    fn composite_to_path_commands(
        &self,
        glyph: &Glyph,
        layout: &FontLayout,
        depth: usize,
    ) -> Vec<PathCommand> {
        let mut offset = 10usize;
        let mut commands = Vec::new();
        let y_max = layout_y_max(layout);

        while offset + 4 <= glyph.glyphs.len() {
            let flags = read_u16(&glyph.glyphs, &mut offset);
            let component_index = read_u16(&glyph.glyphs, &mut offset) as usize;
            let args_are_words = flags & ARG_1_AND_2_ARE_WORDS != 0;
            let args_are_xy_values = flags & ARGS_ARE_XY_VALUES != 0;

            let (arg1, arg2) = if args_are_words {
                if offset + 4 > glyph.glyphs.len() {
                    break;
                }
                (
                    read_i16(&glyph.glyphs, &mut offset) as f64,
                    read_i16(&glyph.glyphs, &mut offset) as f64,
                )
            } else {
                if offset + 2 > glyph.glyphs.len() {
                    break;
                }
                (
                    read_i8(&glyph.glyphs, &mut offset) as f64,
                    read_i8(&glyph.glyphs, &mut offset) as f64,
                )
            };

            let mut transform = CompositeTransform::default();
            if args_are_xy_values {
                transform.dx = arg1;
                transform.dy = arg2;
            }

            if flags & WE_HAVE_A_SCALE != 0 {
                let scale = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.xx = scale;
                transform.yy = scale;
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                transform.xx = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.yy = read_f2dot14(&glyph.glyphs, &mut offset);
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                transform.xx = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.yx = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.xy = read_f2dot14(&glyph.glyphs, &mut offset);
                transform.yy = read_f2dot14(&glyph.glyphs, &mut offset);
            }

            if flags & SCALED_COMPONENT_OFFSET != 0 {
                let dx = transform.xx * transform.dx + transform.xy * transform.dy;
                let dy = transform.yx * transform.dx + transform.yy * transform.dy;
                transform.dx = dx;
                transform.dy = dy;
            }
            if flags & ROUND_XY_TO_GRID != 0 {
                transform.dx = transform.dx.round();
                transform.dy = transform.dy.round();
            }

            let component_commands =
                self.to_path_commands_recursive(component_index, layout, depth + 1);
            commands.extend(transform_path_commands(
                &component_commands,
                y_max,
                transform,
            ));

            if flags & MORE_COMPONENTS == 0 {
                if flags & WE_HAVE_INSTRUCTIONS != 0 && offset + 2 <= glyph.glyphs.len() {
                    let _ = read_u16(&glyph.glyphs, &mut offset);
                }
                break;
            }
        }

        commands
    }
}

fn layout_y_max(layout: &FontLayout) -> f64 {
    match layout {
        FontLayout::Horizontal(layout) => (layout.accender + layout.line_gap) as f64,
        FontLayout::Vertical(layout) => (layout.accender - layout.descender) as f64,
        FontLayout::Unknown => 0.0,
    }
}

fn transform_path_commands(
    commands: &[PathCommand],
    y_max: f64,
    transform: CompositeTransform,
) -> Vec<PathCommand> {
    commands
        .iter()
        .map(|command| match command {
            PathCommand::MoveTo { x, y } => {
                let (x, y) = transform_screen_point(*x, *y, y_max, transform);
                PathCommand::MoveTo { x, y }
            }
            PathCommand::LineTo { x, y } => {
                let (x, y) = transform_screen_point(*x, *y, y_max, transform);
                PathCommand::LineTo { x, y }
            }
            PathCommand::QuadTo { cx, cy, x, y } => {
                let (cx, cy) = transform_screen_point(*cx, *cy, y_max, transform);
                let (x, y) = transform_screen_point(*x, *y, y_max, transform);
                PathCommand::QuadTo { cx, cy, x, y }
            }
            PathCommand::ClosePath => PathCommand::ClosePath,
        })
        .collect()
}

fn translate_path_commands(commands: &[PathCommand], dx: f64, dy: f64) -> Vec<PathCommand> {
    commands
        .iter()
        .map(|command| match command {
            PathCommand::MoveTo { x, y } => PathCommand::MoveTo {
                x: *x + dx,
                y: *y + dy,
            },
            PathCommand::LineTo { x, y } => PathCommand::LineTo {
                x: *x + dx,
                y: *y + dy,
            },
            PathCommand::QuadTo { cx, cy, x, y } => PathCommand::QuadTo {
                cx: *cx + dx,
                cy: *cy + dy,
                x: *x + dx,
                y: *y + dy,
            },
            PathCommand::ClosePath => PathCommand::ClosePath,
        })
        .collect()
}

fn transform_screen_point(x: f64, y: f64, y_max: f64, transform: CompositeTransform) -> (f64, f64) {
    let raw_y = y_max - y;
    let transformed_x = transform.xx * x + transform.xy * raw_y + transform.dx;
    let transformed_y = transform.yx * x + transform.yy * raw_y + transform.dy;
    (transformed_x, y_max - transformed_y)
}

fn transform_outline_point(x: f64, y: f64, transform: CompositeTransform) -> (f64, f64) {
    (
        transform.xx * x + transform.xy * y + transform.dx,
        transform.yx * x + transform.yy * y + transform.dy,
    )
}

fn parsed_glyph_from_points(
    offset: u32,
    length: u32,
    instructions: Vec<u8>,
    points: Vec<(i16, i16, bool)>,
    end_pts_of_contours: Vec<usize>,
) -> ParsedGlyph {
    let mut xs = Vec::with_capacity(points.len());
    let mut ys = Vec::with_capacity(points.len());
    let mut flags = Vec::with_capacity(points.len());
    let mut on_curves = Vec::with_capacity(points.len());
    let mut prev_x = 0i16;
    let mut prev_y = 0i16;
    let mut x_min = i16::MAX;
    let mut y_min = i16::MAX;
    let mut x_max = i16::MIN;
    let mut y_max = i16::MIN;

    for &(x, y, on_curve) in &points {
        xs.push(x.wrapping_sub(prev_x));
        ys.push(y.wrapping_sub(prev_y));
        flags.push(if on_curve { 0x01 } else { 0x00 });
        on_curves.push(on_curve);
        prev_x = x;
        prev_y = y;
        x_min = x_min.min(x);
        y_min = y_min.min(y);
        x_max = x_max.max(x);
        y_max = y_max.max(y);
    }

    if points.is_empty() {
        x_min = 0;
        y_min = 0;
        x_max = 0;
        y_max = 0;
    }

    ParsedGlyph {
        number_of_contours: end_pts_of_contours.len() as i16,
        x_min,
        y_min,
        x_max,
        y_max,
        offset,
        length,
        end_pts_of_contours,
        instructions,
        flags,
        xs,
        ys,
        on_curves,
    }
}

fn round_f64_to_i16(value: f64) -> i16 {
    value.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

fn read_u16(buf: &[u8], offset: &mut usize) -> u16 {
    let value = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]);
    *offset += 2;
    value
}

fn read_i16(buf: &[u8], offset: &mut usize) -> i16 {
    let value = i16::from_be_bytes([buf[*offset], buf[*offset + 1]]);
    *offset += 2;
    value
}

fn read_i8(buf: &[u8], offset: &mut usize) -> i8 {
    let value = buf[*offset] as i8;
    *offset += 1;
    value
}

fn read_f2dot14(buf: &[u8], offset: &mut usize) -> f64 {
    read_i16(buf, offset) as f64 / 16384.0
}

fn path_commands_to_svg_path(commands: &[PathCommand]) -> String {
    let mut d = String::new();
    for command in commands {
        match command {
            PathCommand::MoveTo { x, y } => d += &format!("M{} {} ", x, y),
            PathCommand::LineTo { x, y } => d += &format!("L{} {} ", x, y),
            PathCommand::QuadTo { cx, cy, x, y } => d += &format!("Q{} {} {} {} ", cx, cy, x, y),
            PathCommand::ClosePath => d += "Z ",
        }
    }
    d.trim_end().to_string()
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
