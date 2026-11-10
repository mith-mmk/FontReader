//! Unicode character properties, independent of fonts and shaping.
//!
//! フォントやシェーピングに依存しない Unicode 文字プロパティです。

pub(crate) mod graphemes;
mod vertical_orientation_data;

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
