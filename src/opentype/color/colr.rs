#![allow(dead_code)]

use std::{io::Error, io::SeekFrom};

use bin_rs::reader::BinaryReader;

use crate::commands::{
    Command as DrawCommand, FillRule, GlyphBounds, GlyphGradientSpread, GlyphGradientStop,
    GlyphGradientUnits, GlyphLayer, GlyphLinearGradient, GlyphPaint, GlyphRadialGradient,
    PathGlyphLayer, PathPaintMode,
};
use crate::fontreader::{FontLayout, PathCommand};
use crate::opentype::color::cpal::Cpal;
use crate::opentype::outline::glyf::Glyf;

const MAX_COLR_TABLE_BYTES: u32 = 64 * 1024 * 1024;
const MAX_COLR_V1_DEPTH: usize = 64;
const MAX_COLR_V1_LAYERS: usize = 4096;

#[derive(Debug, Clone)]
pub(crate) struct Colr {
    version: u16,
    num_base_glyphs: u16,
    base_glyph_records: Vec<BaseGlyphRecord>,
    layer_records: Vec<LayerRecord>,
    num_layers: u16,
    v1: Option<ColrV1Data>,
}

#[derive(Debug, Clone)]
struct ColrV1Data {
    bytes: Vec<u8>,
    base_glyph_list: Vec<(u16, u32)>,
    layer_list: Vec<u32>,
    clips: Vec<(u16, u16, GlyphBounds)>,
}

impl Colr {
    pub(crate) fn is_version_zero(&self) -> bool {
        self.version == 0
    }

    pub(crate) fn has_glyph(&self, glyph_id: u16) -> bool {
        if let Some(v1) = &self.v1 {
            if v1
                .base_glyph_list
                .binary_search_by_key(&glyph_id, |(id, _)| *id)
                .is_ok()
            {
                return true;
            }
        }
        !self.get_layer_record(glyph_id).is_empty()
    }

    pub(crate) fn new<R: BinaryReader>(
        reader: &mut R,
        offset: u32,
        length: u32,
    ) -> Result<Self, Error> {
        if !(14..=MAX_COLR_TABLE_BYTES).contains(&length) {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "COLR table length out of bounds",
            ));
        }
        reader.seek(SeekFrom::Start(offset as u64))?;
        let bytes = reader.read_bytes_as_vec(length as usize)?;
        let version = read_u16(&bytes, 0)?;
        let num_base_glyphs = read_u16(&bytes, 2)?;
        let base_glyph_records_offset = read_u32(&bytes, 4)? as usize;
        let layer_records_offset = read_u32(&bytes, 8)? as usize;
        let num_layers = read_u16(&bytes, 12)?;

        let mut base_glyph_records = Vec::new();
        if num_base_glyphs > 0 {
            let needed = (num_base_glyphs as usize)
                .checked_mul(6)
                .and_then(|v| base_glyph_records_offset.checked_add(v))
                .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "COLR v0 overflow"))?;
            if needed > bytes.len() {
                return Err(Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "COLR base glyph records exceed table bounds",
                ));
            }
            for i in 0..num_base_glyphs as usize {
                let p = base_glyph_records_offset + i * 6;
                base_glyph_records.push(BaseGlyphRecord {
                    base_glyph: read_u16(&bytes, p)?,
                    first_layer_index: read_u16(&bytes, p + 2)?,
                    num_layers: read_u16(&bytes, p + 4)?,
                });
            }
        }

        let mut layer_records = Vec::new();
        if num_layers > 0 {
            let needed = (num_layers as usize)
                .checked_mul(4)
                .and_then(|v| layer_records_offset.checked_add(v))
                .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "COLR v0 overflow"))?;
            if needed > bytes.len() {
                return Err(Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "COLR layer records exceed table bounds",
                ));
            }
            for i in 0..num_layers as usize {
                let p = layer_records_offset + i * 4;
                layer_records.push(LayerRecord {
                    glyph_id: read_u16(&bytes, p)?,
                    palette_index: read_u16(&bytes, p + 2)?,
                });
            }
        }

        let v1 = if version == 1 {
            if bytes.len() < 34 {
                return Err(Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "COLR v1 header truncated",
                ));
            }
            let base_glyph_list_offset = read_u32(&bytes, 14)? as usize;
            let layer_list_offset = read_u32(&bytes, 18)? as usize;
            let clip_list_offset = read_u32(&bytes, 22)? as usize;
            let _var_index_map_offset = read_u32(&bytes, 26)?;
            let _item_variation_store_offset = read_u32(&bytes, 30)?;

            let base_glyph_list = if base_glyph_list_offset != 0 {
                parse_base_glyph_list(&bytes, base_glyph_list_offset)?
            } else {
                Vec::new()
            };
            let layer_list = if layer_list_offset != 0 {
                parse_layer_list(&bytes, layer_list_offset)?
            } else {
                Vec::new()
            };
            let clips = if clip_list_offset != 0 {
                parse_clip_list(&bytes, clip_list_offset)?
            } else {
                Vec::new()
            };
            Some(ColrV1Data {
                bytes,
                base_glyph_list,
                layer_list,
                clips,
            })
        } else {
            None
        };

        Ok(Self {
            version,
            num_base_glyphs,
            base_glyph_records,
            layer_records,
            num_layers,
            v1,
        })
    }

    fn dump_text(&self) -> String {
        let mut string = "COLR Table\n".to_string();
        string.push_str(&format!("version: {}\n", self.version));
        string.push_str(&format!("num_base_glyphs: {}\n", self.num_base_glyphs));
        string.push_str(&format!("num_layers: {}\n", self.num_layers));
        let max_length = 10;
        let len = if max_length < self.base_glyph_records.len() {
            max_length
        } else {
            self.base_glyph_records.len()
        };
        for (i, record) in self.base_glyph_records.iter().take(len).enumerate() {
            string.push_str(&format!("base_glyph_record[{}]: {:?}\n", i, record));
        }
        let len = if max_length < self.layer_records.len() {
            max_length
        } else {
            self.layer_records.len()
        };
        for (i, record) in self.layer_records.iter().take(len).enumerate() {
            string.push_str(&format!("layer_record[{}]: {:?}\n", i, record));
        }
        string
    }

    pub(crate) fn get_layer_record(&self, glyph_id: u16) -> Vec<LayerRecord> {
        let mut layer_records = Vec::new();
        let index = match self
            .base_glyph_records
            .binary_search_by_key(&glyph_id, |base| base.base_glyph)
        {
            Ok(index) => index,
            Err(_) => return layer_records,
        };
        let base_glyph_record = &self.base_glyph_records[index];
        let num_layers = base_glyph_record.num_layers;
        let first_layer_index = base_glyph_record.first_layer_index;
        for i in 0..num_layers {
            let layer_index = (first_layer_index + i) as usize;
            let Some(layer) = self.layer_records.get(layer_index) else {
                break;
            };
            layer_records.push(layer.clone());
        }
        layer_records
    }

    pub(crate) fn resolve_v1_layers(
        &self,
        glyph_id: u16,
        cpal: &Cpal,
        glyf: &Glyf,
        layout: &FontLayout,
        scale_x: f32,
        scale_y: f32,
    ) -> Option<Vec<GlyphLayer>> {
        let v1 = self.v1.as_ref()?;
        let idx = v1
            .base_glyph_list
            .binary_search_by_key(&glyph_id, |(id, _)| *id)
            .ok()?;
        let paint_offset = v1.base_glyph_list[idx].1;
        let mut ctx = EvalContext {
            v1,
            cpal,
            glyf,
            layout,
            scale_x,
            scale_y,
            visited: Vec::new(),
            out: Vec::new(),
        };
        ctx.eval_paint(paint_offset, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 0);
        if let Some(clip) = find_clip_box(&v1.clips, glyph_id) {
            let clip_cmds = clip_box_to_commands(clip, scale_x, scale_y);
            for layer in &mut ctx.out {
                if let GlyphLayer::Path(path) = layer {
                    if path.clip_commands.is_empty() {
                        path.clip_commands = clip_cmds.clone();
                    }
                }
            }
        }
        Some(ctx.out)
    }
}

struct EvalContext<'a> {
    v1: &'a ColrV1Data,
    cpal: &'a Cpal,
    glyf: &'a Glyf,
    layout: &'a FontLayout,
    scale_x: f32,
    scale_y: f32,
    visited: Vec<u32>,
    out: Vec<GlyphLayer>,
}

impl<'a> EvalContext<'a> {
    fn eval_paint(&mut self, offset: u32, transform: [f32; 6], depth: usize) {
        if depth >= MAX_COLR_V1_DEPTH || self.out.len() >= MAX_COLR_V1_LAYERS {
            return;
        }
        if self.visited.contains(&offset) {
            return;
        }
        let pos = offset as usize;
        let Ok(format) = read_u8(&self.v1.bytes, pos) else {
            return;
        };
        self.visited.push(offset);
        match format {
            // PaintColrLayers
            1 => {
                if let (Ok(num_layers), Ok(first)) = (
                    read_u8(&self.v1.bytes, pos + 1),
                    read_u32(&self.v1.bytes, pos + 2),
                ) {
                    let start = first as usize;
                    let end = start.saturating_add(num_layers as usize);
                    if end <= self.v1.layer_list.len() {
                        for &layer_off in &self.v1.layer_list[start..end] {
                            self.eval_paint(layer_off, transform, depth + 1);
                        }
                    }
                }
            }
            // PaintGlyph
            10 => {
                if let (Ok(child_rel), Ok(glyph_id)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_u16(&self.v1.bytes, pos + 4),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        if let Some(paint) = self.eval_paint_fill(child_off, transform, depth + 1) {
                            self.push_glyph_layer(glyph_id, transform, paint);
                        }
                    }
                }
            }
            // PaintColrGlyph
            11 => {
                if let Ok(glyph_id) = read_u16(&self.v1.bytes, pos + 1) {
                    if let Ok(idx) = self
                        .v1
                        .base_glyph_list
                        .binary_search_by_key(&glyph_id, |(id, _)| *id)
                    {
                        let child_off = self.v1.base_glyph_list[idx].1;
                        self.eval_paint(child_off, transform, depth + 1);
                    }
                }
            }
            // PaintTransform / PaintVarTransform
            12 | 13 => {
                if let (Ok(child_rel), Ok(tr_rel)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_u24(&self.v1.bytes, pos + 4),
                ) {
                    if let (Some(child_off), Some(tr_off)) =
                        (offset.checked_add(child_rel), offset.checked_add(tr_rel))
                    {
                        if let Ok(m) = read_affine2x3(&self.v1.bytes, tr_off as usize) {
                            self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                        }
                    }
                }
            }
            // PaintTranslate / PaintVarTranslate
            14 | 15 => {
                if let (Ok(child_rel), Ok(dx), Ok(dy)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_i16(&self.v1.bytes, pos + 4),
                    read_i16(&self.v1.bytes, pos + 6),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let m = [1.0, 0.0, 0.0, 1.0, dx as f32, dy as f32];
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintScale / PaintVarScale
            16 | 17 => {
                if let (Ok(child_rel), Ok(sx), Ok(sy)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                    read_f2dot14(&self.v1.bytes, pos + 6),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let m = [sx, 0.0, 0.0, sy, 0.0, 0.0];
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintScaleAroundCenter / PaintVarScaleAroundCenter
            18 | 19 => {
                if let (Ok(child_rel), Ok(sx), Ok(sy), Ok(cx), Ok(cy)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                    read_f2dot14(&self.v1.bytes, pos + 6),
                    read_i16(&self.v1.bytes, pos + 8),
                    read_i16(&self.v1.bytes, pos + 10),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let m = around_center([sx, 0.0, 0.0, sy, 0.0, 0.0], cx as f32, cy as f32);
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintScaleUniform / PaintVarScaleUniform
            20 | 21 => {
                if let (Ok(child_rel), Ok(s)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let m = [s, 0.0, 0.0, s, 0.0, 0.0];
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintScaleUniformAroundCenter / PaintVarScaleUniformAroundCenter
            22 | 23 => {
                if let (Ok(child_rel), Ok(s), Ok(cx), Ok(cy)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                    read_i16(&self.v1.bytes, pos + 6),
                    read_i16(&self.v1.bytes, pos + 8),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let m = around_center([s, 0.0, 0.0, s, 0.0, 0.0], cx as f32, cy as f32);
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintRotate / PaintVarRotate
            24 | 25 => {
                if let (Ok(child_rel), Ok(angle)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let rad = angle * std::f32::consts::PI;
                        let (sin, cos) = rad.sin_cos();
                        let m = [cos, sin, -sin, cos, 0.0, 0.0];
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintRotateAroundCenter / PaintVarRotateAroundCenter
            26 | 27 => {
                if let (Ok(child_rel), Ok(angle), Ok(cx), Ok(cy)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                    read_i16(&self.v1.bytes, pos + 6),
                    read_i16(&self.v1.bytes, pos + 8),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let rad = angle * std::f32::consts::PI;
                        let (sin, cos) = rad.sin_cos();
                        let m =
                            around_center([cos, sin, -sin, cos, 0.0, 0.0], cx as f32, cy as f32);
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintSkew / PaintVarSkew
            28 | 29 => {
                if let (Ok(child_rel), Ok(x_skew), Ok(y_skew)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                    read_f2dot14(&self.v1.bytes, pos + 6),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let tx = (x_skew * std::f32::consts::PI).tan();
                        let ty = (y_skew * std::f32::consts::PI).tan();
                        let m = [1.0, ty, -tx, 1.0, 0.0, 0.0];
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintSkewAroundCenter / PaintVarSkewAroundCenter
            30 | 31 => {
                if let (Ok(child_rel), Ok(x_skew), Ok(y_skew), Ok(cx), Ok(cy)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_f2dot14(&self.v1.bytes, pos + 4),
                    read_f2dot14(&self.v1.bytes, pos + 6),
                    read_i16(&self.v1.bytes, pos + 8),
                    read_i16(&self.v1.bytes, pos + 10),
                ) {
                    if let Some(child_off) = offset.checked_add(child_rel) {
                        let tx = (x_skew * std::f32::consts::PI).tan();
                        let ty = (y_skew * std::f32::consts::PI).tan();
                        let m = around_center([1.0, ty, -tx, 1.0, 0.0, 0.0], cx as f32, cy as f32);
                        self.eval_paint(child_off, mul_affine(transform, m), depth + 1);
                    }
                }
            }
            // PaintComposite
            32 => {
                if let (Ok(src_rel), Ok(_mode), Ok(dst_rel)) = (
                    read_u24(&self.v1.bytes, pos + 1),
                    read_u8(&self.v1.bytes, pos + 4),
                    read_u24(&self.v1.bytes, pos + 5),
                ) {
                    if let Some(dst_off) = offset.checked_add(dst_rel) {
                        self.eval_paint(dst_off, transform, depth + 1);
                    }
                    if let Some(src_off) = offset.checked_add(src_rel) {
                        self.eval_paint(src_off, transform, depth + 1);
                    }
                }
            }
            _ => {}
        }
        self.visited.pop();
    }

    fn eval_paint_fill(
        &mut self,
        offset: u32,
        transform: [f32; 6],
        depth: usize,
    ) -> Option<GlyphPaint> {
        if depth >= MAX_COLR_V1_DEPTH || self.visited.contains(&offset) {
            return None;
        }
        let pos = offset as usize;
        let format = read_u8(&self.v1.bytes, pos).ok()?;
        match format {
            // PaintSolid / PaintVarSolid
            2 | 3 => {
                let palette_index = read_u16(&self.v1.bytes, pos + 1).ok()?;
                let alpha = read_f2dot14(&self.v1.bytes, pos + 3).ok()?.clamp(0.0, 1.0);
                if palette_index == 0xffff {
                    return Some(GlyphPaint::CurrentColor);
                }
                let color = self.cpal.get_palette_color(0, palette_index)?;
                let a = ((color.alpha as f32) * alpha).round().clamp(0.0, 255.0) as u32;
                let argb = (a << 24)
                    | ((color.red as u32) << 16)
                    | ((color.green as u32) << 8)
                    | color.blue as u32;
                Some(GlyphPaint::Solid(argb))
            }
            // PaintLinearGradient / PaintVarLinearGradient
            4 | 5 => {
                let color_line_rel = read_u24(&self.v1.bytes, pos + 1).ok()?;
                let x0 = read_i16(&self.v1.bytes, pos + 4).ok()? as f32;
                let y0 = read_i16(&self.v1.bytes, pos + 6).ok()? as f32;
                let x1 = read_i16(&self.v1.bytes, pos + 8).ok()? as f32;
                let y1 = read_i16(&self.v1.bytes, pos + 10).ok()? as f32;
                let x2 = read_i16(&self.v1.bytes, pos + 12).ok()? as f32;
                let y2 = read_i16(&self.v1.bytes, pos + 14).ok()? as f32;
                let color_line_off = offset.checked_add(color_line_rel)?;
                let (spread, stops) =
                    parse_color_line(&self.v1.bytes, color_line_off as usize, self.cpal)?;

                // Project p1 onto the vector perpendicular to (p2 - p0).
                let dx = x2 - x0;
                let dy = y2 - y0;
                let (ex, ey) = if dx * dx + dy * dy > 0.0 {
                    let nx = -dy;
                    let ny = dx;
                    let denom = nx * nx + ny * ny;
                    let t = ((x1 - x0) * nx + (y1 - y0) * ny) / denom;
                    (x0 + t * nx, y0 + t * ny)
                } else {
                    (x1, y1)
                };

                let (sx0, sy0) = self.transform_font_point(transform, x0, y0);
                let (sx1, sy1) = self.transform_font_point(transform, ex, ey);
                Some(GlyphPaint::LinearGradient(GlyphLinearGradient {
                    x1: sx0,
                    y1: sy0,
                    x2: sx1,
                    y2: sy1,
                    units: GlyphGradientUnits::UserSpaceOnUse,
                    transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    spread,
                    stops,
                }))
            }
            // PaintRadialGradient / PaintVarRadialGradient
            6 | 7 => {
                let color_line_rel = read_u24(&self.v1.bytes, pos + 1).ok()?;
                let x0 = read_i16(&self.v1.bytes, pos + 4).ok()? as f32;
                let y0 = read_i16(&self.v1.bytes, pos + 6).ok()? as f32;
                let _r0 = read_u16(&self.v1.bytes, pos + 8).ok()? as f32;
                let x1 = read_i16(&self.v1.bytes, pos + 10).ok()? as f32;
                let y1 = read_i16(&self.v1.bytes, pos + 12).ok()? as f32;
                let r1 = read_u16(&self.v1.bytes, pos + 14).ok()? as f32;
                let color_line_off = offset.checked_add(color_line_rel)?;
                let (spread, stops) =
                    parse_color_line(&self.v1.bytes, color_line_off as usize, self.cpal)?;

                let (fx, fy) = self.transform_font_point(transform, x0, y0);
                let (cx, cy) = self.transform_font_point(transform, x1, y1);
                let avg_scale = ((transform[0] * self.scale_x).hypot(transform[1] * self.scale_y)
                    + (transform[2] * self.scale_x).hypot(transform[3] * self.scale_y))
                    * 0.5;
                Some(GlyphPaint::RadialGradient(GlyphRadialGradient {
                    cx,
                    cy,
                    r: (r1 * avg_scale).max(0.0),
                    fx,
                    fy,
                    units: GlyphGradientUnits::UserSpaceOnUse,
                    transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    spread,
                    stops,
                }))
            }
            _ => None,
        }
    }

    fn transform_font_point(&self, m: [f32; 6], x: f32, y: f32) -> (f32, f32) {
        let tx = m[0] * x + m[2] * y + m[4];
        let ty = m[1] * x + m[3] * y + m[5];
        (tx * self.scale_x, -ty * self.scale_y)
    }

    fn push_glyph_layer(&mut self, glyph_id: u16, m: [f32; 6], paint: GlyphPaint) {
        if self.glyf.get_glyph(glyph_id as usize).is_none() {
            return;
        }
        let baseline_shift = match self.layout {
            FontLayout::Horizontal(layout) => (layout.accender + layout.line_gap) as f64,
            FontLayout::Vertical(layout) => (layout.accender - layout.descender) as f64,
            FontLayout::Unknown => 0.0,
        };
        let raw_commands = self
            .glyf
            .to_path_commands(glyph_id as usize, self.layout, 0.0, 0.0);
        let to_draw = |px: f64, py: f64| -> (f32, f32) {
            let fx = px as f32;
            let fy = (baseline_shift - py) as f32;
            self.transform_font_point(m, fx, fy)
        };
        let commands: Vec<DrawCommand> = raw_commands
            .into_iter()
            .map(|cmd| match cmd {
                PathCommand::MoveTo { x, y } => {
                    let (dx, dy) = to_draw(x, y);
                    DrawCommand::MoveTo(dx, dy)
                }
                PathCommand::LineTo { x, y } => {
                    let (dx, dy) = to_draw(x, y);
                    DrawCommand::Line(dx, dy)
                }
                PathCommand::QuadTo { cx, cy, x, y } => {
                    let (dcx, dcy) = to_draw(cx, cy);
                    let (dx, dy) = to_draw(x, y);
                    DrawCommand::Bezier((dcx, dcy), (dx, dy))
                }
                PathCommand::ClosePath => DrawCommand::Close,
            })
            .collect();
        let mut layer = PathGlyphLayer::new(commands, paint);
        layer.fill_rule = FillRule::NonZero;
        layer.paint_mode = PathPaintMode::Fill;
        self.out.push(GlyphLayer::Path(layer));
    }
}

fn parse_base_glyph_list(bytes: &[u8], offset: usize) -> Result<Vec<(u16, u32)>, Error> {
    let count = read_u32(bytes, offset)? as usize;
    let needed = count
        .checked_mul(6)
        .and_then(|v| offset.checked_add(4)?.checked_add(v))
        .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "BaseGlyphList overflow"))?;
    if needed > bytes.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "BaseGlyphList exceeds table bounds",
        ));
    }
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let p = offset + 4 + i * 6;
        let glyph_id = read_u16(bytes, p)?;
        let paint_rel = read_u32(bytes, p + 2)?;
        let paint_abs = (offset as u32)
            .checked_add(paint_rel)
            .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "Paint offset overflow"))?;
        records.push((glyph_id, paint_abs));
    }
    records.sort_by_key(|(id, _)| *id);
    Ok(records)
}

fn parse_layer_list(bytes: &[u8], offset: usize) -> Result<Vec<u32>, Error> {
    let count = read_u32(bytes, offset)? as usize;
    let needed = count
        .checked_mul(4)
        .and_then(|v| offset.checked_add(4)?.checked_add(v))
        .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "LayerList overflow"))?;
    if needed > bytes.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "LayerList exceeds table bounds",
        ));
    }
    let mut offsets = Vec::with_capacity(count);
    for i in 0..count {
        let rel = read_u32(bytes, offset + 4 + i * 4)?;
        let abs = (offset as u32)
            .checked_add(rel)
            .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "Layer paint overflow"))?;
        offsets.push(abs);
    }
    Ok(offsets)
}

fn parse_clip_list(bytes: &[u8], offset: usize) -> Result<Vec<(u16, u16, GlyphBounds)>, Error> {
    let format = read_u8(bytes, offset)?;
    if format != 1 {
        return Ok(Vec::new());
    }
    let count = read_u32(bytes, offset + 1)? as usize;
    let needed = count
        .checked_mul(7)
        .and_then(|v| offset.checked_add(5)?.checked_add(v))
        .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "ClipList overflow"))?;
    if needed > bytes.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "ClipList exceeds table bounds",
        ));
    }
    let mut clips = Vec::with_capacity(count);
    for i in 0..count {
        let p = offset + 5 + i * 7;
        let start_gid = read_u16(bytes, p)?;
        let end_gid = read_u16(bytes, p + 2)?;
        let clip_rel = read_u24(bytes, p + 4)? as usize;
        let clip_abs = offset
            .checked_add(clip_rel)
            .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidData, "ClipBox overflow"))?;
        let box_fmt = read_u8(bytes, clip_abs)?;
        if box_fmt == 1 || box_fmt == 2 {
            let x_min = read_i16(bytes, clip_abs + 1)? as f32;
            let y_min = read_i16(bytes, clip_abs + 3)? as f32;
            let x_max = read_i16(bytes, clip_abs + 5)? as f32;
            let y_max = read_i16(bytes, clip_abs + 7)? as f32;
            clips.push((
                start_gid,
                end_gid,
                GlyphBounds {
                    min_x: x_min,
                    min_y: y_min,
                    max_x: x_max,
                    max_y: y_max,
                },
            ));
        }
    }
    Ok(clips)
}

fn find_clip_box(clips: &[(u16, u16, GlyphBounds)], glyph_id: u16) -> Option<GlyphBounds> {
    clips
        .iter()
        .find(|(s, e, _)| glyph_id >= *s && glyph_id <= *e)
        .map(|(_, _, b)| *b)
}

fn clip_box_to_commands(b: GlyphBounds, scale_x: f32, scale_y: f32) -> Vec<DrawCommand> {
    let x0 = b.min_x * scale_x;
    let y0 = -b.max_y * scale_y;
    let x1 = b.max_x * scale_x;
    let y1 = -b.min_y * scale_y;
    vec![
        DrawCommand::MoveTo(x0, y0),
        DrawCommand::Line(x1, y0),
        DrawCommand::Line(x1, y1),
        DrawCommand::Line(x0, y1),
        DrawCommand::Close,
    ]
}

fn parse_color_line(
    bytes: &[u8],
    offset: usize,
    cpal: &Cpal,
) -> Option<(GlyphGradientSpread, Vec<GlyphGradientStop>)> {
    let extend = read_u8(bytes, offset).ok()?;
    let spread = match extend {
        1 => GlyphGradientSpread::Repeat,
        2 => GlyphGradientSpread::Reflect,
        _ => GlyphGradientSpread::Pad,
    };
    let num_stops = read_u16(bytes, offset + 1).ok()? as usize;
    let end = num_stops.checked_mul(6)?.checked_add(offset + 3)?;
    if end > bytes.len() {
        return None;
    }
    let mut stops = Vec::with_capacity(num_stops);
    for i in 0..num_stops {
        let p = offset + 3 + i * 6;
        let stop_offset = read_f2dot14(bytes, p).ok()?;
        let palette_index = read_u16(bytes, p + 2).ok()?;
        let alpha = read_f2dot14(bytes, p + 4).ok()?.clamp(0.0, 1.0);
        let color = if palette_index == 0xffff {
            0xFF00_0000
        } else {
            let rec = cpal.get_palette_color(0, palette_index)?;
            let a = ((rec.alpha as f32) * alpha).round().clamp(0.0, 255.0) as u32;
            (a << 24) | ((rec.red as u32) << 16) | ((rec.green as u32) << 8) | rec.blue as u32
        };
        stops.push(GlyphGradientStop {
            offset: stop_offset,
            color,
        });
    }
    Some((spread, stops))
}

fn mul_affine(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn around_center(m: [f32; 6], cx: f32, cy: f32) -> [f32; 6] {
    let t1 = [1.0, 0.0, 0.0, 1.0, cx, cy];
    let t2 = [1.0, 0.0, 0.0, 1.0, -cx, -cy];
    mul_affine(mul_affine(t1, m), t2)
}

fn read_affine2x3(bytes: &[u8], offset: usize) -> Result<[f32; 6], Error> {
    Ok([
        read_fixed(bytes, offset)?,
        read_fixed(bytes, offset + 4)?,
        read_fixed(bytes, offset + 8)?,
        read_fixed(bytes, offset + 12)?,
        read_fixed(bytes, offset + 16)?,
        read_fixed(bytes, offset + 20)?,
    ])
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, Error> {
    bytes
        .get(offset)
        .copied()
        .ok_or_else(|| Error::new(std::io::ErrorKind::UnexpectedEof, "COLR EOF"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| Error::new(std::io::ErrorKind::UnexpectedEof, "COLR EOF"))?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, Error> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| Error::new(std::io::ErrorKind::UnexpectedEof, "COLR EOF"))?;
    Ok(i16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u24(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let slice = bytes
        .get(offset..offset + 3)
        .ok_or_else(|| Error::new(std::io::ErrorKind::UnexpectedEof, "COLR EOF"))?;
    Ok(((slice[0] as u32) << 16) | ((slice[1] as u32) << 8) | slice[2] as u32)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| Error::new(std::io::ErrorKind::UnexpectedEof, "COLR EOF"))?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, Error> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| Error::new(std::io::ErrorKind::UnexpectedEof, "COLR EOF"))?;
    Ok(i32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_f2dot14(bytes: &[u8], offset: usize) -> Result<f32, Error> {
    Ok(read_i16(bytes, offset)? as f32 / 16384.0)
}

fn read_fixed(bytes: &[u8], offset: usize) -> Result<f32, Error> {
    Ok(read_i32(bytes, offset)? as f32 / 65536.0)
}

#[derive(Debug, Clone)]
pub(crate) struct BaseGlyphRecord {
    base_glyph: u16,
    first_layer_index: u16,
    num_layers: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct LayerRecord {
    pub(crate) glyph_id: u16,
    pub(crate) palette_index: u16, // see CPAL
}

impl std::fmt::Display for Colr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.dump_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    #[test]
    fn colr_returns_error_on_truncated_input() {
        let mut reader = BytesReader::new(&[0x00, 0x00, 0x00]);
        assert!(Colr::new(&mut reader, 0, 3).is_err());
    }

    #[test]
    fn colr_get_layer_record_stops_at_available_layers() {
        let colr = Colr {
            version: 0,
            num_base_glyphs: 1,
            base_glyph_records: vec![BaseGlyphRecord {
                base_glyph: 7,
                first_layer_index: 1,
                num_layers: 3,
            }],
            layer_records: vec![LayerRecord {
                glyph_id: 100,
                palette_index: 0,
            }],
            num_layers: 1,
            v1: None,
        };

        assert!(colr.get_layer_record(7).is_empty());
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn display_preserves_color_layer_dump() {
        let colr = Colr {
            version: 0,
            num_base_glyphs: 1,
            base_glyph_records: vec![BaseGlyphRecord {
                base_glyph: 7,
                first_layer_index: 0,
                num_layers: 1,
            }],
            layer_records: vec![LayerRecord {
                glyph_id: 8,
                palette_index: 2,
            }],
            num_layers: 1,
            v1: None,
        };
        let expected = "COLR Table\nversion: 0\nnum_base_glyphs: 1\nnum_layers: 1\nbase_glyph_record[0]: BaseGlyphRecord { base_glyph: 7, first_layer_index: 0, num_layers: 1 }\nlayer_record[0]: LayerRecord { glyph_id: 8, palette_index: 2 }\n";
        assert_eq!(colr.to_string(), expected);
        assert_eq!(format!("{colr}"), expected);
    }
}
