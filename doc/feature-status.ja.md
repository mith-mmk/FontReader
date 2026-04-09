# 機能実装メモ

このファイルは、以前 `README.ja.md` に置いていた実装メモ寄りの内容を移したものです。
README は公開APIと実行例を優先し、細かい対応状況はここにまとめます。

## Layout 対応状況

`layout` feature は一部のみ実装されています。

### GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- 部分実装: `lookup_ccmp()` はあるが結果展開は未完
- 実装済み: `lookup_locale()`, `lookup_liga()`
- text API では variation selector と基本的な `locl` / `liga` / `dlig` / `ccmp` を適用
- 方向指定 API で縦書きと RTL を扱う
- Arabic shaping は `isol` / `init` / `medi` / `fina` に対応
- Arabic shaping では `rlig`, `rclt`, `calt`, `clig` も存在すれば適用
- locale/script に応じた lookup 選択を行う
- language system 選択では `ur-Arab-PK` のような full locale subtag も見る
- 日本語 variant form は `FontOptions::font_variant` から要求可能
- Context / Chaining は feature-sequence 適用器経由で部分対応
- 未実装: `lookup_width()`, `lookup_number()`

### Lookup パース

- Type 1 Single Substitution: パース済み、展開可能
- Type 2 Multiple Substitution: パース済み、展開可能
- Type 3 Alternate Substitution: パース済み、展開可能
- Type 4 Ligature Substitution: パース済み、展開可能
- Type 5 Context Substitution:
  - Format 1 パース済み、部分適用可能
  - Format 2 パース済み、部分適用可能
  - Format 3 パース済み、適用可能
- Type 6 Chaining Context Substitution:
  - Format 1 パース済み、部分適用可能
  - Format 2 パース済み、部分適用可能
  - Format 3 パース済み、適用可能
- Type 7 Extension Substitution: パース済み、完全適用は未完
- Type 8 Reverse Chaining Contextual Single Substitution: パース済み、未適用

### GDEF

- パース済み: glyph class definition, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- 現状: 部分統合
- Pair positioning では、GDEF の mark glyph を見て前後の spacing glyph 探索時に mark をスキップするようにした
- `GPOS mark-to-base` (Type 4 Format 1) はパースと shaping への統合まで対応した
- `GPOS mark-to-mark` (Type 6 Format 1) もパースと shaping への統合まで対応した
- `mark` / `mkmk` feature から anchor が取れない場合だけ、既存の GDEF ベース fallback を使う
- attach / caret / mark-set 系のデータはまだ上位 layout に完全統合できていない

## 補足

- `FontFamily` は cached face 選択と glyph 単位 fallback に対応
- family fallback chain と Last Resort は未実装
- variable font の metadata と axis 依存 metrics は `fvar` / `avar` / `HVAR` / `VVAR` / `MVAR` まで対応
- 公開API では `FontFace::variation_axes()` と `FontEngine::with_variation()` から使える
- `gvar` は simple glyph に加えて composite glyph の outline delta まで実装済みで、Source Serif variable-font fixture を含めて回帰確認している
- `gvar` phantom point delta は horizontal / vertical の glyph metrics に反映するようにした
- phantom point の挙動は synthetic unit test と Source Serif の real-font regression で確認している
- parser hardening を進めており、GSUB/GPOS の壊れた optional feature variation は panic せず読み飛ばすようにした
- GSUB の未展開 contextual subtable は shaping 中に panic せず `LookupResult::None` として処理を継続する
- `hmtx` / `vmtx` は 0-metric の edge case でも panic せず、advance 0 を fallback として返す
- `OTFHeader` / `TTCHeader` / `get_font_type()` / `COLR::new()` は短い入力でも panic せず error を返すようにした
- `COLR::get_layer_record()` は壊れた layer range を信じ切らず、存在する layer までで停止する
- `fontreader.rs` では name lookup / metrics・layout getter / mandatory table load の `unwrap()` をさらに減らした
- CFF2 outline は共有した `cff.rs` 経路で読み込み、`vsindex` / `blend` を含む charstring 評価まで対応した
- CFF2 variation は outline charstring に加えて Private DICT の `vsindex` / `blend` parser まで対応した
- ただし現在の local corpus には true CFF2 実フォントが確認できず、coverage は実フォント smoke より synthetic / unit test 寄り
- `svg-fonts` 有効時は OpenType `SVG ` glyph の path 化を優先し、単純 shape は `GlyphLayer::Path` に変換する
- path 化できない payload は `GlyphLayer::Svg` として保持する
- 現状の対応範囲は `path` / `rect` / `circle` / `ellipse` / `line` / `polyline` / `polygon`、`defs` / `use`、`fill` / `fill-rule` / `stroke` / `stroke-width`、`clipPath` / `clip-path`、`translate` / `scale` / `matrix`、`linearGradient` / `radialGradient` / `stop`、`gradientUnits` / `gradientTransform` の保持
- pattern / mask / filter、複雑な stroke style、`paintcore` 側の clip/stroke 実描画は未対応
- WOFF2 は完全な byte stream がそろってから decode する前提
- CFF2 の事前調査メモは `cff2-investigation.ja.md` に配置
