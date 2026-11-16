//! Bounded font-aware canonical composition, not general Unicode normalization.
use super::{Font, ResolvedGlyph};
use crate::unicode::latin_composition_data::{DECOMPOSITIONS, MARK_CLASSES};

pub(super) fn original(ch: char) -> Option<&'static [char]> {
    DECOMPOSITIONS
        .iter()
        .find(|(cp, _)| *cp == ch)
        .map(|(_, parts)| *parts)
}

fn admitted(text: &str) -> Option<Vec<char>> {
    // Bound work before allocation. No selectors, joiners, CGJ or reordering.
    let chars: Vec<_> = text.chars().take(4).collect();
    if !(2..=3).contains(&chars.len()) || !chars[0].is_ascii_alphabetic() {
        return None;
    }
    let mut previous = 0;
    for ch in &chars[1..] {
        let class = MARK_CLASSES.iter().find(|(cp, _)| cp == ch)?.1;
        if class < previous {
            return None;
        }
        previous = class;
    }
    Some(chars)
}

impl Font {
    pub(super) fn is_composed_latin(glyph: &ResolvedGlyph) -> bool {
        original(glyph.shaping_ch).is_some_and(|parts| {
            parts.len() == glyph.source_end - glyph.source_start && parts[0] == glyph.ch
        })
    }

    pub(super) fn composed_latin_glyphs(
        &self,
        text: &str,
        source_start: usize,
    ) -> Option<Vec<ResolvedGlyph>> {
        let chars = admitted(text)?;
        // Try the longest supported canonical prefix. The residual mark must
        // remain independently renderable, or retain the entire original EGC.
        for len in (2..=chars.len()).rev() {
            let Some((target, _)) = DECOMPOSITIONS
                .iter()
                .find(|(_, parts)| *parts == &chars[..len])
            else {
                continue;
            };
            let glyph_id = self.resolve_glyph_id_with_uvs(*target, '\0', false).ok()?;
            if !self.composition_glyph_is_renderable(glyph_id) {
                continue;
            }
            let base = ResolvedGlyph {
                byte_start: 0,
                byte_end: 0,
                ch: chars[0],
                shaping_ch: *target,
                source_start,
                source_end: source_start + len,
                glyph_id,
                prefer_color: false,
                ligature_components: 1,
                tibetan_cluster: None,
                kana_residual_mark: false,
                simple_mkmk_cluster: None,
                tibetan_upper_anchor: None,
            };
            let mut output = vec![base];
            for (index, ch) in chars.iter().enumerate().skip(len) {
                let glyph_id = self.resolve_glyph_id_with_uvs(*ch, '\0', false).ok()?;
                if !self.composition_glyph_is_renderable(glyph_id) {
                    return None;
                }
                output.push(ResolvedGlyph {
                    ch: *ch,
                    shaping_ch: *ch,
                    source_start: source_start + index,
                    source_end: source_start + index + 1,
                    glyph_id,
                    ..base
                });
            }
            return Some(output);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_is_bounded_and_does_not_reorder_or_accept_controls() {
        for text in [
            "",
            "a",
            "ã",
            "a\u{303}\u{325}",
            "a\u{303}\u{301}\u{300}",
            "a\u{34f}\u{301}",
            "a\u{fe0f}",
            "a\u{200d}\u{301}",
            "α\u{301}",
        ] {
            assert!(admitted(text).is_none(), "{text:?}");
        }
        for text in ["a\u{303}\u{301}", "a\u{325}\u{303}", "a\u{306}\u{301}"] {
            assert!(admitted(text).is_some());
        }
        assert!(admitted(&format!("a{}", "\u{301}".repeat(100_000))).is_none());
    }
    #[test]
    fn source_spans_survive_composition_and_rollback() {
        use crate::commands::FontVariant;
        use crate::fontreader::ResolvedTextUnit;
        for (face, text, spans) in [
            (0, "a\u{303}\u{301}B", vec![(0, 2), (2, 3), (3, 4)]),
            (0, "a\u{306}\u{301}B", vec![(0, 3), (3, 4)]),
            (3, "a\u{306}\u{301}B", vec![(0, 1), (1, 2), (2, 3), (3, 4)]),
        ] {
            let mut font = Font::get_font_from_buffer(include_bytes!(
                "../../tests/fixtures/latin-composition.ttc"
            ))
            .unwrap();
            font.set_font(face).unwrap();
            let units = font
                .shape_text_units(text, false, false, None, FontVariant::Normal)
                .unwrap();
            assert_eq!(units.len(), spans.len());
            for (unit, (start, end)) in units.iter().zip(spans) {
                let ResolvedTextUnit::Glyph(g) = unit else {
                    panic!("glyph expected")
                };
                assert_eq!((g.source_start, g.source_end), (start, end));
                assert_eq!(g.ch, text.chars().nth(start).unwrap());
                assert_eq!(g.ligature_components, 1);
            }
        }
    }
    #[test]
    fn canonical_table_has_unique_bounded_sources() {
        let mut seen = std::collections::HashSet::new();
        for (target, parts) in DECOMPOSITIONS {
            assert!(seen.insert(*parts));
            let text: String = parts.iter().collect();
            assert_eq!(admitted(&text).unwrap(), *parts);
            assert_eq!(original(*target), Some(*parts));
        }
    }
}
