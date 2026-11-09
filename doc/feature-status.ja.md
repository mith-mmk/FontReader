# 機能実装メモ

このファイルは、以前 `README.ja.md` に置いていた実装メモ寄りの内容を移したものです。
README は公開APIと実行例を優先し、細かい対応状況はここにまとめます。

## Layout 対応状況

`layout` feature は一部のみ実装されています。

### 縦メトリクスのフォールバック

`FontEngine::shape`、計測、SVGは、vhea/vmtxの組がないfaceでも上から下への流れを保ちます。
縦advanceは横メトリクスのascenderからdescenderを引いた値を使い、正でない場合はunits per emを使います。
既存のfont-size倍率を一度だけ適用し、横stretchは縦advanceに影響しません。
vheaだけがある場合も同じフォールバックです。
横advanceが0のグリフは、attachment表のない結合記号やゼロ幅文字も含めて縦advanceを0のままにします。
両表がある場合のvmtx advanceは0も含めて保持します。
`layout` featureなしではsfnt loaderが縦表を読まないため、同じフォールバックを使います。

合成fixtureで縦メトリクスの有無とコレクション内のface選択を検証し、外部フォントを必要としません。
このfallbackとは別に下記のTrueType縦原点補正を行います。Unicode縦方向、約物の回転は別対応です。静的CFFの原点は下記を参照してください。

### TrueTypeの可変縦メトリクス

縦phantom pointは既定の`vmtx`から作り、`gvar`と`VVAR`の同じ変化を二重適用しません。
有効な`VVAR`のadvance/TSBエントリを項目ごとに優先し、省略されたTSB mapでは
輪郭boundsの変化も含む`gvar`由来のTSBを維持します。明示されたゼロdeltaも優先します。
TrueTypeでは`VVAR`の縦原点mapを使いません。

[OpenType VVAR仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/vvar)を参照してください。
不一致データの優先順位は実装の決定的な選択方針であり、適合するフォント作成例や
仕様が定めた競合解決規則という意味ではありません。MITの合成fixtureで軸の既定値・中間値・
最大値、任意map、TTC face、shape・measure・描画boundsを検証します。
この修正は縦方向のみです。横方向の`HVAR`/`gvar`の相互作用は変更・検証していません。

### TrueTypeの縦原点

モノクロTrueTypeの描画layerは、横advanceの半分をx原点、
`glyf.yMax + vmtx.topSideBearing`をy原点として配置します。
基準は[OpenType vmtx仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/vmtx)です。
縦メトリクスがなければ合成セルの縦中央に輪郭を置きます。
layer offsetに倍率を一度だけ適用し、論理的な列位置とadvanceは維持します。
boundsとSVGも同じlayerを使い、GPOS/GDEFで付くmarkはbaseの原点移動量を継承します。

合成fixtureで約物、負のbearing、合成glyph、gvarの輪郭/phantom移動、TTC face選択、sfnt/WOFFを検証します。
BASE、Unicode縦方向、下記の静的COLR v0以外の色layer原点は別対応です。
旧raw glyph/SVG APIは変更しません。TrueTypeのVORGは仕様どおり無視します。

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
- path 化できない payload と、`pattern` / `mask` / `filter` など未対応構文を含む payload は `GlyphLayer::Svg` fallback として保持する
- 現状の対応範囲は `path` / `rect` / `circle` / `ellipse` / `line` / `polyline` / `polygon`、`defs` / `use`、`fill` / `fill-rule` / `stroke` / `stroke-width`、`clipPath` / `clip-path`、単純な `mask`、`translate` / `scale` / `rotate` / `skewX` / `skewY` / `matrix`、`linearGradient` / `radialGradient` / `stop`、`gradientUnits` / `gradientTransform` の保持
- pattern / mask / filter、複雑な stroke style は未対応
- `paintcore` 側でも clip / gradient renderer を使える前提で bridge が追随した
- WOFF2 は完全な byte stream がそろってから decode する前提
- CFF2 の事前調査メモは `cff2-investigation.ja.md` に配置

### 静的CFFの縦原点

高水準のshape・計測・SVG出力では、単色の静的CFFにVORGの既定値または
glyph別の符号付きY原点を適用します。X方向は横advanceの半分だけ移動し、
置換後のglyph IDを使います。結合markはbaseの移動を継承します。
advanceは従来どおりvhea/vmtxから求め、VORGでは変更しません。

VORGがない場合は、三次曲線の正確なyMaxとvmtxのtop side bearingを使います。
縦metricsもない場合はTrueTypeと同じ合成セル中央へ配置します。空輪郭の極値は0です。
公開ink boundsは従来の保守的な制御点込みの範囲のままです。
sfnt・選択したTTC face・WOFFに対応します。不正な任意VORGは無視してfallbackし、
TrueTypeではVORGを使いません。`cff`が必要で、sfntの縦metricsには従来どおり`layout`も必要です。
下記の静的COLR v0以外の色layer原点、BASE、Unicode縦方向、横書き・旧raw APIは今回の対象外です。
MIT合成fixtureとSHA-256は`tests/fixtures/manifest.toml`を参照してください。

### CFF2の可変縦書き原点

`cff2`有効時、単色CFF2にも同じ高水準原点処理を適用します。VORGの値を
既定インスタンスの原点とし、選択した正規化座標で明示的にmapされたVVAR原点差分だけを
加算します。原点mapがなければ差分は0です。advance用の暗黙mapは使いません。
VORG由来の原点にTSB差分を重ねて加算しません。TrueTypeでは両方の原点tableを無視します。

有効なVORGがなければ、座標変動を適用したCFF2三次曲線の実際のyMaxと
vmtx/VVARの変動後top side bearingを使います。このfallbackに原点map差分は加算しません。
縦metricsもなければ変動後の輪郭極値を合成セルの中央に配置します。
metricsの丸めと範囲制限は既存処理に従い、公開ink boundsは制御点を含む従来の範囲です。

既定・中間・最大座標、選択TTC face、WOFF、結合mark、横伸縮0を含む倍率を検証します。
検証はMIT合成fixtureによるもので、実在CFF2縦書きフォント群での検証ではありません。
不正な任意VORGはfallbackしますが、CFF2/VVAR全般の破損回復、色原点、BASE、
Unicode縦方向、旧raw APIは拡張しません。

仕様根拠: [VORG](https://learn.microsoft.com/en-us/typography/opentype/spec/vorg)、
[VVAR](https://learn.microsoft.com/en-us/typography/opentype/spec/vvar)。
fixtureの出典・再生成方法・SHA-256は`tests/fixtures/manifest.toml`に記載しています。

### 静的TrueType COLR v0の縦原点

`layout`とvhea/vmtxがある場合、実際に描画したCOLR v0/CPALの全path layerへ、
base glyphの横advanceの半分と`glyf.yMax + vmtx.topSideBearing`による原点を適用します。
空のbaseはyMaxが0なのでTSBが原点そのものです。色layerのunion上端へTSBを足しません。
これは[COLRのglyph metrics要件](https://learn.microsoft.com/en-us/typography/opentype/spec/colr#glyph-metrics-and-boundedness)に従います。
paintとlayer構成を保ち、付着した単色・色markはbaseの移動量を継承します。
文字サイズ・横伸縮は一度だけ適用し、bounds・計測・SVGへ反映します。横書き・raw APIは変更しません。

fvarを持つフォント（色layerはまだ変形しません）、COLR v1、CFFの色outline、sbix、SVGは対象外です。
縦metricsなし、または`layout`無効時は従来のCOLR offsetを維持し、描画unionを使う中央配置は別対応です。
CPAL欠落や利用できないpaletteで単色へfallbackする場合も、COLR metadataだけでは新しい原点補正を適用しません。
MITの再現可能なfixtureは`colr-vertical-origin.ttc`です。生成scriptとSHA-256はmanifestに記録しています。

### 従来方式の縦書きフィーチャー選択

`lookup_vertical()` は、既存の収集処理で有効な参照先 Lookup が一つ以上
見つかる場合に `vrt2`、それ以外は `vert` を選択する。空のフィーチャーや
無効な Lookup インデックスだけを持つフィーチャーは利用不可とする。
`vrt2` 選択後に対象グリフの coverage がなくても `vert` へ戻らない。
サブテーブルが空の Lookup も同様。LangSys から参照されないレコードは
選択対象に含めない。描画側でグリフを回転しないため `vrtr` は適用しない。
[OpenType vrt2 仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrt2)
の回転済み字形とフィーチャーの排他性に従う。

既存の探索範囲は維持する。全 Script の既定・名前付き LangSys と必須
フィーチャーを含み、DFLT 以外だけを持つフォントも対象となる。
Script run ごとの選択や Unicode の混在方向は未対応。
高水準のテキスト整形では cmap/UVS を先に解決し、通常の GSUB 段階
（合成、locale/variant、RTL 字形、合字）の後に選択した縦書きフィーチャーを
適用する。メトリクス、輪郭の原点、GPOS は最終的なグリフ ID を使う。
raw の `get_glyph_with_uvs_axis()` はテキスト整形を
行わず、従来どおり直接縦書き字形へ置換する。既存の単一フィーチャー内の
Lookup 連鎖処理やソースクラスタ表現は変更しない。
横書きと `layout` 無効時は変更なし。MIT の合成フォントで選択結果、
最終字形の bounds・送り量、measure、SVG を検証する。
出典とハッシュは `tests/fixtures/manifest.toml` を参照。
`vertical-gsub-order.ttc` は合成、variant、locale、合字、UVS、最終字形の
マークアンカーと原点、選択 face の fallback、raw グリフ互換性を検証する。
