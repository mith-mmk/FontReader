use crate::commands::{Command, FillRule, GlyphPaint, PathGlyphLayer};
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

#[derive(Debug, Clone, Copy)]
struct RenderState {
    offset_x: f32,
    offset_y: f32,
    fill: Option<GlyphPaint>,
    fill_rule: FillRule,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            fill: None,
            fill_rule: FillRule::NonZero,
        }
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
    flatten_node(
        &root,
        &defs,
        RenderState::default(),
        scale_x,
        scale_y,
        &mut layers,
    );
    layers
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
            if element.name != name {
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
            name: name.to_string(),
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
    state: RenderState,
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
    let local_fill = style
        .as_ref()
        .and_then(|style| style.get("fill"))
        .or_else(|| element.attrs.get("fill"))
        .and_then(|value| parse_fill(value));
    let local_fill_rule = style
        .as_ref()
        .and_then(|style| style.get("fill-rule"))
        .or_else(|| element.attrs.get("fill-rule"))
        .map(|value| parse_fill_rule(value))
        .unwrap_or(state.fill_rule);
    let next_state = RenderState {
        offset_x: state.offset_x,
        offset_y: state.offset_y,
        fill: local_fill.or(state.fill),
        fill_rule: local_fill_rule,
    };

    match element.name.as_str() {
        "svg" | "g" | "symbol" => {
            for child in &element.children {
                flatten_node(child, defs, next_state, scale_x, scale_y, out);
            }
        }
        "use" => flatten_use(element, defs, next_state, scale_x, scale_y, out),
        "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
            if let Some(layer) = element_to_path_layer(element, next_state, scale_x, scale_y) {
                out.push(layer);
            }
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

    let use_state = RenderState {
        offset_x: state.offset_x + parse_attr_f32(&element.attrs, "x").unwrap_or(0.0) * scale_x,
        offset_y: state.offset_y + parse_attr_f32(&element.attrs, "y").unwrap_or(0.0) * scale_y,
        fill: state.fill,
        fill_rule: state.fill_rule,
    };
    flatten_node(
        &SvgNode::Element(referenced.clone()),
        defs,
        use_state,
        scale_x,
        scale_y,
        out,
    );
}

fn element_to_path_layer(
    element: &SvgElement,
    state: RenderState,
    scale_x: f32,
    scale_y: f32,
) -> Option<PathGlyphLayer> {
    let commands = match element.name.as_str() {
        "path" => element
            .attrs
            .get("d")
            .and_then(|d| parse_path_data(d, scale_x, scale_y))?,
        "rect" => rect_to_commands(&element.attrs, scale_x, scale_y)?,
        "circle" => circle_to_commands(&element.attrs, scale_x, scale_y)?,
        "ellipse" => ellipse_to_commands(&element.attrs, scale_x, scale_y)?,
        "line" => line_to_commands(&element.attrs, scale_x, scale_y)?,
        "polyline" => poly_points_to_commands(&element.attrs, false, scale_x, scale_y)?,
        "polygon" => poly_points_to_commands(&element.attrs, true, scale_x, scale_y)?,
        _ => return None,
    };
    if commands.is_empty() {
        return None;
    }
    let mut layer = PathGlyphLayer::new(commands, state.fill.unwrap_or(GlyphPaint::CurrentColor));
    layer.fill_rule = state.fill_rule;
    layer.offset_x = state.offset_x;
    layer.offset_y = state.offset_y;
    Some(layer)
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

fn parse_fill(value: &str) -> Option<GlyphPaint> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    if value.eq_ignore_ascii_case("currentcolor") {
        return Some(GlyphPaint::CurrentColor);
    }
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex).map(GlyphPaint::Solid);
    }
    None
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

fn parse_fill_rule(value: &str) -> FillRule {
    if value.trim().eq_ignore_ascii_case("evenodd") {
        FillRule::EvenOdd
    } else {
        FillRule::NonZero
    }
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
            'Z' | 'z' => {
                commands.push(Command::Close);
                current = subpath_start;
            }
            _ => return None,
        }
    }

    Some(commands)
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
        assert_eq!(layers[0].offset_x, 20.0);
        assert_eq!(layers[0].offset_y, 60.0);
        assert!(matches!(layers[0].commands[1], Command::Line(8.0, 0.0)));
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
}
