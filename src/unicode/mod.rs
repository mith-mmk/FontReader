//! Unicode character properties, independent of fonts and shaping.
//!
//! フォントやシェーピングに依存しない Unicode 文字プロパティです。

mod enclosing_mark_data;
pub(crate) mod graphemes;
mod vertical_orientation_data;

/// Whether a scalar has General_Category=Me in the pinned Unicode 18.0.0 data.
/// Kept internal: this is not a general combining-mark or grapheme query.
pub(crate) fn is_enclosing_mark(character: char) -> bool {
    let scalar = character as u32;
    enclosing_mark_data::RANGES
        .binary_search_by(|&(start, end)| {
            if end < scalar {
                std::cmp::Ordering::Less
            } else if start > scalar {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

#[cfg(test)]
mod enclosing_mark_tests {
    use super::{enclosing_mark_data, is_enclosing_mark};

    #[test]
    fn enclosing_mark_matches_independent_source_for_every_scalar() {
        let source = include_str!("../../data/unicode/18.0.0/DerivedGeneralCategory.txt");
        assert!(source.starts_with("# DerivedGeneralCategory-18.0.0.txt\n"));
        // Parse every category, independently of the generator and lookup table.
        // The official extracted file partitions the whole Unicode code space.
        let mut categories = vec![None; 0x110000];
        for line in source.lines() {
            let data = line.split('#').next().unwrap().trim();
            if data.is_empty() {
                continue;
            }
            let (range, category) = data.split_once(';').unwrap();
            let mut bounds = range.trim().split("..");
            let start = u32::from_str_radix(bounds.next().unwrap(), 16).unwrap();
            let end = bounds
                .next()
                .map(|value| u32::from_str_radix(value, 16).unwrap())
                .unwrap_or(start);
            assert!(bounds.next().is_none());
            assert!(start <= end && end <= 0x10FFFF);
            for scalar in start..=end {
                assert!(categories[scalar as usize].is_none());
                categories[scalar as usize] = Some(category.trim() == "Me");
            }
        }
        let mut scalar_count = 0;
        let mut mark_count = 0;
        for (scalar, category) in categories.into_iter().enumerate() {
            let expected = category.expect("source must cover every code point");
            if let Some(character) = char::from_u32(scalar as u32) {
                assert_eq!(is_enclosing_mark(character), expected, "U+{scalar:04X}");
                scalar_count += 1;
                mark_count += usize::from(expected);
            } else {
                assert!(!expected, "surrogates cannot be enclosing marks");
            }
        }
        assert_eq!(scalar_count, 1_112_064);
        assert_eq!(mark_count, 13);
    }

    #[test]
    fn enclosing_mark_ranges_and_boundaries() {
        assert_eq!(
            enclosing_mark_data::RANGES,
            &[
                (0x0488, 0x0489),
                (0x1ABE, 0x1ABE),
                (0x20DD, 0x20E0),
                (0x20E2, 0x20E4),
                (0xA670, 0xA672)
            ]
        );
        for &(start, end) in enclosing_mark_data::RANGES {
            assert!(!is_enclosing_mark(char::from_u32(start - 1).unwrap()));
            assert!(!is_enclosing_mark(char::from_u32(end + 1).unwrap()));
            for scalar in start..=end {
                assert!(is_enclosing_mark(char::from_u32(scalar).unwrap()));
            }
        }
        // Mn, Mc, spacing marks, supplementary selectors and noncharacters
        // must not be confused with the Me category.
        for character in [
            '\u{0301}',
            '\u{0903}',
            '\u{309B}',
            '\u{E0100}',
            '\0',
            '\u{10FFFF}',
        ] {
            assert!(!is_enclosing_mark(character));
        }
    }
}

/// Unicode version used by [`vertical_orientation`], currently `"18.0.0"`.
///
/// This version applies only to this property table, not to other fontcore APIs.
/// See `data/unicode/README.md` in the source distribution for provenance.
///
/// [`vertical_orientation`] のデータに用いる Unicode バージョンです。
/// 他の API の Unicode 対応バージョンを表すものではありません。
pub const VERTICAL_ORIENTATION_UNICODE_VERSION: &str = vertical_orientation_data::UNICODE_VERSION;

/// The four values of the Unicode `Vertical_Orientation` (`vo`) property.
///
/// These are scalar properties from [UAX #50](https://www.unicode.org/reports/tr50/),
/// not final glyph orientations. In particular, `Tr` prefers a typographically
/// transformed upright form; rotation is only the fallback without that form.
/// Do not reduce this enum to an unconditional rotation flag.
///
/// Unicode の `Vertical_Orientation` の 4 値です。文字単位のプロパティであり、
/// 最終的なグリフの向きではありません。特に `Tr` は縦書き用の字形を優先し、
/// その字形がない場合にのみ回転して表示する値です。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VerticalOrientation {
    /// `U`: upright, in the same orientation as the Unicode code charts.
    ///
    /// `U`: Unicode コード表と同じ正立の向きです。
    Upright,
    /// `R`: rotated 90 degrees clockwise relative to the Unicode code charts.
    ///
    /// `R`: Unicode コード表の向きから時計回りに 90 度回転します。
    Rotated,
    /// `Tu`: use a typographically transformed vertical form; if unavailable,
    /// fall back to the code-chart glyph upright.
    ///
    /// `Tu`: 縦書き用の字形を使い、なければコード表の字形を正立で表示します。
    TransformWithUprightFallback,
    /// `Tr`: use a typographically transformed upright vertical form; if
    /// unavailable, fall back to the code-chart glyph rotated 90 degrees clockwise.
    /// This value does not mean that a transformed glyph should be rotated.
    ///
    /// `Tr`: 正立の縦書き用字形を使い、なければコード表の字形を時計回りに
    /// 90 度回転します。縦書き用字形そのものを回転する意味ではありません。
    TransformWithRotatedFallback,
}

/// Returns a scalar's Unicode `Vertical_Orientation` (`vo`) property.
///
/// Uses the pinned [`VERTICAL_ORIENTATION_UNICODE_VERSION`] data, including its
/// unassigned-code-point and private-use ranges. Any scalar not explicitly
/// listed in the Unicode data has the default property `R` ([`Rotated`]).
/// The lookup is allocation-free, needs no font, and works without any features.
///
/// This query does **not** resolve grapheme clusters, combining sequences,
/// variation selectors, emoji sequences, writing modes, or font substitutions.
/// It does not change shaping or choose automatic mixed vertical layout.
/// Callers must resolve those separately before assigning glyph orientations.
///
/// 指定した Unicode スカラー値の縦書きプロパティを返します。未割当文字・
/// 私用文字も固定バージョンのデータに従い、省略された値は `R` になります。
/// メモリ確保やフォント、機能フラグは不要です。書記素クラスタや結合文字列、
/// 異体字セレクタ、絵文字列、字形置換を解決するものではなく、既存のシェーピングや
/// 混在縦書きのレイアウトを変更しません。
///
/// ```
/// use fontcore::{vertical_orientation, VerticalOrientation as Vo};
///
/// assert_eq!(vertical_orientation('漢'), Vo::Upright); // U
/// assert_eq!(vertical_orientation('A'), Vo::Rotated); // R
/// assert_eq!(vertical_orientation('、'), Vo::TransformWithUprightFallback); // Tu
/// assert_eq!(vertical_orientation('ー'), Vo::TransformWithRotatedFallback); // Tr
/// ```
///
/// [`Rotated`]: VerticalOrientation::Rotated
#[must_use]
pub fn vertical_orientation(character: char) -> VerticalOrientation {
    let scalar = character as u32;
    let ranges = vertical_orientation_data::RANGES;
    match ranges.binary_search_by(|&(start, end, _)| {
        if end < scalar {
            std::cmp::Ordering::Less
        } else if start > scalar {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }) {
        Ok(index) => ranges[index].2,
        Err(_) => VerticalOrientation::Rotated,
    }
}
