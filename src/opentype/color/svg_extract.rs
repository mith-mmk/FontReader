//! Bounded, passive SVG subset extraction. This is not a general XML/CSS engine.
//!
//! Validate and index the source, then follow only the selected glyph's local
//! references and serialize a pruned original tree.
//! Unselected dependency branches live in defs, preserving inherited properties
//! without painting sibling glyphs. No external resources or entity expansion.
use std::collections::{HashMap, HashSet, VecDeque};

const MAX_NODES: usize = 262_144;
const MAX_DEPTH: usize = 128;
const MAX_REACHABLE_REFERENCES: usize = 65_536;
const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    parent: Option<usize>,
    children: Vec<usize>,
    end: usize,
}

fn name_ok(name: &str) -> bool {
    let mut chars = name.bytes();
    chars
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && chars.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
        && name.matches(':').count() <= 1
        && !name.ends_with(':')
}

fn xml_value(value: &str) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find('&') {
        output.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let end = tail.find(';')?;
        let entity = &tail[..end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(n) = entity.strip_prefix("#x") {
                    u32::from_str_radix(n, 16).ok()?
                } else {
                    entity.strip_prefix('#')?.parse().ok()?
                };
                char::from_u32(code)?
            }
        };
        output.push(c);
        rest = &tail[end + 1..];
    }
    output.push_str(rest);
    // XML 1.0 legal scalar values, including attribute whitespace normalization.
    if output.chars().any(|c| {
        !(matches!(c, '\t' | '\n' | '\r') || c >= '\u{20}') || matches!(c, '\u{fffe}' | '\u{ffff}')
    }) {
        return None;
    }
    Some(output.replace(['\n', '\r', '\t'], " "))
}

fn attributes(tag: &str) -> Option<(String, Vec<(String, String)>)> {
    let bytes = tag.as_bytes();
    let mut cursor = 0;
    while bytes.get(cursor).is_some_and(|b| !b.is_ascii_whitespace()) {
        cursor += 1;
    }
    let name = &tag[..cursor];
    if !name_ok(name) {
        return None;
    }
    let mut attrs = Vec::new();
    let mut names = HashSet::new();
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_whitespace() {
            return None;
        }
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if cursor == bytes.len() {
            break;
        }
        let start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'=')
        {
            cursor += 1;
        }
        let attr = &tag[start..cursor];
        if !name_ok(attr) || !names.insert(attr) || attrs.len() >= 128 {
            return None;
        }
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            return None;
        }
        cursor += 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        let quote = *bytes.get(cursor)?;
        if !matches!(quote, b'\'' | b'"') {
            return None;
        }
        cursor += 1;
        let start = cursor;
        while *bytes.get(cursor)? != quote {
            cursor += 1;
        }
        let raw = &tag[start..cursor];
        if raw.contains('<') {
            return None;
        }
        attrs.push((attr.to_string(), xml_value(raw)?));
        cursor += 1;
    }
    Some((name.to_string(), attrs))
}

fn namespace<'a>(nodes: &'a [Node], mut index: usize, prefix: &str) -> Option<&'a str> {
    let attr = if prefix.is_empty() {
        "xmlns".to_string()
    } else {
        format!("xmlns:{prefix}")
    };
    loop {
        if let Some((_, value)) = nodes[index].attrs.iter().find(|(name, _)| name == &attr) {
            return Some(value);
        }
        index = nodes[index].parent?;
    }
}

fn local_name<'a>(nodes: &[Node], index: usize, name: &'a str, attribute: bool) -> Option<&'a str> {
    if let Some((prefix, local)) = name.split_once(':') {
        let ns = namespace(nodes, index, prefix)?;
        if (attribute && ns == XLINK_NS && local == "href") || (!attribute && ns == SVG_NS) {
            return Some(local);
        }
        return None;
    }
    if !attribute && namespace(nodes, index, "").is_some_and(|ns| ns != SVG_NS) {
        return None;
    }
    Some(name)
}

fn local_id(value: &str) -> Option<&str> {
    let id = value.trim().strip_prefix('#')?;
    (!id.is_empty()
        && id.bytes().all(|b| {
            !b.is_ascii_whitespace() && !matches!(b, b'\'' | b'"' | b'(' | b')' | b'\\' | b'#')
        }))
    .then_some(id)
}

fn references<'a>(value: &'a str, visit: &mut impl FnMut(&'a str) -> Option<()>) -> Option<()> {
    // Reject CSS escapes/comments rather than attempting a partial CSS decoder.
    if value.contains(['\\', '@']) || value.contains("/*") {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if lower.contains("javascript:") || lower.contains("vbscript:") || lower.contains("expression(")
    {
        return None;
    }
    let mut rest = lower.as_str();
    let mut offset = 0;
    while let Some(pos) = rest.find("url") {
        let token = offset + pos;
        if token > 0
            && (lower.as_bytes()[token - 1].is_ascii_alphanumeric()
                || matches!(lower.as_bytes()[token - 1], b'_' | b'-'))
        {
            offset = token + 3;
            rest = &lower[offset..];
            continue;
        }
        let start = token + 3;
        if lower
            .as_bytes()
            .get(start)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(*b, b'_' | b'-'))
        {
            offset = start;
            rest = &lower[offset..];
            continue;
        }
        let after = value[start..].trim_start();
        if !after.starts_with('(') {
            return None;
        }
        let leading = value[start..].len() - after.len();
        let end = after.find(')')?;
        let arg = after[1..end].trim();
        let arg = if arg.starts_with(['\'', '"']) {
            let quote = arg.as_bytes()[0];
            if arg.len() < 2 || arg.as_bytes().last() != Some(&quote) {
                return None;
            }
            &arg[1..arg.len() - 1]
        } else {
            arg
        };
        visit(local_id(arg)?)?;
        offset = start + leading + end + 1;
        rest = &lower[offset..];
    }
    Some(())
}

fn presentation(name: &str) -> bool {
    matches!(
        name,
        "fill"
            | "fill-opacity"
            | "fill-rule"
            | "stroke"
            | "stroke-width"
            | "stroke-opacity"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "opacity"
            | "color"
            | "stop-color"
            | "stop-opacity"
            | "clip-path"
            | "clip-rule"
            | "mask"
            | "display"
            | "visibility"
            | "color-interpolation"
            | "color-interpolation-filters"
            | "paint-order"
            | "vector-effect"
            | "transform"
            | "transform-origin"
            | "transform-box"
            | "overflow"
    )
}

fn validate<'a>(
    nodes: &'a [Node],
    i: usize,
    mut visit: impl FnMut(&'a str) -> Option<()>,
) -> Option<()> {
    let node = &nodes[i];
    let local = local_name(nodes, i, &node.name, false)?;
    if !matches!(
        local,
        "svg"
            | "g"
            | "path"
            | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "polyline"
            | "polygon"
            | "defs"
            | "linearGradient"
            | "radialGradient"
            | "stop"
            | "clipPath"
            | "mask"
            | "use"
            | "symbol"
    ) {
        return None;
    }
    let mut expanded = HashSet::new();
    for (name, value) in &node.attrs {
        if name == "xmlns" || name.starts_with("xmlns:") {
            if !matches!(value.as_str(), SVG_NS | XLINK_NS | XML_NS) {
                return None;
            }
            if name == "xmlns:xmlns" || (name != "xmlns:xml" && value == XML_NS) {
                return None;
            }
            if name == "xmlns:xml" && value != XML_NS {
                return None;
            }
            continue;
        }
        if name == "xml:space" && matches!(value.as_str(), "default" | "preserve") {
            continue;
        }
        let local = local_name(nodes, i, name, true)?;
        // href and an xlink alias cannot ambiguously select different resources.
        if !expanded.insert(local) {
            return None;
        }
        if local == "href" {
            visit(local_id(value)?)?;
        } else if local == "style" {
            for declaration in value.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                let (property, value) = declaration.split_once(':')?;
                if !presentation(property.trim()) || value.contains('!') {
                    return None;
                }
                references(value, &mut visit)?;
            }
        } else if presentation(local) {
            references(value, &mut visit)?;
        } else if !local.starts_with("data-")
            && !matches!(
                local,
                "id" | "class"
                    | "version"
                    | "viewBox"
                    | "preserveAspectRatio"
                    | "x"
                    | "y"
                    | "x1"
                    | "y1"
                    | "x2"
                    | "y2"
                    | "cx"
                    | "cy"
                    | "r"
                    | "rx"
                    | "ry"
                    | "fx"
                    | "fy"
                    | "fr"
                    | "width"
                    | "height"
                    | "d"
                    | "points"
                    | "pathLength"
                    | "offset"
                    | "gradientUnits"
                    | "gradientTransform"
                    | "spreadMethod"
                    | "clipPathUnits"
                    | "maskUnits"
                    | "maskContentUnits"
            )
        {
            return None;
        }
    }
    Some(())
}

// enable-background only establishes a filter background buffer. This subset
// rejects every filter element/property, so the supported grammar is inert.
// Validate narrowly before removing it; do not discard arbitrary CSS/metadata.
fn background_buffer(value: &str) -> Option<()> {
    let words: Vec<_> = value.split_ascii_whitespace().collect();
    if words == ["accumulate"] || words == ["new"] {
        return Some(());
    }
    if words.len() != 5 || words[0] != "new" {
        return None;
    }
    let values: Option<Vec<f64>> = words[1..].iter().map(|s| s.parse::<f64>().ok()).collect();
    let values = values?;
    (values.iter().all(|n| n.is_finite()) && values[2] >= 0.0 && values[3] >= 0.0).then_some(())
}

fn normalize_export_metadata(attrs: &mut Vec<(String, String)>) -> Option<()> {
    for (name, value) in attrs.iter_mut() {
        if name == "enable-background" {
            background_buffer(value)?;
        }
        if name == "style" {
            let mut kept = Vec::new();
            for declaration in value.split(';').filter(|s| !s.trim().is_empty()) {
                let (property, item) = declaration.split_once(':')?;
                if property.trim() == "enable-background" {
                    background_buffer(item)?;
                } else {
                    kept.push(declaration);
                }
            }
            *value = kept.join(";");
        }
    }
    attrs.retain(|(name, value)| {
        name != "enable-background" && !(name == "style" && value.is_empty())
    });
    Some(())
}

fn parse(document: &str) -> Option<Vec<Node>> {
    let document = document.strip_prefix('\u{feff}').unwrap_or(document);
    let mut nodes: Vec<Node> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut cursor = 0;
    while cursor < document.len() {
        let tail = &document[cursor..];
        if tail.starts_with("<!--") {
            let end = tail.find("-->")?;
            if tail[4..end].contains("--") {
                return None;
            }
            cursor += end + 3;
            continue;
        }
        if tail.starts_with("<?xml ") && nodes.is_empty() && document[..cursor].trim().is_empty() {
            let end = tail.find("?>")?;
            if !tail[..end].contains("version=") && !tail[..end].contains("version =") {
                return None;
            }
            cursor += end + 2;
            continue;
        }
        if !tail.starts_with('<') {
            let length = tail.find('<').unwrap_or(tail.len());
            if !tail[..length].trim().is_empty() {
                return None;
            }
            cursor += length;
            continue;
        }
        let end = super::svg::find_svg_tag_end(document, cursor)?;
        let raw = &document[cursor + 1..end - 1];
        if let Some(close) = raw.strip_prefix('/') {
            let index = stack.pop()?;
            if close.trim_end() != nodes[index].name {
                return None;
            }
            nodes[index].end = nodes.len();
        } else {
            let self_closing = raw.ends_with('/');
            let raw = if self_closing {
                &raw[..raw.len() - 1]
            } else {
                raw
            };
            let (name, mut attrs) = attributes(raw)?;
            normalize_export_metadata(&mut attrs)?;
            if nodes.len() >= MAX_NODES
                || stack.len() >= MAX_DEPTH
                || (stack.is_empty() && !nodes.is_empty())
            {
                return None;
            }
            let parent = stack.last().copied();
            let index = nodes.len();
            if let Some(parent) = parent {
                nodes[parent].children.push(index);
            }
            nodes.push(Node {
                name,
                attrs,
                parent,
                children: Vec::new(),
                end: index + 1,
            });
            // Checks apply to the entire document, but do not allocate reference
            // edges for glyphs that may never be extracted. The byte, node,
            // depth and per-element attribute limits still bound this scan.
            validate(&nodes, index, |_| Some(()))?;
            if !self_closing {
                stack.push(index);
            }
        }
        cursor = end;
    }
    if !stack.is_empty()
        || nodes.is_empty()
        || local_name(&nodes, 0, &nodes[0].name, false)? != "svg"
    {
        return None;
    }
    Some(nodes)
}

fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attrs
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

fn glyph_id(id: &str) -> Option<u16> {
    let suffix = id.strip_prefix("glyph").unwrap_or(id);
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

fn escaped(output: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(c),
        }
    }
}

fn emit(
    nodes: &[Node],
    i: usize,
    included: &[bool],
    visible: &[bool],
    hidden: bool,
    output: &mut String,
) {
    let node = &nodes[i];
    output.push('<');
    output.push_str(&node.name);
    for (name, value) in &node.attrs {
        output.push(' ');
        output.push_str(name);
        output.push_str("=\"");
        escaped(output, value);
        output.push('"');
    }
    // Legacy unqualified documents need an explicit namespace as standalone SVG.
    if i == 0 && !node.name.contains(':') && attr(node, "xmlns").is_none() {
        output.push_str(" xmlns=\"http://www.w3.org/2000/svg\"");
    }
    output.push('>');
    let local = node.name.rsplit(':').next().unwrap();
    let hidden = hidden
        || matches!(
            local,
            "defs" | "symbol" | "clipPath" | "mask" | "linearGradient" | "radialGradient"
        );
    for &child in &node.children {
        if !included[child] {
            continue;
        }
        let child_hidden = matches!(
            nodes[child].name.rsplit(':').next().unwrap(),
            "defs" | "symbol" | "clipPath" | "mask" | "linearGradient" | "radialGradient"
        );
        let wrap = !hidden && !visible[child] && !child_hidden;
        // Use an explicit namespace so prefixed source documents also work.
        if wrap {
            output.push_str("<defs xmlns=\"http://www.w3.org/2000/svg\">");
        }
        emit(nodes, child, included, visible, hidden || wrap, output);
        if wrap {
            output.push_str("</defs>");
        }
    }
    output.push_str("</");
    output.push_str(&node.name);
    output.push('>');
}

#[cfg(test)]
pub(super) fn extract(document: &str, gid: u16) -> Option<String> {
    extract_with_units(document, gid, 1000)
}

pub(super) fn extract_with_units(document: &str, gid: u16, units_per_em: u16) -> Option<String> {
    if units_per_em == 0 {
        return None;
    }
    let mut nodes = parse(document)?;
    let mut ids = HashMap::new();
    let mut selected = None;
    let mut any_glyph = false;
    for (i, node) in nodes.iter().enumerate() {
        if let Some(id) = attr(node, "id") {
            if id.is_empty() || id.chars().any(char::is_whitespace) || ids.insert(id, i).is_some() {
                return None;
            }
            if let Some(glyph) = glyph_id(id) {
                any_glyph = true;
                if glyph == gid && selected.replace(i).is_some() {
                    return None;
                }
            }
        }
    }
    let selected = selected.or_else(|| (!any_glyph).then_some(0))?;
    let mut included = vec![false; nodes.len()];
    let mut visible = vec![false; nodes.len()];
    let mut full = vec![false; nodes.len()];
    let mut queue = VecDeque::new();
    // Retain whole glyph subtree and only skeleton ancestors, not siblings.
    for i in selected..nodes[selected].end {
        included[i] = true;
        visible[i] = true;
        full[i] = true;
        queue.push_back(i);
    }
    let mut parent = nodes[selected].parent;
    while let Some(i) = parent {
        included[i] = true;
        visible[i] = true;
        queue.push_back(i);
        parent = nodes[i].parent;
    }
    // Process each included node once. Budget its references, including those
    // in ancestors and transitive dependencies. Retain only resolved edges
    // so cycle/cost checks can reuse them without rescanning.
    let mut targets: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut references_remaining = MAX_REACHABLE_REFERENCES;
    while let Some(i) = queue.pop_front() {
        validate(&nodes, i, |reference| {
            references_remaining = references_remaining.checked_sub(1)?;
            let target = *ids.get(reference)?;
            targets.entry(i).or_default().push(target);
            let mut j = target;
            while j < nodes[target].end {
                if full[j] {
                    j = nodes[j].end;
                    continue;
                }
                full[j] = true;
                if !included[j] {
                    included[j] = true;
                    queue.push_back(j);
                }
                j += 1;
            }
            let mut parent = nodes[target].parent;
            while let Some(j) = parent {
                if included[j] {
                    break;
                }
                included[j] = true;
                queue.push_back(j);
                parent = nodes[j].parent;
            }
            Some(())
        })?;
    }
    // Reject reference cycles (including use -> ancestor -> use) without
    // recursively expanding references. Iterative DFS bounded by node count.
    let mut colors = vec![0u8; nodes.len()];
    let mut costs = vec![0usize; nodes.len()];
    let mut depths = vec![0usize; nodes.len()];
    let mut dfs = vec![(0usize, false)];
    while let Some((i, exit)) = dfs.pop() {
        if exit {
            let mut cost = 1usize;
            let mut depth = 1usize;
            for target in nodes[i]
                .children
                .iter()
                .copied()
                .filter(|&c| included[c])
                .chain(targets.get(&i).into_iter().flatten().copied())
            {
                cost = cost.checked_add(costs[target])?;
                depth = depth.max(depths[target] + 1);
                if cost > MAX_NODES || depth > MAX_DEPTH {
                    return None;
                }
            }
            costs[i] = cost;
            depths[i] = depth;
            colors[i] = 2;
            continue;
        }
        if colors[i] == 1 {
            return None;
        }
        if colors[i] == 2 {
            continue;
        }
        colors[i] = 1;
        dfs.push((i, true));
        for &target in targets.get(&i).into_iter().flatten() {
            dfs.push((target, false));
        }
        for &child in &nodes[i].children {
            if included[child] {
                dfs.push((child, false));
            }
        }
    }
    // OpenType sets the initial viewport to the em square, never the advance
    // width, and forbids clipping at the document root. Explicit dimensions
    // retain their meaning; percentages are resolved against that em square.
    for key in ["width", "height"] {
        let size = match attr(&nodes[0], key) {
            None => units_per_em as f64,
            Some(value) => {
                if let Some(percent) = value.trim().strip_suffix('%') {
                    percent.trim().parse::<f64>().ok()? * units_per_em as f64 / 100.0
                } else {
                    value
                        .trim()
                        .strip_suffix("px")
                        .unwrap_or(value.trim())
                        .trim()
                        .parse::<f64>()
                        .ok()?
                }
            }
        };
        if !size.is_finite() || size <= 0.0 {
            return None;
        }
        nodes[0].attrs.retain(|(name, _)| name != key);
        nodes[0].attrs.push((key.to_string(), size.to_string()));
    }
    nodes[0]
        .attrs
        .retain(|(name, _)| !matches!(name.as_str(), "x" | "y" | "overflow"));
    if let Some((_, style)) = nodes[0].attrs.iter_mut().find(|(name, _)| name == "style") {
        *style = style
            .split(';')
            .filter(|s| {
                s.split_once(':')
                    .is_none_or(|(key, _)| key.trim() != "overflow")
            })
            .collect::<Vec<_>>()
            .join(";");
        style.push_str(";overflow:visible");
    }
    nodes[0]
        .attrs
        .push(("overflow".to_string(), "visible".to_string()));
    let mut output = String::new();
    emit(&nodes, 0, &included, &visible, false, &mut output);
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTO: &str = include_str!("../../../tests/fixtures/svg-extraction/noto-emoji_u1f600.svg");
    const TWEMOJI: &str = include_str!("../../../tests/fixtures/svg-extraction/twemoji-glyph5.svg");

    #[test]
    fn real_artwork_and_font_document_extract() {
        let noto = extract_with_units(NOTO, 1, 128).unwrap();
        assert!(noto.contains("radialGradient"));
        assert!(noto.contains("style="));
        assert!(noto.contains("viewBox=\"0 0 128 128\""));
        let twemoji = extract_with_units(TWEMOJI, 5, 2048).unwrap();
        assert!(twemoji.contains("translate(0,-1638.4)"));
        assert!(twemoji.contains("width=\"2048\""));
        assert!(twemoji.contains("overflow=\"visible\""));
        assert!(extract_with_units(TWEMOJI, 6, 2048).is_none());
    }

    #[test]
    fn illustrator_inert_background_metadata_and_pixel_viewport() {
        let source = "<svg version='1.1' x='0px' y='0px' width='128px' height='128px' xml:space='preserve' style='enable-background:new 0 0 128 128;fill:#123456'><g enable-background='new'><path id='glyph1' d='M0 0L1 1'/></g></svg>";
        let out = extract_with_units(source, 1, 128).unwrap();
        assert!(!out.contains("enable-background"));
        assert!(out.contains("fill:#123456"));
        assert!(out.contains("width=\"128\""));
        assert!(out.contains("xml:space=\"preserve\""));
        for value in [
            "new url(https://evil.test)",
            "new 0 0 NaN 128",
            "new 0 0 -1 128",
            "new 0 0 128",
            "new;fill:red",
            "new 0 0 128 128 !important",
        ] {
            assert!(extract(&format!("<svg enable-background='{value}'/>"), 1).is_none());
        }
        assert!(extract(
            "<svg style='enable-background:new;filter:url(#f)'><filter id='f'/></svg>",
            1
        )
        .is_none());
        assert!(extract(
            "<!DOCTYPE svg [<!ENTITY ns 'http://www.w3.org/2000/svg'>]><svg xmlns='&ns;'/>",
            1
        )
        .is_none());
        assert!(extract("<svg width='128pt'/>", 1).is_none());
    }

    #[test]
    fn real_illustrator_noto_document_extracts_without_background_metadata() {
        let source = include_str!("../../../tests/fixtures/svg-extraction/noto-emoji_u1f308.svg");
        let out = extract_with_units(source, 5, 128).unwrap();
        assert!(out.contains("#FF5117"));
        assert!(!out.contains("enable-background"));
        assert_eq!(
            out.matches("<path").count(),
            source.matches("<path").count()
        );
    }

    #[test]
    #[ignore = "requires FONTCORE_NOTO_SVG_CORPUS directory of pinned source assets"]
    fn external_noto_artwork_extraction_corpus() {
        let directory =
            std::path::PathBuf::from(std::env::var("FONTCORE_NOTO_SVG_CORPUS").unwrap());
        let mut passed = Vec::new();
        let mut rejected = Vec::new();
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|s| s != "svg") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            if extract_with_units(&source, 5, 128).is_some() {
                passed.push(path.file_name().unwrap().to_string_lossy().into_owned());
            } else {
                rejected.push(path.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
        rejected.sort();
        eprintln!(
            "Noto extraction: {} accepted; rejected {:?}",
            passed.len(),
            rejected
        );
        assert_eq!(passed.len(), 18);
        assert_eq!(rejected, ["emoji_u1f436.svg", "emoji_u1f680.svg"]);
    }

    #[test]
    fn root_viewport_and_passive_color_regressions() {
        let out = extract_with_units("<svg width='50%' style='overflow:hidden;fill:burlywood'><path id='glyph1' d='M0 -800h500v800H0Z'/></svg>", 1, 2048).unwrap();
        assert!(out.contains("width=\"1024\""));
        assert!(out.contains("height=\"2048\""));
        assert!(out.contains("overflow:visible"));
        assert!(!out.contains("overflow:hidden"));
        assert!(out.contains("burlywood"));
    }

    #[test]
    #[ignore = "writes extracted real-data examples to FONTCORE_SVG_EXPORT_DIR"]
    fn export_real_svg_fixtures() {
        let output = std::path::PathBuf::from(std::env::var("FONTCORE_SVG_EXPORT_DIR").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        for (name, input, gid, em) in [
            ("noto-smile.svg", NOTO, 1, 128),
            ("twemoji-glyph5.svg", TWEMOJI, 5, 2048),
        ] {
            std::fs::write(
                output.join(name),
                extract_with_units(input, gid, em).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn ancestor_styles_transforms_and_transitive_references_are_preserved() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100" fill="red"><defs><linearGradient id="base"><stop offset="0" stop-color="gold"/></linearGradient><linearGradient id="paint" xlink:href="#base"/><path id="unused" d="M99 99"/></defs><g transform="translate(2 3)" style="opacity: .5; fill: url('#paint')"><g id="glyph1"><use xlink:href="#shape"/></g><g id="glyph2"><path d="M8 8"/></g></g><g fill="blue"><path id="shape" d="M0 0L10 10"/></g></svg>"##;
        let out = extract(svg, 1).unwrap();
        for needed in [
            "viewBox",
            "translate(2 3)",
            "opacity: .5",
            "#paint",
            "id=\"base\"",
            "id=\"shape\"",
            "fill=\"blue\"",
            "xmlns:xlink",
        ] {
            assert!(out.contains(needed), "{needed}: {out}");
        }
        for unwanted in ["glyph2", "unused", "M8 8"] {
            assert!(!out.contains(unwanted), "{out}");
        }
        assert!(out.contains("<defs xmlns=\"http://www.w3.org/2000/svg\"><g fill=\"blue\">"));
        assert!(extract(&out, 1).unwrap().contains("id=\"shape\""));
    }

    #[test]
    fn prefixes_and_namespace_rebinding_are_checked() {
        let source = r##"<s:svg xmlns:s="http://www.w3.org/2000/svg" xmlns:l="http://www.w3.org/1999/xlink"><s:defs><s:path id="p"/></s:defs><s:g id="glyph1"><s:use l:href="#p"/></s:g></s:svg>"##;
        assert!(extract(source, 1).unwrap().contains("l:href"));
        for bad in [
            "<svg xmlns='urn:evil'><g/></svg>",
            "<svg><s:g/></svg>",
            "<svg xmlns:s='http://www.w3.org/2000/svg'><s:script/></svg>",
            "<svg xml:base='https://evil.test'><path/></svg>",
        ] {
            assert!(extract(bad, 1).is_none(), "{bad}");
        }
    }

    #[test]
    fn entities_comments_and_quoted_tag_delimiters() {
        let source = "<?xml version='1.0'?><!-- <g id='glyph8'> --><svg><path id='shape' d='M0 0'/><g id='glyph1' fill='url(&quot;#sh&#97;pe&quot;)'/></svg>";
        assert!(extract(source, 1).unwrap().contains("id=\"shape\""));
        assert!(extract("<svg><g id='glyph1' class='a>b'/></svg>", 1).is_some());
        assert!(extract("<svg><!--oops-- --><path/></svg>", 1).is_none());
        assert!(extract("<!DOCTYPE svg [<!ENTITY a 'x'>]><svg/>", 1).is_none());
        assert!(extract("<svg fill='&unknown;'/>", 1).is_none());
        assert!(extract("<svg fill='&#0;'/>", 1).is_none());
    }

    #[test]
    fn rejects_external_active_and_ambiguous_inputs() {
        for body in [
            "<use href='&#104;ttps://evil.test/x'/>",
            "<use href='//evil.test/x'/>",
            "<path fill='url(https://evil.test/x)'/>",
            "<path style='fill: u\\72l(#p)'/>",
            "<path style='fill: url(/**/#p)'/>",
            "<path onclick='x'/>",
            "<script/>",
            "<style>path{fill:red}</style>",
            "<image href='data:image/png;base64,AA'/>",
            "<g id='x'/><g id='x'/>",
            "<g id='x' id='y'/>",
            "<g><path/></svg>",
            "<use href='#missing'/>",
            "<g xmlns='http://www.w3.org/1999/xhtml'/>",
        ] {
            assert!(
                extract(&format!("<svg>{body}</svg>"), 1).is_none(),
                "{body}"
            );
        }
    }

    #[test]
    fn cycles_depth_and_exponential_reference_expansion_are_bounded() {
        for body in [
            "<g id='a'><use href='#a'/></g>",
            "<g id='a'><use href='#b'/></g><g id='b'><use href='#a'/></g>",
            "<linearGradient id='a' href='#b'/><linearGradient id='b' href='#a'/>",
        ] {
            assert!(extract(&format!("<svg>{body}</svg>"), 1).is_none());
        }
        let deep = format!(
            "<svg>{}<path/>{}</svg>",
            "<g>".repeat(MAX_DEPTH),
            "</g>".repeat(MAX_DEPTH)
        );
        assert!(extract(&deep, 1).is_none());
        let mut bomb = String::from("<svg><defs><path id='n0'/>");
        for i in 1..24 {
            bomb.push_str(&format!(
                "<g id='n{i}'><use href='#n{}'/><use href='#n{}'/></g>",
                i - 1,
                i - 1
            ));
        }
        bomb.push_str("</defs><g id='glyph1'><use href='#n23'/></g></svg>");
        assert!(extract(&bomb, 1).is_none());
    }

    #[test]
    fn shared_document_budgets_only_reachable_references() {
        let mut document = String::from("<svg><defs><path id='p' d='M1 1'/><g id='many'>");
        for _ in 0..MAX_REACHABLE_REFERENCES {
            document.push_str("<use href='#p'/>");
        }
        document.push_str(
            "</g></defs><g id='glyph1'><use href='#p'/></g><use id='glyph2' href='#many'/></svg>",
        );
        let out = extract(&document, 1).unwrap();
        assert!(out.contains("id=\"p\""));
        assert!(!out.contains("id=\"many\""));
        assert!(!out.contains("glyph2"));
        // The same dense subtree is over budget when it is reachable through
        // a local dependency, even though its node count is below the limit.
        assert!(extract(&document, 2).is_none());
    }

    #[test]
    fn unselected_nodes_still_receive_passive_validation() {
        for sibling in [
            "<use href='https://example.test/x'/>",
            "<path fill='url(https://example.test/x)'/>",
            "<style>path{fill:red}</style>",
            "<g id='glyph1'/>",
        ] {
            let document =
                format!("<svg><path id='glyph1' d='M0 0'/><g id='glyph2'>{sibling}</g></svg>");
            assert!(extract(&document, 1).is_none(), "{sibling}");
        }
    }

    #[test]
    fn large_shared_document_keeps_only_reachable_resources() {
        let mut source = String::from("<svg><defs>");
        for i in 0..10_000 {
            source.push_str(&format!("<path id='p{i}' d='M{i} 0'/>"));
        }
        source.push_str("</defs>");
        for i in 0..10_000 {
            source.push_str(&format!("<g id='glyph{i}'><use href='#p{i}'/></g>"));
        }
        source.push_str("</svg>");
        let out = extract(&source, 123).unwrap();
        assert!(source.len() > 700_000);
        assert!(out.len() < 400, "{}", out.len());
        assert!(out.contains("id=\"p123\""));
        assert!(!out.contains("glyph1234"));
    }
}
