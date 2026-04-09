use crate::commands::{
    Command, FillRule, GlyphGradientSpread, GlyphGradientStop, GlyphGradientUnits,
    GlyphLinearGradient, GlyphPaint, GlyphRadialGradient, PathGlyphLayer,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum SvgNode {
    Element(SvgElement),
    Text,
}

#[derive(Debug, Clone)]
struct SvgElement {
    name: String,
    attrs: HashMap<String, String>,
    children: Vec<SvgNode>,
}

#[derive(Debug, Clone)]
struct RenderState {
    transform: Transform2D,
    fill: Option<GlyphPaint>,
    fill_rule: FillRule,
    stroke: Option<GlyphPaint>,
    stroke_width: f32,
    clip_path: Option<ClipPathSpec>,
    mask_path: Option<ClipPathSpec>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            transform: Transform2D::IDENTITY,
            fill: Some(GlyphPaint::CurrentColor),
            fill_rule: FillRule::NonZero,
            stroke: None,
            stroke_width: 1.0,
            clip_path: None,
            mask_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipPathUnits {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

#[derive(Debug, Clone)]
struct ClipPathSpec {
    commands: Vec<Command>,
    units: ClipPathUnits,
}

#[derive(Debug, Clone, Copy)]
struct Transform2D {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Transform2D {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn multiply(self, next: Self) -> Self {
        Self {
            a: self.a * next.a + self.c * next.b,
            b: self.b * next.a + self.d * next.b,
            c: self.a * next.c + self.c * next.d,
            d: self.b * next.c + self.d * next.d,
            e: self.a * next.e + self.c * next.f + self.e,
            f: self.b * next.e + self.d * next.f + self.f,
        }
    }

    fn translate(x: f32, y: f32) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }

    fn scale(x: f32, y: f32) -> Self {
        Self {
            a: x,
            d: y,
            ..Self::IDENTITY
        }
    }

    fn rotate_radians(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            ..Self::IDENTITY
        }
    }

    fn skew_x_radians(angle: f32) -> Self {
        Self {
            c: angle.tan(),
            ..Self::IDENTITY
        }
    }

    fn skew_y_radians(angle: f32) -> Self {
        Self {
            b: angle.tan(),
            ..Self::IDENTITY
        }
    }

    fn apply(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
}

pub(crate) fn svg_to_path_layers(
    document: &str,
    scale_x: f32,
    scale_y: f32,
) -> Vec<PathGlyphLayer> {
    let Ok(root) = parse_svg_document(document) else {
        return Vec::new();
    };

    let mut defs = HashMap::new();
    collect_definitions(&root, &mut defs);

    let mut layers = Vec::new();
    let state = RenderState::default();
    flatten_node(
        &root,
        &defs,
        &state,
        scale_x,
        scale_y,
        &mut layers,
    );
    layers
}

pub(crate) fn svg_requires_svg_fallback(document: &str) -> bool {
    let lowered = document.to_ascii_lowercase();
    lowered.contains("<pattern")
        || lowered.contains("<mask")
        || lowered.contains("<filter")
        || lowered.contains("patternunits=")
        || lowered.contains("maskunits=")
        || lowered.contains("maskcontentunits=")
        || lowered.contains(" filter=")
        || lowered.contains("filter=\"")
        || lowered.contains("mask=\"")
        || lowered.contains("mask:")
}

fn parse_svg_document(document: &str) -> Result<SvgNode, ()> {
    let mut stack: Vec<SvgElement> = vec![SvgElement {
        name: "#document".to_string(),
        attrs: HashMap::new(),
        children: Vec::new(),
    }];
    let mut cursor = 0usize;

    while cursor < document.len() {
        let Some(relative_start) = document[cursor..].find('<') else {
            let trailing = &document[cursor..];
            if !trailing.trim().is_empty() {
                stack
                    .last_mut()
                    .expect("document root")
                    .children
                    .push(SvgNode::Text);
            }
            break;
        };
        let start = cursor + relative_start;
        let text = &document[cursor..start];
        if !text.trim().is_empty() {
            stack
                .last_mut()
                .expect("document root")
                .children
                .push(SvgNode::Text);
        }

        let Some(relative_end) = document[start..].find('>') else {
            return Err(());
        };
        let end = start + relative_end + 1;
        let tag = &document[start..end];
        cursor = end;

        if tag.starts_with("<?") || tag.starts_with("<!") {
            continue;
        }

        if tag.starts_with("</") {
            let Some(name) = closing_tag_name(tag) else {
                return Err(());
            };
            let element = stack.pop().ok_or(())?;
            if element.name != name.to_ascii_lowercase() {
                return Err(());
            }
            stack
                .last_mut()
                .ok_or(())?
                .children
                .push(SvgNode::Element(element));
            continue;
        }

        let Some(name) = tag_name(tag) else {
            return Err(());
        };
        let attrs = parse_attributes(tag);
        let self_closing = tag[..tag.len().saturating_sub(1)].trim_end().ends_with('/');
        let element = SvgElement {
            name: name.to_ascii_lowercase(),
            attrs,
            children: Vec::new(),
        };
        if self_closing {
            stack
                .last_mut()
                .ok_or(())?
                .children
                .push(SvgNode::Element(element));
        } else {
            stack.push(element);
        }
    }

    while stack.len() > 1 {
        let element = stack.pop().ok_or(())?;
        stack
            .last_mut()
            .ok_or(())?
            .children
            .push(SvgNode::Element(element));
    }

    Ok(SvgNode::Element(stack.pop().ok_or(())?))
}

fn collect_definitions(node: &SvgNode, defs: &mut HashMap<String, SvgElement>) {
    let SvgNode::Element(element) = node else {
        return;
    };

    if element.name == "defs" {
        collect_id_elements(&element.children, defs);
    }

    for child in &element.children {
        collect_definitions(child, defs);
    }
}

fn collect_id_elements(children: &[SvgNode], defs: &mut HashMap<String, SvgElement>) {
    for child in children {
        let SvgNode::Element(element) = child else {
            continue;
        };
        if let Some(id) = element.attrs.get("id") {
            defs.entry(id.clone()).or_insert_with(|| element.clone());
        }
        collect_id_elements(&element.children, defs);
    }
}

fn flatten_node(
    node: &SvgNode,
    defs: &HashMap<String, SvgElement>,
    state: &RenderState,
    scale_x: f32,
    scale_y: f32,
    out: &mut Vec<PathGlyphLayer>,
) {
    let SvgNode::Element(element) = node else {
        return;
    };

    if element.name == "#document" {
        for child in &element.children {
            flatten_node(child, defs, state, scale_x, scale_y, out);
        }
        return;
    }

    if element.name == "defs" {
        return;
    }

    let style = element.attrs.get("style").map(|value| parse_style(value));
    let local_fill = resolve_paint(
        style.as_ref().and_then(|style| style.get("fill")),
        element.attrs.get("fill"),
        state.fill.as_ref(),
        defs,
        scale_x,
        scale_y,
    );
    let local_fill_rule = style
        .as_ref()
        .and_then(|style| style.get("fill-rule"))
        .or_else(|| element.attrs.get("fill-rule"))
        .map(|value| parse_fill_rule(value))
        .unwrap_or(state.fill_rule);
    let local_stroke = resolve_paint(
        style.as_ref().and_then(|style| style.get("stroke")),
        element.attrs.get("stroke"),
        state.stroke.as_ref(),
        defs,
        scale_x,
        scale_y,
    );
    let local_stroke_width = style
        .as_ref()
        .and_then(|style| style.get("stroke-width"))
        .or_else(|| element.attrs.get("stroke-width"))
        .and_then(|value| parse_number(value))
        .map(|value| value * scale_x.abs().max(scale_y.abs()))
        .unwrap_or(state.stroke_width);
    let local_clip_path = resolve_clip_path(
        style.as_ref().and_then(|style| style.get("clip-path")),
        element.attrs.get("clip-path"),
        state.clip_path.as_ref(),
        defs,
        scale_x,
        scale_y,
    );
    let local_mask_path = resolve_mask_path(
        style.as_ref().and_then(|style| style.get("mask")),
        element.attrs.get("mask"),
        state.mask_path.as_ref(),
        defs,
        scale_x,
        scale_y,
    );
    let next_state = RenderState {
        transform: state.transform.multiply(parse_transform(
            style
                .as_ref()
                .and_then(|style| style.get("transform"))
                .or_else(|| element.attrs.get("transform")),
            scale_x,
            scale_y,
        )),
        fill: local_fill,
        fill_rule: local_fill_rule,
        stroke: local_stroke,
        stroke_width: local_stroke_width,
        clip_path: local_clip_path,
        mask_path: local_mask_path,
    };

    match element.name.as_str() {
        "svg" | "g" | "symbol" => {
            for child in &element.children {
                flatten_node(child, defs, &next_state, scale_x, scale_y, out);
            }
        }
        "use" => flatten_use(element, defs, next_state, scale_x, scale_y, out),
        "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
            out.extend(element_to_path_layers(element, &next_state, scale_x, scale_y));
        }
        _ => {}
    }
}

fn flatten_use(
    element: &SvgElement,
    defs: &HashMap<String, SvgElement>,
    state: RenderState,
    scale_x: f32,
    scale_y: f32,
    out: &mut Vec<PathGlyphLayer>,
) {
    let Some(reference) = element
        .attrs
        .get("href")
        .or_else(|| element.attrs.get("xlink:href"))
        .and_then(|value| value.strip_prefix('#'))
    else {
        return;
    };
    let Some(referenced) = defs.get(reference) else {
        return;
    };
    let mut referenced = referenced.clone();
    for key in ["fill", "fill-rule", "stroke", "stroke-width"] {
        if let Some(value) = element.attrs.get(key) {
            referenced.attrs.insert(key.to_string(), value.clone());
        }
    }

    let use_state = RenderState {
        transform: state.transform.multiply(Transform2D::translate(
            parse_attr_f32(&element.attrs, "x").unwrap_or(0.0) * scale_x,
            parse_attr_f32(&element.attrs, "y").unwrap_or(0.0) * scale_y,
        )),
        fill: state.fill,
        fill_rule: state.fill_rule,
        stroke: state.stroke,
        stroke_width: state.stroke_width,
        clip_path: state.clip_path,
        mask_path: state.mask_path,
    };
    flatten_node(
        &SvgNode::Element(referenced),
        defs,
        &use_state,
        scale_x,
        scale_y,
        out,
    );
}

fn element_to_path_layers(
    element: &SvgElement,
    state: &RenderState,
    scale_x: f32,
    scale_y: f32,
) -> Vec<PathGlyphLayer> {
    let commands = match element.name.as_str() {
        "path" => element
            .attrs
            .get("d")
            .and_then(|d| parse_path_data(d, scale_x, scale_y)),
        "rect" => rect_to_commands(&element.attrs, scale_x, scale_y),
        "circle" => circle_to_commands(&element.attrs, scale_x, scale_y),
        "ellipse" => ellipse_to_commands(&element.attrs, scale_x, scale_y),
        "line" => line_to_commands(&element.attrs, scale_x, scale_y),
        "polyline" => poly_points_to_commands(&element.attrs, false, scale_x, scale_y),
        "polygon" => poly_points_to_commands(&element.attrs, true, scale_x, scale_y),
        _ => return Vec::new(),
    };
    let Some(commands) = commands else {
        return Vec::new();
    };
    if commands.is_empty() {
        return Vec::new();
    }
    let transformed = transform_commands(&commands, state.transform);
    let bounds = command_bounds(&transformed);
    let mut layers = Vec::new();
    let supports_fill = !matches!(element.name.as_str(), "line");

    if supports_fill {
        if let Some(paint) = state.fill.clone() {
            let mut layer = PathGlyphLayer::new(
                transformed.clone(),
                resolve_gradient_paint(paint, bounds, state.transform),
            );
            layer.clip_commands = resolve_combined_clip_commands(
                state.clip_path.as_ref(),
                state.mask_path.as_ref(),
                bounds,
                state.transform,
            );
            layer.fill_rule = state.fill_rule;
            layers.push(layer);
        }
    }

    if let Some(paint) = state.stroke.clone() {
        if state.stroke_width > 0.0 {
            let mut layer = PathGlyphLayer::stroke(
                transformed,
                resolve_gradient_paint(paint, bounds, state.transform),
                state.stroke_width,
            );
            layer.clip_commands = resolve_combined_clip_commands(
                state.clip_path.as_ref(),
                state.mask_path.as_ref(),
                bounds,
                state.transform,
            );
            layers.push(layer);
        }
    }

    layers
}

fn resolve_gradient_paint(
    paint: GlyphPaint,
    bounds: Option<(f32, f32, f32, f32)>,
    element_transform: Transform2D,
) -> GlyphPaint {
    match paint {
        GlyphPaint::LinearGradient(gradient) => {
            GlyphPaint::LinearGradient(resolve_linear_gradient(gradient, bounds, element_transform))
        }
        GlyphPaint::RadialGradient(gradient) => {
            GlyphPaint::RadialGradient(resolve_radial_gradient(gradient, bounds, element_transform))
        }
        other => other,
    }
}

fn resolve_combined_clip_commands(
    clip_path: Option<&ClipPathSpec>,
    mask_path: Option<&ClipPathSpec>,
    bounds: Option<(f32, f32, f32, f32)>,
    element_transform: Transform2D,
) -> Vec<Command> {
    let mut commands = Vec::new();
    if let Some(clip_path) = clip_path {
        commands.extend(resolve_clip_spec_commands(clip_path, bounds, element_transform));
    }
    if let Some(mask_path) = mask_path {
        commands.extend(resolve_clip_spec_commands(mask_path, bounds, element_transform));
    }
    commands
}

fn resolve_clip_spec_commands(
    clip_path: &ClipPathSpec,
    bounds: Option<(f32, f32, f32, f32)>,
    element_transform: Transform2D,
) -> Vec<Command> {
    if matches!(clip_path.units, ClipPathUnits::ObjectBoundingBox) {
        let (min_x, min_y, max_x, max_y) = bounds.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let width = max_x - min_x;
        let height = max_y - min_y;
        let transform = Transform2D {
            a: width,
            b: 0.0,
            c: 0.0,
            d: height,
            e: min_x,
            f: min_y,
        };
        transform_commands(&clip_path.commands, transform)
    } else {
        transform_commands(&clip_path.commands, element_transform)
    }
}

fn resolve_linear_gradient(
    mut gradient: GlyphLinearGradient,
    bounds: Option<(f32, f32, f32, f32)>,
    element_transform: Transform2D,
) -> GlyphLinearGradient {
    let matrix = matrix_from_array(gradient.transform);
    let (x1, y1, x2, y2) = if matches!(gradient.units, GlyphGradientUnits::ObjectBoundingBox) {
        let (min_x, min_y, max_x, max_y) = bounds.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let width = max_x - min_x;
        let height = max_y - min_y;
        (
            min_x + gradient.x1 * width,
            min_y + gradient.y1 * height,
            min_x + gradient.x2 * width,
            min_y + gradient.y2 * height,
        )
    } else {
        let start = element_transform.apply(gradient.x1, gradient.y1);
        let end = element_transform.apply(gradient.x2, gradient.y2);
        (start.0, start.1, end.0, end.1)
    };
    let start = matrix.apply(x1, y1);
    let end = matrix.apply(x2, y2);
    gradient.x1 = start.0;
    gradient.y1 = start.1;
    gradient.x2 = end.0;
    gradient.y2 = end.1;
    gradient.units = GlyphGradientUnits::UserSpaceOnUse;
    gradient.transform = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    gradient
}

fn resolve_radial_gradient(
    mut gradient: GlyphRadialGradient,
    bounds: Option<(f32, f32, f32, f32)>,
    element_transform: Transform2D,
) -> GlyphRadialGradient {
    let matrix = matrix_from_array(gradient.transform);
    let (cx, cy, fx, fy, radius) = if matches!(gradient.units, GlyphGradientUnits::ObjectBoundingBox) {
        let (min_x, min_y, max_x, max_y) = bounds.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let width = max_x - min_x;
        let height = max_y - min_y;
        let scale = width.abs().max(height.abs());
        (
            min_x + gradient.cx * width,
            min_y + gradient.cy * height,
            min_x + gradient.fx * width,
            min_y + gradient.fy * height,
            gradient.r * scale,
        )
    } else {
        let center = element_transform.apply(gradient.cx, gradient.cy);
        let focus = element_transform.apply(gradient.fx, gradient.fy);
        (
            center.0,
            center.1,
            focus.0,
            focus.1,
            gradient.r * transform_max_scale(element_transform),
        )
    };
    let center = matrix.apply(cx, cy);
    let focus = matrix.apply(fx, fy);
    gradient.cx = center.0;
    gradient.cy = center.1;
    gradient.fx = focus.0;
    gradient.fy = focus.1;
    gradient.r = radius * transform_max_scale(matrix);
    gradient.units = GlyphGradientUnits::UserSpaceOnUse;
    gradient.transform = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    gradient
}

fn matrix_from_array(values: [f32; 6]) -> Transform2D {
    Transform2D {
        a: values[0],
        b: values[1],
        c: values[2],
        d: values[3],
        e: values[4],
        f: values[5],
    }
}

fn transform_max_scale(transform: Transform2D) -> f32 {
    let sx = (transform.a * transform.a + transform.b * transform.b).sqrt();
    let sy = (transform.c * transform.c + transform.d * transform.d).sqrt();
    sx.max(sy)
}

fn command_bounds(commands: &[Command]) -> Option<(f32, f32, f32, f32)> {
    let mut bounds = None;
    for command in commands {
        match command {
            Command::MoveTo(x, y) | Command::Line(x, y) => extend_command_bounds(&mut bounds, *x, *y),
            Command::Bezier((cx, cy), (x, y)) => {
                extend_command_bounds(&mut bounds, *cx, *cy);
                extend_command_bounds(&mut bounds, *x, *y);
            }
            Command::CubicBezier((x1, y1), (x2, y2), (x, y)) => {
                extend_command_bounds(&mut bounds, *x1, *y1);
                extend_command_bounds(&mut bounds, *x2, *y2);
                extend_command_bounds(&mut bounds, *x, *y);
            }
            Command::Close => {}
        }
    }
    bounds
}

fn extend_command_bounds(bounds: &mut Option<(f32, f32, f32, f32)>, x: f32, y: f32) {
    if let Some((min_x, min_y, max_x, max_y)) = bounds.as_mut() {
        *min_x = min_x.min(x);
        *min_y = min_y.min(y);
        *max_x = max_x.max(x);
        *max_y = max_y.max(y);
    } else {
        *bounds = Some((x, y, x, y));
    }
}

fn tag_name(tag: &str) -> Option<&str> {
    let start = 1usize;
    let end = tag[start..]
        .find(|ch: char| ch.is_whitespace() || ch == '>' || ch == '/')
        .map(|offset| start + offset)?;
    Some(&tag[start..end])
}

fn closing_tag_name(tag: &str) -> Option<&str> {
    let start = 2usize;
    let end = tag[start..]
        .find(|ch: char| ch.is_whitespace() || ch == '>')
        .map(|offset| start + offset)?;
    Some(&tag[start..end])
}

fn parse_attributes(tag: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let Some(mut index) = tag.find(char::is_whitespace) else {
        return attrs;
    };

    while index < tag.len() {
        while index < tag.len()
            && matches!(
                tag.as_bytes()[index],
                b' ' | b'\n' | b'\r' | b'\t' | b'/' | b'>'
            )
        {
            index += 1;
        }
        if index >= tag.len() || tag.as_bytes()[index] == b'>' {
            break;
        }

        let key_start = index;
        while index < tag.len()
            && !matches!(
                tag.as_bytes()[index],
                b'=' | b' ' | b'\n' | b'\r' | b'\t' | b'>'
            )
        {
            index += 1;
        }
        if index >= tag.len() || tag.as_bytes()[index] != b'=' {
            while index < tag.len() && tag.as_bytes()[index] != b'>' {
                if tag.as_bytes()[index] == b' ' {
                    break;
                }
                index += 1;
            }
            continue;
        }

        let key = tag[key_start..index].trim().to_ascii_lowercase();
        index += 1;
        if index >= tag.len() {
            break;
        }

        let quote = tag.as_bytes()[index];
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        index += 1;
        let value_start = index;
        while index < tag.len() && tag.as_bytes()[index] != quote {
            index += 1;
        }
        if index > value_start {
            attrs.insert(key, tag[value_start..index].to_string());
        }
        index += 1;
    }

    attrs
}

fn parse_style(style: &str) -> HashMap<String, String> {
    style
        .split(';')
        .filter_map(|entry| {
            let (key, value) = entry.split_once(':')?;
            Some((key.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

fn parse_fill(
    value: &str,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<GlyphPaint> {
    let value = value.trim();
    if let Some(paint) = parse_basic_paint(value) {
        return Some(paint);
    }
    if let Some(reference) = parse_url_reference(value) {
        return defs
            .get(reference)
            .and_then(|element| parse_gradient(element, defs, scale_x, scale_y));
    }
    None
}

fn parse_basic_paint(value: &str) -> Option<GlyphPaint> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("currentcolor") {
        return Some(GlyphPaint::CurrentColor);
    }
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex).map(GlyphPaint::Solid);
    }
    None
}

fn parse_url_reference(value: &str) -> Option<&str> {
    value
        .strip_prefix("url(")?
        .strip_suffix(')')?
        .trim()
        .strip_prefix('#')
}

fn parse_gradient(
    element: &SvgElement,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<GlyphPaint> {
    match element.name.as_str() {
        "lineargradient" => parse_linear_gradient(element, defs, scale_x, scale_y)
            .map(GlyphPaint::LinearGradient),
        "radialgradient" => parse_radial_gradient(element, defs, scale_x, scale_y)
            .map(GlyphPaint::RadialGradient),
        _ => None,
    }
}

fn parse_linear_gradient(
    element: &SvgElement,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<GlyphLinearGradient> {
    let inherited = gradient_reference(element, defs);
    let inherited_gradient = inherited.and_then(|gradient| {
        parse_linear_gradient(gradient, defs, scale_x, scale_y).or_else(|| {
            parse_radial_gradient(gradient, defs, scale_x, scale_y).map(|gradient| {
                let center_x = gradient.cx;
                let center_y = gradient.cy;
                GlyphLinearGradient {
                    x1: center_x - gradient.r,
                    y1: center_y,
                    x2: center_x + gradient.r,
                    y2: center_y,
                    units: gradient.units,
                    transform: gradient.transform,
                    spread: gradient.spread,
                    stops: gradient.stops,
                }
            })
        })
    });
    let units = gradient_units(element, inherited, inherited_gradient.as_ref().map(|g| g.units));
    let x1 = gradient_number(element, inherited, "x1").or_else(|| inherited_gradient.as_ref().map(|g| g.x1)).unwrap_or(0.0);
    let y1 = gradient_number(element, inherited, "y1").or_else(|| inherited_gradient.as_ref().map(|g| g.y1)).unwrap_or(0.0);
    let x2 = gradient_number(element, inherited, "x2").or_else(|| inherited_gradient.as_ref().map(|g| g.x2)).unwrap_or(1.0);
    let y2 = gradient_number(element, inherited, "y2").or_else(|| inherited_gradient.as_ref().map(|g| g.y2)).unwrap_or(0.0);
    let (x1, y1, x2, y2) = if matches!(units, GlyphGradientUnits::UserSpaceOnUse) {
        (x1 * scale_x, y1 * scale_y, x2 * scale_x, y2 * scale_y)
    } else {
        (x1, y1, x2, y2)
    };
    let transform = gradient_transform(
        element,
        inherited,
        inherited_gradient.as_ref().map(|g| g.transform),
        scale_x,
        scale_y,
    );
    let spread = gradient_spread(
        element,
        inherited,
        inherited_gradient.as_ref().map(|g| g.spread),
    );
    let stops = gradient_stops(
        element,
        inherited,
        inherited_gradient.as_ref().map(|g| g.stops.as_slice()),
        defs,
        scale_x,
        scale_y,
    );
    if stops.is_empty() {
        return None;
    }
    Some(GlyphLinearGradient {
        x1,
        y1,
        x2,
        y2,
        units,
        transform,
        spread,
        stops,
    })
}

fn parse_radial_gradient(
    element: &SvgElement,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<GlyphRadialGradient> {
    let inherited = gradient_reference(element, defs);
    let inherited_gradient = inherited.and_then(|gradient| parse_radial_gradient(gradient, defs, scale_x, scale_y));
    let units = gradient_units(element, inherited, inherited_gradient.as_ref().map(|g| g.units));
    let cx = gradient_number(element, inherited, "cx").or_else(|| inherited_gradient.as_ref().map(|g| g.cx)).unwrap_or(0.5);
    let cy = gradient_number(element, inherited, "cy").or_else(|| inherited_gradient.as_ref().map(|g| g.cy)).unwrap_or(0.5);
    let r = gradient_number(element, inherited, "r").or_else(|| inherited_gradient.as_ref().map(|g| g.r)).unwrap_or(0.5);
    let fx = gradient_number(element, inherited, "fx").or_else(|| inherited_gradient.as_ref().map(|g| g.fx)).unwrap_or(cx);
    let fy = gradient_number(element, inherited, "fy").or_else(|| inherited_gradient.as_ref().map(|g| g.fy)).unwrap_or(cy);
    let (cx, cy, fx, fy, r) = if matches!(units, GlyphGradientUnits::UserSpaceOnUse) {
        (
            cx * scale_x,
            cy * scale_y,
            fx * scale_x,
            fy * scale_y,
            r * scale_x.abs().max(scale_y.abs()),
        )
    } else {
        (cx, cy, fx, fy, r)
    };
    let transform = gradient_transform(
        element,
        inherited,
        inherited_gradient.as_ref().map(|g| g.transform),
        scale_x,
        scale_y,
    );
    let spread = gradient_spread(
        element,
        inherited,
        inherited_gradient.as_ref().map(|g| g.spread),
    );
    let stops = gradient_stops(
        element,
        inherited,
        inherited_gradient.as_ref().map(|g| g.stops.as_slice()),
        defs,
        scale_x,
        scale_y,
    );
    if stops.is_empty() {
        return None;
    }
    Some(GlyphRadialGradient {
        cx,
        cy,
        r,
        fx,
        fy,
        units,
        transform,
        spread,
        stops,
    })
}

fn gradient_reference<'a>(
    element: &'a SvgElement,
    defs: &'a HashMap<String, SvgElement>,
) -> Option<&'a SvgElement> {
    element
        .attrs
        .get("href")
        .or_else(|| element.attrs.get("xlink:href"))
        .and_then(|value| value.strip_prefix('#'))
        .and_then(|reference| defs.get(reference))
}

fn gradient_number(element: &SvgElement, inherited: Option<&SvgElement>, key: &str) -> Option<f32> {
    element
        .attrs
        .get(key)
        .and_then(|value| parse_number_or_percent(value))
        .or_else(|| {
            inherited.and_then(|inherited| {
                inherited
                    .attrs
                    .get(key)
                    .and_then(|value| parse_number_or_percent(value))
            })
        })
}

fn gradient_spread(
    element: &SvgElement,
    inherited: Option<&SvgElement>,
    inherited_value: Option<GlyphGradientSpread>,
) -> GlyphGradientSpread {
    let value = element
        .attrs
        .get("spreadmethod")
        .or_else(|| inherited.and_then(|gradient| gradient.attrs.get("spreadmethod")));
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "reflect" => GlyphGradientSpread::Reflect,
        Some(value) if value == "repeat" => GlyphGradientSpread::Repeat,
        _ => inherited_value.unwrap_or(GlyphGradientSpread::Pad),
    }
}

fn gradient_units(
    element: &SvgElement,
    inherited: Option<&SvgElement>,
    inherited_value: Option<GlyphGradientUnits>,
) -> GlyphGradientUnits {
    let value = element
        .attrs
        .get("gradientunits")
        .or_else(|| inherited.and_then(|gradient| gradient.attrs.get("gradientunits")));
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "userspaceonuse" => GlyphGradientUnits::UserSpaceOnUse,
        _ => inherited_value.unwrap_or(GlyphGradientUnits::ObjectBoundingBox),
    }
}

fn gradient_transform(
    element: &SvgElement,
    inherited: Option<&SvgElement>,
    inherited_value: Option<[f32; 6]>,
    scale_x: f32,
    scale_y: f32,
) -> [f32; 6] {
    let transform = element
        .attrs
        .get("gradienttransform")
        .or_else(|| inherited.and_then(|gradient| gradient.attrs.get("gradienttransform")));
    if let Some(transform) = transform {
        let transform = parse_transform(Some(transform), scale_x, scale_y);
        [
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e,
            transform.f,
        ]
    } else {
        inherited_value.unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }
}

fn gradient_stops(
    element: &SvgElement,
    inherited: Option<&SvgElement>,
    inherited_stops: Option<&[GlyphGradientStop]>,
    _defs: &HashMap<String, SvgElement>,
    _scale_x: f32,
    _scale_y: f32,
) -> Vec<GlyphGradientStop> {
    let mut stops = collect_gradient_stops(element);
    if stops.is_empty() {
        if let Some(inherited) = inherited {
            stops = collect_gradient_stops(inherited);
            if stops.is_empty() {
                stops = gradient_stops(
                    inherited,
                    gradient_reference(inherited, _defs),
                    inherited_stops,
                    _defs,
                    _scale_x,
                    _scale_y,
                );
            }
        }
    }
    if stops.is_empty() {
        inherited_stops.unwrap_or(&[]).to_vec()
    } else {
        stops
    }
}

fn collect_gradient_stops(element: &SvgElement) -> Vec<GlyphGradientStop> {
    let mut stops = Vec::new();
    for child in &element.children {
        let SvgNode::Element(stop) = child else {
            continue;
        };
        if stop.name != "stop" {
            continue;
        }
        let style = stop.attrs.get("style").map(|value| parse_style(value));
        let offset = style
            .as_ref()
            .and_then(|style| style.get("offset"))
            .or_else(|| stop.attrs.get("offset"))
            .and_then(|value| parse_number_or_percent(value))
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let color = style
            .as_ref()
            .and_then(|style| style.get("stop-color"))
            .or_else(|| stop.attrs.get("stop-color"))
            .and_then(|value| parse_basic_paint(value));
        let opacity = style
            .as_ref()
            .and_then(|style| style.get("stop-opacity"))
            .or_else(|| stop.attrs.get("stop-opacity"))
            .and_then(|value| parse_number_or_percent(value))
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let Some(color) = color else {
            continue;
        };
        let argb = match color {
            GlyphPaint::Solid(argb) => apply_alpha(argb, opacity),
            GlyphPaint::CurrentColor => apply_alpha(0xff00_0000, opacity),
            GlyphPaint::LinearGradient(_) | GlyphPaint::RadialGradient(_) => continue,
        };
        stops.push(GlyphGradientStop {
            offset,
            color: argb,
        });
    }
    stops
}

fn apply_alpha(color: u32, opacity: f32) -> u32 {
    let alpha = (((color >> 24) & 0xff) as f32 * opacity).round().clamp(0.0, 255.0) as u32;
    (color & 0x00ff_ffff) | (alpha << 24)
}

fn resolve_paint(
    style_value: Option<&String>,
    attr_value: Option<&String>,
    inherited: Option<&GlyphPaint>,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<GlyphPaint> {
    let Some(value) = style_value.or(attr_value) else {
        return inherited.cloned();
    };
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        None
    } else {
        parse_fill(value, defs, scale_x, scale_y).or_else(|| inherited.cloned())
    }
}

fn resolve_clip_path(
    style_value: Option<&String>,
    attr_value: Option<&String>,
    inherited: Option<&ClipPathSpec>,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<ClipPathSpec> {
    let Some(value) = style_value.or(attr_value) else {
        return inherited.cloned();
    };
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    let reference = parse_url_reference(value)?;
    let element = defs.get(reference)?;
    parse_clip_path_spec(element, defs, scale_x, scale_y).or_else(|| inherited.cloned())
}

fn resolve_mask_path(
    style_value: Option<&String>,
    attr_value: Option<&String>,
    inherited: Option<&ClipPathSpec>,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<ClipPathSpec> {
    let Some(value) = style_value.or(attr_value) else {
        return inherited.cloned();
    };
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    let reference = parse_url_reference(value)?;
    let element = defs.get(reference)?;
    parse_mask_spec(element, defs, scale_x, scale_y).or_else(|| inherited.cloned())
}

fn parse_clip_path_spec(
    element: &SvgElement,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<ClipPathSpec> {
    if element.name != "clippath" {
        return None;
    }
    let units = match element
        .attrs
        .get("clippathunits")
        .map(|value| value.trim().to_ascii_lowercase())
    {
        Some(value) if value == "objectboundingbox" => ClipPathUnits::ObjectBoundingBox,
        _ => ClipPathUnits::UserSpaceOnUse,
    };
    let mut commands = Vec::new();
    for child in &element.children {
        collect_clip_commands(
            child,
            defs,
            Transform2D::IDENTITY,
            scale_x,
            scale_y,
            &mut commands,
        );
    }
    if commands.is_empty() {
        None
    } else {
        Some(ClipPathSpec { commands, units })
    }
}

fn parse_mask_spec(
    element: &SvgElement,
    defs: &HashMap<String, SvgElement>,
    scale_x: f32,
    scale_y: f32,
) -> Option<ClipPathSpec> {
    if element.name != "mask" {
        return None;
    }
    let units = match element
        .attrs
        .get("maskcontentunits")
        .or_else(|| element.attrs.get("maskunits"))
        .map(|value| value.trim().to_ascii_lowercase())
    {
        Some(value) if value == "objectboundingbox" => ClipPathUnits::ObjectBoundingBox,
        _ => ClipPathUnits::UserSpaceOnUse,
    };
    let mut commands = Vec::new();
    for child in &element.children {
        collect_clip_commands(
            child,
            defs,
            Transform2D::IDENTITY,
            scale_x,
            scale_y,
            &mut commands,
        );
    }
    if commands.is_empty() {
        None
    } else {
        Some(ClipPathSpec { commands, units })
    }
}

fn collect_clip_commands(
    node: &SvgNode,
    defs: &HashMap<String, SvgElement>,
    transform: Transform2D,
    scale_x: f32,
    scale_y: f32,
    out: &mut Vec<Command>,
) {
    let SvgNode::Element(element) = node else {
        return;
    };
    let style = element.attrs.get("style").map(|value| parse_style(value));
    let next_transform = transform.multiply(parse_transform(
        style
            .as_ref()
            .and_then(|style| style.get("transform"))
            .or_else(|| element.attrs.get("transform")),
        scale_x,
        scale_y,
    ));

    match element.name.as_str() {
        "g" | "svg" | "symbol" | "clippath" => {
            for child in &element.children {
                collect_clip_commands(child, defs, next_transform, scale_x, scale_y, out);
            }
        }
        "use" => {
            let Some(reference) = element
                .attrs
                .get("href")
                .or_else(|| element.attrs.get("xlink:href"))
                .and_then(|value| value.strip_prefix('#'))
            else {
                return;
            };
            let Some(referenced) = defs.get(reference) else {
                return;
            };
            let translate = Transform2D::translate(
                parse_attr_f32(&element.attrs, "x").unwrap_or(0.0) * scale_x,
                parse_attr_f32(&element.attrs, "y").unwrap_or(0.0) * scale_y,
            );
            collect_clip_commands(
                &SvgNode::Element(referenced.clone()),
                defs,
                next_transform.multiply(translate),
                scale_x,
                scale_y,
                out,
            );
        }
        "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
            let commands = match element.name.as_str() {
                "path" => element
                    .attrs
                    .get("d")
                    .and_then(|d| parse_path_data(d, scale_x, scale_y)),
                "rect" => rect_to_commands(&element.attrs, scale_x, scale_y),
                "circle" => circle_to_commands(&element.attrs, scale_x, scale_y),
                "ellipse" => ellipse_to_commands(&element.attrs, scale_x, scale_y),
                "line" => line_to_commands(&element.attrs, scale_x, scale_y),
                "polyline" => poly_points_to_commands(&element.attrs, false, scale_x, scale_y),
                "polygon" => poly_points_to_commands(&element.attrs, true, scale_x, scale_y),
                _ => None,
            };
            if let Some(commands) = commands {
                out.extend(transform_commands(&commands, next_transform));
            }
        }
        _ => {}
    }
}

fn parse_hex_color(hex: &str) -> Option<u32> {
    match hex.len() {
        3 => {
            let mut chars = hex.chars();
            let r = chars.next()?.to_digit(16)? as u8;
            let g = chars.next()?.to_digit(16)? as u8;
            let b = chars.next()?.to_digit(16)? as u8;
            Some(
                0xff00_0000
                    | (((r << 4) | r) as u32) << 16
                    | (((g << 4) | g) as u32) << 8
                    | ((b << 4) | b) as u32,
            )
        }
        6 => u32::from_str_radix(hex, 16)
            .ok()
            .map(|rgb| 0xff00_0000 | rgb),
        8 => u32::from_str_radix(hex, 16)
            .ok()
            .map(|rgba| (rgba << 24) | (rgba >> 8)),
        _ => None,
    }
}

fn parse_number_or_percent(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    if let Some(percent) = trimmed.strip_suffix('%') {
        return percent.trim().parse::<f32>().ok().map(|value| value / 100.0);
    }
    parse_number(trimmed)
}

fn parse_fill_rule(value: &str) -> FillRule {
    if value.trim().eq_ignore_ascii_case("evenodd") {
        FillRule::EvenOdd
    } else {
        FillRule::NonZero
    }
}

fn parse_transform(transform: Option<&String>, scale_x: f32, scale_y: f32) -> Transform2D {
    let Some(transform) = transform else {
        return Transform2D::IDENTITY;
    };

    let mut current = Transform2D::IDENTITY;
    let mut source = transform.as_str();
    while let Some(open) = source.find('(') {
        let name = source[..open].trim().to_ascii_lowercase();
        let Some(close) = source[open + 1..].find(')') else {
            break;
        };
        let values = parse_numbers(&source[open + 1..open + 1 + close]);
        let next = match name.as_str() {
            "translate" if !values.is_empty() => Transform2D::translate(
                values[0] * scale_x,
                values.get(1).copied().unwrap_or(0.0) * scale_y,
            ),
            "scale" if !values.is_empty() => {
                Transform2D::scale(values[0], values.get(1).copied().unwrap_or(values[0]))
            }
            "rotate" if !values.is_empty() => {
                let angle = values[0].to_radians();
                if values.len() >= 3 {
                    let cx = values[1] * scale_x;
                    let cy = values[2] * scale_y;
                    Transform2D::translate(cx, cy)
                        .multiply(Transform2D::rotate_radians(angle))
                        .multiply(Transform2D::translate(-cx, -cy))
                } else {
                    Transform2D::rotate_radians(angle)
                }
            }
            "skewx" if !values.is_empty() => Transform2D::skew_x_radians(values[0].to_radians()),
            "skewy" if !values.is_empty() => Transform2D::skew_y_radians(values[0].to_radians()),
            "matrix" if values.len() == 6 => Transform2D {
                a: values[0],
                b: values[1],
                c: values[2],
                d: values[3],
                e: values[4] * scale_x,
                f: values[5] * scale_y,
            },
            _ => Transform2D::IDENTITY,
        };
        current = current.multiply(next);
        source = &source[open + 1 + close + 1..];
    }

    current
}

fn transform_commands(commands: &[Command], transform: Transform2D) -> Vec<Command> {
    commands
        .iter()
        .map(|command| match command {
            Command::MoveTo(x, y) => {
                let (x, y) = transform.apply(*x, *y);
                Command::MoveTo(x, y)
            }
            Command::Line(x, y) => {
                let (x, y) = transform.apply(*x, *y);
                Command::Line(x, y)
            }
            Command::Bezier((cx, cy), (x, y)) => {
                let (cx, cy) = transform.apply(*cx, *cy);
                let (x, y) = transform.apply(*x, *y);
                Command::Bezier((cx, cy), (x, y))
            }
            Command::CubicBezier((x1, y1), (x2, y2), (x, y)) => {
                let (x1, y1) = transform.apply(*x1, *y1);
                let (x2, y2) = transform.apply(*x2, *y2);
                let (x, y) = transform.apply(*x, *y);
                Command::CubicBezier((x1, y1), (x2, y2), (x, y))
            }
            Command::Close => Command::Close,
        })
        .collect()
}

fn rect_to_commands(
    attrs: &HashMap<String, String>,
    scale_x: f32,
    scale_y: f32,
) -> Option<Vec<Command>> {
    let x = parse_attr_f32(attrs, "x").unwrap_or(0.0) * scale_x;
    let y = parse_attr_f32(attrs, "y").unwrap_or(0.0) * scale_y;
    let width = parse_attr_f32(attrs, "width")? * scale_x;
    let height = parse_attr_f32(attrs, "height")? * scale_y;
    Some(vec![
        Command::MoveTo(x, y),
        Command::Line(x + width, y),
        Command::Line(x + width, y + height),
        Command::Line(x, y + height),
        Command::Close,
    ])
}

fn line_to_commands(
    attrs: &HashMap<String, String>,
    scale_x: f32,
    scale_y: f32,
) -> Option<Vec<Command>> {
    Some(vec![
        Command::MoveTo(
            parse_attr_f32(attrs, "x1")? * scale_x,
            parse_attr_f32(attrs, "y1")? * scale_y,
        ),
        Command::Line(
            parse_attr_f32(attrs, "x2")? * scale_x,
            parse_attr_f32(attrs, "y2")? * scale_y,
        ),
    ])
}

fn circle_to_commands(
    attrs: &HashMap<String, String>,
    scale_x: f32,
    scale_y: f32,
) -> Option<Vec<Command>> {
    let cx = parse_attr_f32(attrs, "cx")? * scale_x;
    let cy = parse_attr_f32(attrs, "cy")? * scale_y;
    let r = parse_attr_f32(attrs, "r")?;
    ellipse_commands(cx, cy, r * scale_x, r * scale_y)
}

fn ellipse_to_commands(
    attrs: &HashMap<String, String>,
    scale_x: f32,
    scale_y: f32,
) -> Option<Vec<Command>> {
    ellipse_commands(
        parse_attr_f32(attrs, "cx")? * scale_x,
        parse_attr_f32(attrs, "cy")? * scale_y,
        parse_attr_f32(attrs, "rx")? * scale_x,
        parse_attr_f32(attrs, "ry")? * scale_y,
    )
}

fn ellipse_commands(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<Vec<Command>> {
    if rx == 0.0 || ry == 0.0 {
        return None;
    }
    let kappa = 0.552_284_8f32;
    Some(vec![
        Command::MoveTo(cx + rx, cy),
        Command::CubicBezier(
            (cx + rx, cy + ry * kappa),
            (cx + rx * kappa, cy + ry),
            (cx, cy + ry),
        ),
        Command::CubicBezier(
            (cx - rx * kappa, cy + ry),
            (cx - rx, cy + ry * kappa),
            (cx - rx, cy),
        ),
        Command::CubicBezier(
            (cx - rx, cy - ry * kappa),
            (cx - rx * kappa, cy - ry),
            (cx, cy - ry),
        ),
        Command::CubicBezier(
            (cx + rx * kappa, cy - ry),
            (cx + rx, cy - ry * kappa),
            (cx + rx, cy),
        ),
        Command::Close,
    ])
}

fn poly_points_to_commands(
    attrs: &HashMap<String, String>,
    close: bool,
    scale_x: f32,
    scale_y: f32,
) -> Option<Vec<Command>> {
    let points = attrs.get("points")?;
    let numbers = parse_numbers(points);
    if numbers.len() < 4 || numbers.len() % 2 != 0 {
        return None;
    }
    let mut commands = Vec::new();
    commands.push(Command::MoveTo(numbers[0] * scale_x, numbers[1] * scale_y));
    let mut index = 2usize;
    while index + 1 < numbers.len() {
        commands.push(Command::Line(
            numbers[index] * scale_x,
            numbers[index + 1] * scale_y,
        ));
        index += 2;
    }
    if close {
        commands.push(Command::Close);
    }
    Some(commands)
}

fn parse_attr_f32(attrs: &HashMap<String, String>, key: &str) -> Option<f32> {
    attrs.get(key).and_then(|value| parse_number(value))
}

fn parse_number(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let end = trimmed
        .find(|ch: char| !(ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(trimmed.len());
    trimmed[..end].parse::<f32>().ok()
}

fn parse_numbers(source: &str) -> Vec<f32> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    let mut prev = '\0';

    for ch in source.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | 'e' | 'E') {
            current.push(ch);
        } else if matches!(ch, '-' | '+') {
            if !current.is_empty() && prev != 'e' && prev != 'E' {
                if let Ok(value) = current.parse::<f32>() {
                    numbers.push(value);
                }
                current.clear();
            }
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(value) = current.parse::<f32>() {
                numbers.push(value);
            }
            current.clear();
        }
        prev = ch;
    }

    if !current.is_empty() {
        if let Ok(value) = current.parse::<f32>() {
            numbers.push(value);
        }
    }

    numbers
}

fn parse_path_data(d: &str, scale_x: f32, scale_y: f32) -> Option<Vec<Command>> {
    let tokens = tokenize_path_data(d);
    let mut index = 0usize;
    let mut commands = Vec::new();
    let mut current = (0.0f32, 0.0f32);
    let mut subpath_start = current;
    let mut last_command = 'M';

    while index < tokens.len() {
        let command = if let PathToken::Command(command) = tokens[index] {
            index += 1;
            last_command = command;
            command
        } else {
            last_command
        };

        match command {
            'M' | 'm' => {
                let absolute = command == 'M';
                let mut first = true;
                while let Some((x, y)) = read_pair(&tokens, &mut index) {
                    let target = if absolute {
                        (x * scale_x, y * scale_y)
                    } else {
                        (current.0 + x * scale_x, current.1 + y * scale_y)
                    };
                    if first {
                        commands.push(Command::MoveTo(target.0, target.1));
                        subpath_start = target;
                        first = false;
                    } else {
                        commands.push(Command::Line(target.0, target.1));
                    }
                    current = target;
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'L' | 'l' => {
                let absolute = command == 'L';
                while let Some((x, y)) = read_pair(&tokens, &mut index) {
                    current = if absolute {
                        (x * scale_x, y * scale_y)
                    } else {
                        (current.0 + x * scale_x, current.1 + y * scale_y)
                    };
                    commands.push(Command::Line(current.0, current.1));
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'H' | 'h' => {
                let absolute = command == 'H';
                while let Some(x) = read_number(&tokens, &mut index) {
                    current.0 = if absolute {
                        x * scale_x
                    } else {
                        current.0 + x * scale_x
                    };
                    commands.push(Command::Line(current.0, current.1));
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'V' | 'v' => {
                let absolute = command == 'V';
                while let Some(y) = read_number(&tokens, &mut index) {
                    current.1 = if absolute {
                        y * scale_y
                    } else {
                        current.1 + y * scale_y
                    };
                    commands.push(Command::Line(current.0, current.1));
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'Q' | 'q' => {
                let absolute = command == 'Q';
                while let Some((cx, cy)) = read_pair(&tokens, &mut index) {
                    let (x, y) = read_pair(&tokens, &mut index)?;
                    let control = if absolute {
                        (cx * scale_x, cy * scale_y)
                    } else {
                        (current.0 + cx * scale_x, current.1 + cy * scale_y)
                    };
                    let target = if absolute {
                        (x * scale_x, y * scale_y)
                    } else {
                        (current.0 + x * scale_x, current.1 + y * scale_y)
                    };
                    commands.push(Command::Bezier(control, target));
                    current = target;
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'C' | 'c' => {
                let absolute = command == 'C';
                while let Some((x1, y1)) = read_pair(&tokens, &mut index) {
                    let (x2, y2) = read_pair(&tokens, &mut index)?;
                    let (x, y) = read_pair(&tokens, &mut index)?;
                    let control1 = if absolute {
                        (x1 * scale_x, y1 * scale_y)
                    } else {
                        (current.0 + x1 * scale_x, current.1 + y1 * scale_y)
                    };
                    let control2 = if absolute {
                        (x2 * scale_x, y2 * scale_y)
                    } else {
                        (current.0 + x2 * scale_x, current.1 + y2 * scale_y)
                    };
                    let target = if absolute {
                        (x * scale_x, y * scale_y)
                    } else {
                        (current.0 + x * scale_x, current.1 + y * scale_y)
                    };
                    commands.push(Command::CubicBezier(control1, control2, target));
                    current = target;
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'A' | 'a' => {
                let absolute = command == 'A';
                while let Some(rx) = read_number(&tokens, &mut index) {
                    let ry = read_number(&tokens, &mut index)?;
                    let axis_rotation = read_number(&tokens, &mut index)?;
                    let large_arc = read_number(&tokens, &mut index)? != 0.0;
                    let sweep = read_number(&tokens, &mut index)? != 0.0;
                    let (x, y) = read_pair(&tokens, &mut index)?;
                    let target = if absolute {
                        (x * scale_x, y * scale_y)
                    } else {
                        (current.0 + x * scale_x, current.1 + y * scale_y)
                    };
                    commands.extend(arc_to_cubic_beziers(
                        current,
                        target,
                        rx.abs() * scale_x.abs(),
                        ry.abs() * scale_y.abs(),
                        axis_rotation,
                        large_arc,
                        sweep,
                    ));
                    current = target;
                    if index < tokens.len() && matches!(tokens[index], PathToken::Command(_)) {
                        break;
                    }
                }
            }
            'Z' | 'z' => {
                commands.push(Command::Close);
                current = subpath_start;
            }
            _ => return None,
        }
    }

    Some(commands)
}

fn arc_to_cubic_beziers(
    start: (f32, f32),
    end: (f32, f32),
    mut rx: f32,
    mut ry: f32,
    x_axis_rotation_degrees: f32,
    large_arc: bool,
    sweep: bool,
) -> Vec<Command> {
    if nearly_equal_points(start, end) {
        return Vec::new();
    }
    if rx <= f32::EPSILON || ry <= f32::EPSILON {
        return vec![Command::Line(end.0, end.1)];
    }

    let phi = x_axis_rotation_degrees.to_radians();
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();
    let dx = (start.0 - end.0) * 0.5;
    let dy = (start.1 - end.1) * 0.5;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    let mut rx_sq = rx * rx;
    let mut ry_sq = ry * ry;
    let x1p_sq = x1p * x1p;
    let y1p_sq = y1p * y1p;

    let radius_scale = x1p_sq / rx_sq + y1p_sq / ry_sq;
    if radius_scale > 1.0 {
        let scale = radius_scale.sqrt();
        rx *= scale;
        ry *= scale;
        rx_sq = rx * rx;
        ry_sq = ry * ry;
    }

    let numerator = (rx_sq * ry_sq) - (rx_sq * y1p_sq) - (ry_sq * x1p_sq);
    let denominator = (rx_sq * y1p_sq) + (ry_sq * x1p_sq);
    let factor = if denominator <= f32::EPSILON {
        0.0
    } else {
        let sign = if large_arc == sweep { -1.0 } else { 1.0 };
        sign * (numerator / denominator).max(0.0).sqrt()
    };

    let cxp = factor * (rx * y1p / ry);
    let cyp = factor * (-ry * x1p / rx);
    let center = (
        cos_phi * cxp - sin_phi * cyp + (start.0 + end.0) * 0.5,
        sin_phi * cxp + cos_phi * cyp + (start.1 + end.1) * 0.5,
    );

    let start_vector = ((x1p - cxp) / rx, (y1p - cyp) / ry);
    let end_vector = ((-x1p - cxp) / rx, (-y1p - cyp) / ry);
    let start_angle = start_vector.1.atan2(start_vector.0);
    let mut delta_angle = vector_angle(start_vector, end_vector);
    if !sweep && delta_angle > 0.0 {
        delta_angle -= std::f32::consts::TAU;
    } else if sweep && delta_angle < 0.0 {
        delta_angle += std::f32::consts::TAU;
    }

    let segments = (delta_angle.abs() / (std::f32::consts::FRAC_PI_2)).ceil() as usize;
    let segments = segments.max(1);
    let step = delta_angle / segments as f32;
    let mut commands = Vec::with_capacity(segments);

    for index in 0..segments {
        let theta1 = start_angle + step * index as f32;
        let theta2 = theta1 + step;
        let alpha = (4.0 / 3.0) * ((theta2 - theta1) * 0.25).tan();
        let (sin1, cos1) = theta1.sin_cos();
        let (sin2, cos2) = theta2.sin_cos();
        let control1 = map_arc_point(
            center,
            rx,
            ry,
            cos_phi,
            sin_phi,
            cos1 - alpha * sin1,
            sin1 + alpha * cos1,
        );
        let control2 = map_arc_point(
            center,
            rx,
            ry,
            cos_phi,
            sin_phi,
            cos2 + alpha * sin2,
            sin2 - alpha * cos2,
        );
        let target = map_arc_point(center, rx, ry, cos_phi, sin_phi, cos2, sin2);
        commands.push(Command::CubicBezier(control1, control2, target));
    }

    commands
}

fn map_arc_point(
    center: (f32, f32),
    rx: f32,
    ry: f32,
    cos_phi: f32,
    sin_phi: f32,
    x: f32,
    y: f32,
) -> (f32, f32) {
    (
        center.0 + cos_phi * rx * x - sin_phi * ry * y,
        center.1 + sin_phi * rx * x + cos_phi * ry * y,
    )
}

fn vector_angle(start: (f32, f32), end: (f32, f32)) -> f32 {
    let cross = start.0 * end.1 - start.1 * end.0;
    let dot = start.0 * end.0 + start.1 * end.1;
    cross.atan2(dot)
}

fn nearly_equal_points(left: (f32, f32), right: (f32, f32)) -> bool {
    (left.0 - right.0).abs() <= 0.001 && (left.1 - right.1).abs() <= 0.001
}

#[derive(Clone, Copy)]
enum PathToken {
    Command(char),
    Number(f32),
}

fn tokenize_path_data(d: &str) -> Vec<PathToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut prev = '\0';

    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            flush_path_number(&mut current, &mut tokens);
            tokens.push(PathToken::Command(ch));
        } else if ch.is_ascii_digit() || matches!(ch, '.' | 'e' | 'E') {
            current.push(ch);
        } else if matches!(ch, '-' | '+') {
            if !current.is_empty() && prev != 'e' && prev != 'E' {
                flush_path_number(&mut current, &mut tokens);
            }
            current.push(ch);
        } else {
            flush_path_number(&mut current, &mut tokens);
        }
        prev = ch;
    }
    flush_path_number(&mut current, &mut tokens);
    tokens
}

fn flush_path_number(current: &mut String, tokens: &mut Vec<PathToken>) {
    if current.is_empty() {
        return;
    }
    if let Ok(value) = current.parse::<f32>() {
        tokens.push(PathToken::Number(value));
    }
    current.clear();
}

fn read_number(tokens: &[PathToken], index: &mut usize) -> Option<f32> {
    match tokens.get(*index)? {
        PathToken::Number(value) => {
            *index += 1;
            Some(*value)
        }
        PathToken::Command(_) => None,
    }
}

fn read_pair(tokens: &[PathToken], index: &mut usize) -> Option<(f32, f32)> {
    Some((read_number(tokens, index)?, read_number(tokens, index)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_svg_document_builds_nested_nodes() {
        let root = parse_svg_document("<svg><g id=\"a\"><path d=\"M0 0\"/></g></svg>").unwrap();
        let SvgNode::Element(document) = root else {
            panic!("expected document root");
        };
        assert_eq!(document.children.len(), 1);
        let SvgNode::Element(svg) = &document.children[0] else {
            panic!("expected svg element");
        };
        assert_eq!(svg.name, "svg");
        assert_eq!(svg.children.len(), 1);
    }

    #[test]
    fn svg_to_path_layers_parses_path_rect_and_fill_rule() {
        let document = concat!(
            "<svg>",
            "<path d=\"M0 0 L10 0 L10 10 Z\" fill=\"#ff00aa\" fill-rule=\"evenodd\"/>",
            "<rect x=\"2\" y=\"3\" width=\"4\" height=\"5\" style=\"fill:#00ff00\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 2);
        assert!(matches!(layers[0].fill_rule, FillRule::EvenOdd));
        assert!(matches!(layers[0].paint, GlyphPaint::Solid(_)));
        assert!(matches!(layers[0].commands[0], Command::MoveTo(0.0, 0.0)));
        assert!(matches!(layers[1].commands[0], Command::MoveTo(2.0, 3.0)));
    }

    #[test]
    fn svg_to_path_layers_parses_circle_and_polyline() {
        let document = concat!(
            "<svg>",
            "<circle cx=\"10\" cy=\"20\" r=\"5\" fill=\"currentColor\"/>",
            "<polyline points=\"0,0 4,4 8,0\" fill=\"#000\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);
        assert_eq!(layers.len(), 2);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(_, _)));
        assert!(matches!(layers[0].paint, GlyphPaint::CurrentColor));
        assert!(matches!(layers[1].commands[1], Command::Line(8.0, 12.0)));
    }

    #[test]
    fn svg_to_path_layers_expands_defs_and_use_with_offsets() {
        let document = concat!(
            "<svg>",
            "<defs>",
            "<g id=\"shape\"><path d=\"M0 0 L4 0 L4 4 Z\" fill=\"#123456\"/></g>",
            "</defs>",
            "<use href=\"#shape\" x=\"10\" y=\"20\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].paint, GlyphPaint::Solid(_)));
        assert!(matches!(layers[0].commands[0], Command::MoveTo(20.0, 60.0)));
        assert!(matches!(layers[0].commands[1], Command::Line(28.0, 60.0)));
    }

    #[test]
    fn svg_to_path_layers_allows_use_to_override_fill() {
        let document = concat!(
            "<svg>",
            "<defs><path id=\"dot\" d=\"M1 1 L2 1\" fill=\"#123456\"/></defs>",
            "<use xlink:href=\"#dot\" fill=\"currentColor\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].paint, GlyphPaint::CurrentColor));
    }

    #[test]
    fn svg_to_path_layers_applies_parent_translate_transform() {
        let document = concat!(
            "<svg>",
            "<g transform=\"translate(10,20)\">",
            "<path d=\"M1 2 L3 4\"/>",
            "</g>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(22.0, 66.0)));
        assert!(matches!(layers[0].commands[1], Command::Line(26.0, 72.0)));
    }

    #[test]
    fn svg_to_path_layers_applies_element_scale_transform() {
        let document = "<svg><path d=\"M1 2 L3 4\" transform=\"scale(2,3)\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(2.0, 6.0)));
        assert!(matches!(layers[0].commands[1], Command::Line(6.0, 12.0)));
    }

    #[test]
    fn svg_to_path_layers_applies_rotate_transform() {
        let document = "<svg><path d=\"M1 0 L3 0\" transform=\"rotate(90)\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(x, y) if x.abs() < 0.01 && (y - 1.0).abs() < 0.01));
        assert!(matches!(layers[0].commands[1], Command::Line(x, y) if x.abs() < 0.01 && (y - 3.0).abs() < 0.01));
    }

    #[test]
    fn svg_to_path_layers_applies_rotate_about_center_transform() {
        let document = "<svg><path d=\"M2 1 L4 1\" transform=\"rotate(90 2 1)\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(x, y) if (x - 2.0).abs() < 0.01 && (y - 1.0).abs() < 0.01));
        assert!(matches!(layers[0].commands[1], Command::Line(x, y) if (x - 2.0).abs() < 0.01 && (y - 3.0).abs() < 0.01));
    }

    #[test]
    fn svg_to_path_layers_applies_skewx_transform() {
        let document = "<svg><path d=\"M0 1 L0 3\" transform=\"skewX(45)\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(x, y) if (x - 1.0).abs() < 0.01 && (y - 1.0).abs() < 0.01));
        assert!(matches!(layers[0].commands[1], Command::Line(x, y) if (x - 3.0).abs() < 0.01 && (y - 3.0).abs() < 0.01));
    }

    #[test]
    fn svg_to_path_layers_applies_skewy_transform() {
        let document = "<svg><path d=\"M1 0 L3 0\" transform=\"skewY(45)\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(x, y) if (x - 1.0).abs() < 0.01 && (y - 1.0).abs() < 0.01));
        assert!(matches!(layers[0].commands[1], Command::Line(x, y) if (x - 3.0).abs() < 0.01 && (y - 3.0).abs() < 0.01));
    }

    #[test]
    fn svg_to_path_layers_applies_use_and_transform_together() {
        let document = concat!(
            "<svg>",
            "<defs><path id=\"dot\" d=\"M1 1 L2 1\"/></defs>",
            "<g transform=\"translate(5,6)\"><use href=\"#dot\" x=\"7\" y=\"8\"/></g>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(26.0, 45.0)));
        assert!(matches!(layers[0].commands[1], Command::Line(28.0, 45.0)));
    }

    #[test]
    fn svg_to_path_layers_emits_fill_and_stroke_layers() {
        let document = concat!(
            "<svg>",
            "<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#123456\" stroke=\"#abcdef\" stroke-width=\"2\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);

        assert_eq!(layers.len(), 2);
        assert!(matches!(layers[0].paint, GlyphPaint::Solid(_)));
        assert!(matches!(layers[1].paint, GlyphPaint::Solid(_)));
        assert_eq!(layers[1].stroke_width, 6.0);
    }

    #[test]
    fn svg_to_path_layers_keeps_line_as_stroke_only() {
        let document = "<svg><line x1=\"1\" y1=\"2\" x2=\"3\" y2=\"4\" stroke=\"#123456\" stroke-width=\"2\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);

        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].paint_mode, crate::commands::PathPaintMode::Stroke));
        assert_eq!(layers[0].stroke_width, 2.0);
    }

    #[test]
    fn svg_to_path_layers_resolves_linear_gradient_fill() {
        let document = concat!(
            "<svg>",
            "<defs>",
            "<linearGradient id=\"grad\"><stop offset=\"0%\" stop-color=\"#112233\"/><stop offset=\"100%\" stop-color=\"#445566\" stop-opacity=\"0.5\"/></linearGradient>",
            "</defs>",
            "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"url(#grad)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);

        assert_eq!(layers.len(), 1);
        match &layers[0].paint {
            GlyphPaint::LinearGradient(gradient) => {
                assert_eq!(gradient.units, GlyphGradientUnits::UserSpaceOnUse);
                assert_eq!(gradient.x1, 0.0);
                assert_eq!(gradient.y1, 0.0);
                assert_eq!(gradient.x2, 20.0);
                assert_eq!(gradient.y2, 0.0);
                assert_eq!(gradient.stops.len(), 2);
            }
            other => panic!("expected linear gradient paint, got {other:?}"),
        }
    }

    #[test]
    fn svg_to_path_layers_resolves_radial_gradient_stroke() {
        let document = concat!(
            "<svg>",
            "<defs>",
            "<radialGradient id=\"grad\" cx=\"50%\" cy=\"50%\" r=\"25%\"><stop offset=\"0\" stop-color=\"#abcdef\"/></radialGradient>",
            "</defs>",
            "<circle cx=\"5\" cy=\"6\" r=\"4\" stroke=\"url(#grad)\" stroke-width=\"2\" fill=\"none\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);

        assert_eq!(layers.len(), 1);
        match &layers[0].paint {
            GlyphPaint::RadialGradient(gradient) => {
                assert_eq!(gradient.units, GlyphGradientUnits::UserSpaceOnUse);
                assert_eq!(gradient.cx, 10.0);
                assert_eq!(gradient.cy, 18.0);
                assert_eq!(gradient.r, 6.0);
            }
            other => panic!("expected radial gradient paint, got {other:?}"),
        }
    }

    #[test]
    fn svg_to_path_layers_keeps_gradient_units_and_transform() {
        let document = concat!(
            "<svg>",
            "<defs>",
            "<linearGradient id=\"grad\" gradientUnits=\"userSpaceOnUse\" gradientTransform=\"translate(3,4) scale(2,3)\">",
            "<stop offset=\"0%\" stop-color=\"#112233\"/>",
            "<stop offset=\"100%\" stop-color=\"#445566\"/>",
            "</linearGradient>",
            "</defs>",
            "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"url(#grad)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);

        match &layers[0].paint {
            GlyphPaint::LinearGradient(gradient) => {
                assert_eq!(gradient.units, GlyphGradientUnits::UserSpaceOnUse);
                assert_eq!(gradient.transform, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
                assert_eq!(gradient.x1, 6.0);
                assert_eq!(gradient.y1, 12.0);
            }
            other => panic!("expected linear gradient paint, got {other:?}"),
        }
    }

    #[test]
    fn svg_to_path_layers_inherits_gradient_stops_via_href() {
        let document = concat!(
            "<svg>",
            "<defs>",
            "<linearGradient id=\"base\"><stop offset=\"0%\" stop-color=\"#112233\"/><stop offset=\"100%\" stop-color=\"#445566\"/></linearGradient>",
            "<linearGradient id=\"derived\" href=\"#base\" x1=\"25%\" x2=\"75%\"/>",
            "</defs>",
            "<rect x=\"0\" y=\"0\" width=\"20\" height=\"10\" fill=\"url(#derived)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 1.0, 1.0);

        match &layers[0].paint {
            GlyphPaint::LinearGradient(gradient) => {
                assert_eq!(gradient.stops.len(), 2);
                assert_eq!(gradient.x1, 5.0);
                assert_eq!(gradient.x2, 15.0);
            }
            other => panic!("expected inherited linear gradient paint, got {other:?}"),
        }
    }

    #[test]
    fn svg_to_path_layers_parses_absolute_arc_path() {
        let document = "<svg><path d=\"M10 10 A10 10 0 0 1 20 20\" fill=\"#000\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].commands[0], Command::MoveTo(10.0, 10.0)));
        assert!(
            layers[0]
                .commands
                .iter()
                .any(|command| matches!(command, Command::CubicBezier(_, _, _))),
            "expected arc to expand into cubic segments"
        );
    }

    #[test]
    fn svg_to_path_layers_parses_relative_arc_path() {
        let document = "<svg><path d=\"M10 10 a10 10 0 0 1 10 10\" fill=\"#000\"/></svg>";
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(
            matches!(layers[0].commands.last(), Some(Command::CubicBezier(_, _, (x, y))) if (*x - 20.0).abs() < 0.01 && (*y - 20.0).abs() < 0.01),
            "expected relative arc to end at the translated target"
        );
    }

    #[test]
    fn svg_requires_svg_fallback_detects_mask_pattern_and_filter() {
        assert!(svg_requires_svg_fallback("<svg><mask id=\"m\"/></svg>"));
        assert!(svg_requires_svg_fallback("<svg><pattern id=\"p\"/></svg>"));
        assert!(svg_requires_svg_fallback("<svg><filter id=\"f\"/></svg>"));
        assert!(!svg_requires_svg_fallback("<svg><path d=\"M0 0 L1 1\"/></svg>"));
    }

    #[test]
    fn svg_to_path_layers_resolves_simple_clip_path() {
        let document = concat!(
            "<svg>",
            "<defs><clipPath id=\"clip\"><rect x=\"1\" y=\"2\" width=\"3\" height=\"4\"/></clipPath></defs>",
            "<rect x=\"10\" y=\"20\" width=\"30\" height=\"40\" clip-path=\"url(#clip)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);

        assert_eq!(layers.len(), 1);
        assert!(!layers[0].clip_commands.is_empty());
        assert!(matches!(layers[0].clip_commands[0], Command::MoveTo(2.0, 6.0)));
    }

    #[test]
    fn svg_to_path_layers_resolves_clip_path_use_reference() {
        let document = concat!(
            "<svg>",
            "<defs>",
            "<path id=\"shape\" d=\"M0 0 L1 0 L1 1 Z\"/>",
            "<clipPath id=\"clip\"><use href=\"#shape\" x=\"2\" y=\"3\"/></clipPath>",
            "</defs>",
            "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" clip-path=\"url(#clip)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 2.0, 3.0);

        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].clip_commands[0], Command::MoveTo(4.0, 9.0)));
    }

    #[test]
    fn svg_to_path_layers_resolves_simple_mask_as_clip_commands() {
        let document = concat!(
            "<svg>",
            "<defs><mask id=\"m\"><rect x=\"2\" y=\"3\" width=\"4\" height=\"5\"/></mask></defs>",
            "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"#123456\" mask=\"url(#m)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(!layers[0].clip_commands.is_empty());
        assert!(matches!(layers[0].clip_commands[0], Command::MoveTo(2.0, 3.0)));
    }

    #[test]
    fn svg_to_path_layers_resolves_object_bounding_box_mask_as_clip_commands() {
        let document = concat!(
            "<svg>",
            "<defs><mask id=\"m\" maskUnits=\"objectBoundingBox\"><rect x=\"0.25\" y=\"0.5\" width=\"0.5\" height=\"0.5\"/></mask></defs>",
            "<rect x=\"10\" y=\"20\" width=\"8\" height=\"6\" fill=\"#123456\" mask=\"url(#m)\"/>",
            "</svg>"
        );
        let layers = svg_to_path_layers(document, 1.0, 1.0);
        assert_eq!(layers.len(), 1);
        assert!(matches!(layers[0].clip_commands[0], Command::MoveTo(12.0, 23.0)));
    }
}
