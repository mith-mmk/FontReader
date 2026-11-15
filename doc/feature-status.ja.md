# 機能実装メモ

このファイルは、以前 `README.ja.md` に置いていた実装メモ寄りの内容を移したものです。
README は公開APIと実行例を優先し、細かい対応状況はここにまとめます。

## Unicode の縦書きプロパティ

`vertical_orientation(char)` は Unicode 18.0.0 の `Vertical_Orientation` を
4 値 (`U`、`R`、`Tu`、`Tr`) で返し、フォントや機能フラグを必要としません。
`Tr` は正立の縦書き用字形を優先し、その字形がない場合にのみ回転します。
文字単位の問い合わせであり、書記素クラスタや字形置換を解決せず、既存の shaping も
変えません。混在縦書きは通常の shaping 方針ではなく、下記の
[明示的な混在縦書き API](#明示的な混在縦書き-ltr-アウトライン)で利用できます。
[使用例](api-recipes.ja.md#unicode-の縦書きプロパティを調べる)と
[データの出典](../data/unicode/README.md)を参照してください。

## Layout 対応状況

`layout` feature は一部のみ実装されています。

### 基本的なチベット文字の母音アンカー

`layout` 有効時、隣接する未合字のチベット子音（U+0F40–U+0F6C、
未割当の U+0F48 を除く）と単純母音（U+0F71、U+0F72、U+0F74、
U+0F7A–U+0F7D、U+0F80）に限定した GPOS fallback を行います。
既存の `mkmk` / `mark` attachment を優先し、一致がない場合だけ
明示的な `tibt` script の `abvm` / `blwm` MarkToBase を検索します。
locale の言語系選択は保持し、`DFLT` や無関係な script には広げません。
GDEF lookup filtering に従い、必要な GDEF 情報がない flagged lookup は
安全側に倒して適用しません。アンカー差分には既存の画面座標変換と倍率を適用します。

完全なチベット文字 shaping ではありません。積み重ね、分解可能な母音の正規化、
並べ替えや複数記号列、script itemization、GSUB feature plan は未対応です。
`bo` / `dz` から `tibt` へのグローバル GSUB locale mapping も変更しません。
下母音の一部は `blws` 置換を必要とし、単独の GPOS アンカーが正しくても
完全な HarfBuzz shaping とは異なります。コミット済みの合成テストに外部フォントは不要です。

### 縦メトリクスのフォールバック

`FontEngine::shape`、計測、SVGは、vhea/vmtxの組がないfaceでも上から下への流れを保ちます。
縦advanceは横メトリクスのascenderからdescenderを引いた値を使い、正でない場合はunits per emを使います。
既存のfont-size倍率を一度だけ適用し、横stretchは縦advanceに影響しません。
vheaだけがある場合も同じフォールバックです。
横advanceが0のグリフは、attachment表のない結合記号やゼロ幅文字も含めて縦advanceを0のままにします。
両表がある場合のvmtx advanceは0も含めて保持します。
`layout` featureなしではsfnt loaderが縦表を読まないため、同じフォールバックを使います。

合成fixtureで縦メトリクスの有無とコレクション内のface選択を検証し、外部フォントを必要としません。
このfallbackとは別に下記のTrueType縦原点補正を行います。この従来フローは向きを判定しません。混在縦書きは下記の明示 API を使用します。静的CFFの原点は下記を参照してください。

### TrueTypeの可変横メトリクス

横phantom pointは選択faceの既定`hmtx`から構築します。`HVAR`と`gvar`は代替の
メトリクス情報であり、有効なHVARのadvance/LSBを項目ごとに優先して二重加算を防ぎます。
任意LSB mapの省略時はgvar由来のbearingを保持し、明示されたゼロは有効な値として扱います。
gvar tupleが適用されない場合もHVARを利用します。`layout` featureなしでも動作します。

[OpenType HVAR仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/hvar)を参照してください。
不一致tableのfixtureは実装の選択方針を検証するもので、準拠フォントや仕様上の競合解決規則ではありません。
MIT fixtureで暗黙・明示map、TTC face、phantomのみのtuple、符号付き・小数delta、
拡大縮小・横stretch、measure/SVG boundsを検証します。範囲外の飽和テストは堅牢性の診断です。

複合グリフのtupleは、展開後の輪郭点ではなくcomponent番号とその後の4つのphantom pointを使います。
componentのXY deltaはoffsetの拡大縮小より前に適用し、省略deltaはゼロとします。輪郭用IUPは行いません。
既定phantomは元の複合グリフboundsと選択faceの`hmtx`/`vmtx`から作り、変化後のboundsでbearingを計算します。
親にtupleがなく子だけが変化する場合も対象です。項目ごとのHVAR/VVAR優先順位は維持します。
入れ子componentの小数offsetは最外側の`ParsedGlyph`変換まで保持します。この未hint輪郭経路では
`ROUND_XY_TO_GRID`をdesign unit単位の丸めとして適用しません。公開された整数輪郭表現と
simple glyphの丸めは変更していないため、font unit未満の精度全般を保証する修正ではありません。

専用MIT fixtureとfontTools 4.68.0の独立したinstance生成結果で、XY移動、入れ子、均等拡大縮小されたoffset、
疎なtuple、符号付き・小数phantom delta、子のみの変化、TTC face選択、raw getter、shape、
拡大縮小・横stretch、measure/SVG boundsを検証します。左phantomがゼロでないケースと不一致tableは診断用で、
準拠する可変フォントの作成例ではありません。これは1つのface内の複合グリフで、複数フォントのfallbackとは別です。
[複合グリフのgvar仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/gvar#point-numbers-and-processing-for-composite-glyphs)を参照してください。

制限：点同士で接続する複合グリフは従来の未変形輪郭経路を使い、正しい点の対応付けは未実装です。
`USE_MY_METRICS`のphantom継承・hintingも未実装のため、親自身のphantom deltaは無視し、
`hmtx`/HVARと`vmtx`/VVARのfallbackを保持します。対応するXY輪郭は変形します。
fallbackテストはこの境界を検証するもので、仕様準拠を示しません。任意の回転・斜行を含む複合グリフは今回の検証範囲外です。

### TrueTypeの可変縦メトリクス

縦phantom pointは既定の`vmtx`から作り、`gvar`と`VVAR`の同じ変化を二重適用しません。
有効な`VVAR`のadvance/TSBエントリを項目ごとに優先し、省略されたTSB mapでは
輪郭boundsの変化も含む`gvar`由来のTSBを維持します。明示されたゼロdeltaも優先します。
TrueTypeでは`VVAR`の縦原点mapを使いません。

[OpenType VVAR仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/vvar)を参照してください。
不一致データの優先順位は実装の決定的な選択方針であり、適合するフォント作成例や
仕様が定めた競合解決規則という意味ではありません。MITの合成fixtureで軸の既定値・中間値・
最大値、任意map、TTC face、shape・measure・描画boundsを検証します。
縦方向のfixtureは上記の横方向の検証とは別です。

### TrueTypeの縦原点

モノクロTrueTypeの描画layerは、横advanceの半分をx原点、
`glyf.yMax + vmtx.topSideBearing`をy原点として配置します。
基準は[OpenType vmtx仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/vmtx)です。
縦メトリクスがなければ合成セルの縦中央に輪郭を置きます。
layer offsetに倍率を一度だけ適用し、論理的な列位置とadvanceは維持します。
boundsとSVGも同じlayerを使い、GPOS/GDEFで付くmarkはbaseの原点移動量を継承します。

合成fixtureで約物、負のbearing、合成glyph、gvarの輪郭/phantom移動、TTC face選択、sfnt/WOFFを検証します。
BASE と下記の静的COLR v0以外の色layer原点は別対応です。向きは従来の原点補正で判定せず、明示混在 API が指定します。
旧raw glyph/SVG APIは変更しません。TrueTypeのVORGは仕様どおり無視します。

### GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- 部分実装: `lookup_ccmp()` はあるが結果展開は未完
- 実装済み: `lookup_locale()`, `lookup_liga()`
- text API では variation selector と基本的な `locl` / `liga` / `dlig` / `ccmp` を適用
- 方向指定 API で縦書きと RTL を扱う
- Arabic-style RTL joining は Unicode Joining_Type から `isol` / `init` / `medi` / `fina` を選択（制限は下記）
- LTR/RTL 両方の text shaping で `rlig`, `rclt`, `calt`, `clig` を存在すれば適用
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

### GPOS Single Positioning

- Type 1 SinglePos Format 1/2 と、それを包む Type 9 をパースし、実行する。
- 既定の実行対象は横組み `kern` / `dist`、縦組み `vkrn` / `dist`。既存の locale / script 選択を利用し、任意 feature 選択や contextual GPOS は追加しない。
- 静的 ValueRecord の4値を shaping と測定へ反映する。付着する mark の明示 advance も保持し、lookup 間は i32 で加算する。同一 lookup ではゼロ値を含む最初の一致 subtable だけを適用する。
- SinglePos は GDEF の glyph class、IgnoreBaseGlyphs / IgnoreLigatures / IgnoreMarks、MarkAttachmentType、MarkFilteringSet を尊重する。PairPos もこの filtering を共有する。mark attachment は従来の別経路を維持する。
- Device / VariationIndex offset は読み進めるが delta は未評価。筆記体接続と完全な Arabic shaping は別の残件。
- MIT の合成フォント `tests/fixtures/gpos-single.ttf` と byte fixture により、切断入力、不正 coverage / count、lookup 加算、locale、拡縮、縦組み、mark 付着を検証する。広範な実フォント / HarfBuzz 比較は未実施。
- 仕様: [OpenType GPOS Single Adjustment](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-1-single-adjustment-positioning-subtable)。

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
- `gvar` は simple glyph とXY配置のcomposite glyphの輪郭delta・再帰的component変形に対応。点接続とメトリクス継承には上記の制限がある
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
選択対象に含めない。この従来の正立配置経路では描画側でグリフを回転しないため、
`vrtr` は適用しない。
[OpenType vrt2 仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrt2)
の回転済み字形とフィーチャーの排他性に従う。

既存の探索範囲は維持する。全 Script の既定・名前付き LangSys と必須
フィーチャーを含み、DFLT 以外だけを持つフォントも対象となる。
Script run ごとの選択や Unicode の混在方向は未対応。
高水準のテキスト整形では cmap/UVS と従来の locale 前処理を先に解決する。
LTR は選択した縦書きフィーチャーと通常の GSUB を LookupList 順に各一回適用する。
RTL は合成・variant と joining の境界を維持し、その後に文脈・合字・方向の
置換を行う。メトリクス、輪郭の原点、GPOS は最終グリフ ID を使う。
raw の `get_glyph_with_uvs_axis()` はテキスト整形を
行わず、従来どおり直接縦書き字形へ置換する。ソースクラスタ表現は変更しない。
縦書きフィーチャー内の Lookup 連続適用は次節で説明する。
`layout` 無効時は変更なし。MIT の合成フォントで選択結果、
最終字形の bounds・送り量、measure、SVG を検証する。
出典とハッシュは `tests/fixtures/manifest.toml` を参照。
`vertical-gsub-order.ttc` は合成、variant、locale、合字、UVS、最終字形の
マークアンカーと原点、選択 face の fallback、raw グリフ互換性を検証する。


### 従来方式の縦書き Lookup の連続適用

選択済みの縦書きフィーチャー内では、Single 置換（format 1/2）と
Extension で包まれた Single 置換を LookupList 順に各一回適用する。
Feature 内の参照順が逆でも、複数の LangSys から参照されても順番と回数は
変わらない。前の Lookup の出力を次の入力とし、各 Lookup は最初に
適用できたサブテーブルで終了する。同一 ID への置換も適用として扱い、
循環して元の ID に戻った場合も `Some(original_id)` を返す。未適用は `None`。

単一グリフ API では Multiple・Alternate・Ligature・文脈依存・逆方向文脈依存の
Lookup と、それらを包む Extension はスキップする。複数出力の先頭だけを
採用したり、文脈条件を無視したりしない。未対応・空の Lookup を理由に
グリフ単位で `vert` へ戻ることもない。単一グリフ API の全 Script/LangSys 探索範囲は維持する。
text API は後述の列処理を使う。単一グリフ API 自体は引き続き限定的である。

根拠は [OpenType 共通レイアウト仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#feature-table)
と [GSUB 置換形式](https://learn.microsoft.com/en-us/typography/opentype/spec/gsub)。
MIT 合成フォント `vertical-lookup-chain.ttc` で順序・恒等置換・循環・重複参照・
未対応形式と、最終 bounds/原点・送り量・計測・SVG を検証する。
生成スクリプトと SHA-256 は manifest に記録する。

この単一グリフ API は GDEF のクラス・マークフィルタ情報を受け取らないため、
IgnoreBaseGlyphs / IgnoreLigatures / IgnoreMarks / UseMarkFilteringSet /
MarkAttachmentType を指定する Lookup は全体をスキップする。これは保守的な
未対応扱いであり、GDEF による完全な選別処理ではない。対象外グリフへの誤置換を
防ぎ、選択済みの `vrt2` から `vert` へのフォールバックも行わない。
RightToLeft は OpenType 仕様上 GSUB では無意味なので、置換を妨げない。
直接・Extension の各形式で、単独および連続 Lookup とマーク集合を検証する。

### Sideways LTR の run

`FontEngine::shape_sideways_ltr`・`measure_layout_sideways_ltr`・
`measure_sideways_ltr`・`render_svg_sideways_ltr` は、単一 face・単一行を扱う
明示的な API である。横書き LTR として整形した後、run 全体を時計回りに 90 度回転する。
engine の LTR・RTL・上から下という設定は呼び出しの間だけ上書きし、engine や
options 自体は変更しない。style・locale・variant・対応済みの可変軸座標・
文字サイズ・横伸縮・SVG 単位を保持する。永続的なモード、必須の公開フィールド、
公開 enum の variant は追加せず、既存の汎用 API と旧 raw API も変更しない。

`layout` 有効時は `vrtr` と通常の LTR GSUB を LookupList 順に適用し、
最終グリフ ID をメトリクスと GPOS に使う。
sideways 呼び出しでは `vert` と `vrt2` を選択せず、`vrtr` の置換がない場合や
未対応の場合にも、いずれのフィーチャーへも fallback しない。`layout` 無効時も
回転は行うが、OpenType layout による置換と位置調整は利用できない。

`vrtr` の収集範囲は従来の全 Script・全 LangSys を維持する。単一グリフの
Lookup 連続適用では、直接または Extension 内の Single 置換を LookupList 順に
参照先ごとに一回適用する。列全体の処理が必要な置換や GDEF に依存するフィルタ
フラグは、上記の従来方式と同じくスキップする。通常の横書き整形が持つ
GSUB/GPOS の列処理・フィルタリングの制限も残る。pair-GPOS の `yPlacement` は、
通常の横書き・正立縦書きと同じく、上向きが正のフォント座標から下向きが正の run 座標へ
変換してから sideways 回転する。`yAdvance` の既存動作は変更しない。
完全な OpenType 整形や Unicode の混在方向処理ではない。

送り量と原点は横メトリクスを使い、縦組み用の `vmtx` や `VORG` は使わない。
y が下向きの座標系で、配置位置・path・offset・送り量ベクトルを
`(x, y) -> (-y, x)` と変換し、ベースライン原点 `(0, 0)` を中心に回転する。
em 中央への移動は行わず、bounds も回転する。フォントの ascent・descent・line gap
は横組みの値を交差軸の参照値として保持し、`FontMetrics::flow` を
`GlyphFlow::Vertical` にする（ascent 側が右、descent 側が左）。従来の bearing は
LSB と上向きを正とする ascender の参照値なので、
`(bearing_x, bearing_y) -> (bearing_y, -bearing_x)` と変換する。描画位置そのもの
ではないため、ink の配置には変換後の bounds を使う。

計測と SVG は同じ変換後の run を使う。`inline_advance` は y 方向送り量の合計の
絶対値であり、`measure_sideways_ltr` はこのスカラー値を返す。`ink_bounds` は
配置済みのグリフ bounds の和集合である。`line_bounds` は各配置原点と、回転後の
送り量ベクトルの両成分を足した終点の和集合となる。`block_extent` は
`line_bounds` の x 方向の幅であり、フォントの行高ではない。空の run の bounds は
なく、各 extent は 0 となる。空白だけの run も横送り量を回転した送り量を保持する。
SVG は通常の ink padding と engine の SVG 単位を使う。

対応可否はフォント形式の許可リストではなく、実際の layer で判定する。
すべての layer が clip のない `GlyphLayer::Path` で、paint が
`GlyphPaint::CurrentColor`、描画モードが `PathPaintMode::Fill` なら対応する。
条件を満たす path 化済み SVG や foreground のみの COLR layer も含まれる。
単色指定・gradient・stroke・clip・raster・raw SVG layer は
`std::io::ErrorKind::Unsupported` となり、部分的な出力は返さない。
TAB・CR・LF・VT・FF・NEL・U+2028・U+2029 は `std::io::ErrorKind::InvalidInput`
となる。family fallback・BiDi・Unicode の方向や向きの自動判定・タブ・複数行・
正立 CJK のベースラインとの位置合わせは、この API の対象外である。

描画側の回転と専用代替字形の関係は
[CSS Writing Modes の vertical font features](https://www.w3.org/TR/css-writing-modes-4/#vertical-font-features)
と [OpenType `vrtr`](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrtr)
を参照する。これらを根拠に CSS layout 全体への対応を示すものではない。
公開 API の使用例は [APIレシピ](api-recipes.ja.md#sideways-ltr-の整形計測svg-出力) に記載する。

## ZWJ に隣接するレイアウト制御文字

共通テキストパーサーは U+200D ZERO WIDTH JOINER の直後でも LF と TAB を
レイアウト制御文字として扱う。CR は従来どおり無視するため、CRLF は改行1回に
なる。横書き・正立縦書き・ファミリーフォールバックに共通で、LF は次の行・列へ
移動し、TAB は行高の4倍だけ進む。制御文字が欠落グリフの送りやフォールバック用
クラスタに混ざることを防ぐ。グリフ送りの合計とファミリーの最大行長という
既存の計測仕様は変更しない。

既存の CR/LF/TAB の扱いを維持する。拡張書記素クラスタの分割は後述のとおりで、
それ以外の Unicode 制御文字を新しく改行扱いにすることはない。テストは同梱の
MIT フォントを使用する。

## ペア位置調整の座標

GPOS PairPos Format 1・2 の符号付きフォント単位レコードは変更しない。
高水準 run では両グリフのスケール済み `yPlacement` を一度だけ下向きが正の座標へ
変換し、ペン位置または付加先原点へ加える。正の配置調整は LTR・RTL・正立 TTB の
インクを上へ移動する。sideways は修正済みの横書き配置を回転する。
X 配置・送り量、mark attachment、従来の raw command API は変更しない。

送り量は配置の直交座標ベクトルとは異なり、組方向に依存する。
[OpenType GPOS 仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#overview)
は、正の `YAdvance` が上から下への送り高さを増やすと明記している。
下向きが正の `vmtx` 送り量へ加算し、符号は反転しない。
横書きの交差軸送り量と、通常の line bounds が組方向の終点のみを見る既存動作は
維持する。GPOS の軸適用条件や列処理全体への完全対応を主張するものではない。

MIT の `pair-coordinates.ttc` は両 PairPos 形式、両グリフの正・負・ゼロの Y 値、
異なる横・縦メトリクス、サイズ・stretch、raw 値、LTR・RTL・TTB の位置と送り量、
インク・行計測、SVG、sideways 回転を検証する。再生成は fontTools 4.65.0 で
`python tests/fixtures/generate_pair_coordinates.py` を実行する。

## 拡張書記素クラスタの境界

シェーピングと FontFamily のフォールバックは、厳密に固定した
`unicode-segmentation = "=1.13.3"` による **Unicode 17.0.0** の拡張書記素
クラスタを共用する。手書きの結合文字範囲と無条件の ZWJ 連結を置き換え、
Hangul L/V/T、SpacingMark、Prepend、インド系文字の結合（GB9c）、絵文字 ZWJ、
地域指示記号のペアを UAX #29 に従って分割する。`A + ZWJ + B` は
`A + ZWJ` と `B` の2単位になる。地域指示記号の間に結合文字がある場合も、
その文字をまたいでペアにしない。

レイアウト用の調整では、単独の CR を無視し、CRLF または LF は改行1回、
TAB はタブとして扱う。NUL・VT・FF・NEL・U+2028・U+2029 等を新しく改行扱いには
せず、従来のスカラー単位のグリフ処理を保つ。先頭の孤立した異体字セレクターは
連続していても無視するが、同じクラスタに続く結合文字は落とさない。
基底文字＋セレクターの UVS 検索と、ZWJ／欠落グリフの処理は変更しない。

フォールバックは調整後のクラスタ全体を1つの face で処理する。全体をカバーする
候補がない場合は従来どおり先頭候補とその .notdef を使用し、複数の不完全な
フォントからクラスタを継ぎ合わせない。分割だけでインド系文字や絵文字の完全な
シェーピング・単一グリフ化・正規化・BiDi が実現するわけではない。
旧 HTML ヘルパーも2文字のクラスタをすべて基底文字＋セレクターと見なさず、
全スカラーを残す。複数スカラー用の既存 SVG 経路と縦書きの制約は維持する。

公開の `Vertical_Orientation` スカラープロパティは別に **Unicode 18.0.0**
へ固定されている。分割処理自体は方向選択、出典追跡、CSS 組版を提供しない。
下記の明示的な混在 API が方向ごとのグループ化を行う。[出典・ライセンス・オフライン検証](../data/unicode/README.md)を参照。

## 明示的な混在縦書き LTR アウトライン

`FontEngine::shape_vertical_mixed_ltr`、`measure_vertical_mixed_ltr`、
`measure_layout_vertical_mixed_ltr`、`render_svg_vertical_mixed_ltr` は、
単一フェイス・単一行の LTR テキスト用です。呼び出し単位で方向を上書きし、
エンジンの設定や従来 API、公開構造体・方向 enum を変更しません。

分割は **Unicode 17.0 の拡張書記素クラスタ（EGC）**、方向プロパティ VO と
囲み文字カテゴリ Me はそれぞれ **Unicode 18.0** です。GSUB の前に元の EGC の
最初のスカラーの VO で向きを決め、EGC 内に Me（13 文字・5 範囲）があれば
全体を正立にします。U/Tu/Tr は正立、R は横倒し。同じ向きの隣接 EGC をまとめ、
そのグループ内で通常 GSUB、合字、カーニング、マーク配置を行います。
異なる向きの境界を GSUB/GPOS が越えないのは意図した制限です。

正立グループは従来の縦メトリクス・原点を使い、通常 GSUB と **vert のみ**を
LookupList 順に適用します。vrt2/vrtr は選びません。Tr に vert 代替がない場合や同一字形への
置換でも正立のままです。回転フォールバックの合成は未対応です。横倒しは横組み
LTR と **vrtr のみ**を使用し、グループ全体を時計回りに回転します。従来の縦組みの
vrt2 優先・なければ vert という選択は変えません。layout 無効時は同じ分割・
幾何変換を行いますが GSUB/GPOS は適用しません。

横倒しグループの横位置には、そのインスタンスで既にスケールされた水平
FontMetrics の `-(ascent - descent) / 2` を加えます。descent は正の距離です。
既存の対応範囲の可変フォント・MVAR メトリクスを保持し、stretch を再適用しません。
インク境界、line gap、指定行高を中点の代わりにはしません。ただし従来のサイズ
スケーリング自体にはフォントの line gap が関わる場合があります。
これは fontcore 独自の合成で、BASE、OS/2 優先順位、CSS の字号・ベースラインの
同等性を実装するものではありません。中央揃えは配置にのみ加えます。次のグループの
ペンには最終グリフの advance の X/Y 両成分の和だけを渡し、GPOS placement、
最後の描画位置、インク端、中央揃え量は渡しません。既存 GPOS advance の扱いを保持します。

全グループが CurrentColor・fill・clip なし Path である場合だけ成功します。
色、グラデーション、stroke、clip、画像、raw SVG は Unsupported で全体を失敗させ、
部分結果は返しません。エンジンの通常 API はその後も使用できます。TAB/CR/LF/VT/FF/
NEL/U+2028/U+2029 は InvalidInput。空入力は境界なし・長さゼロ、空白は回転した
advance を保持します。計測は最終 run の配置点と advance ベクトルの全終点を使用し、
SVG も同じ run を通常のインク余白・設定単位で出力します。

これは **UAX #50 型の「先頭スカラー＋任意の Me」方針**です。
[CSS Writing Modes](https://www.w3.org/TR/css-writing-modes-4/#mixed-orientation) と
[CSS Text Appendix E](https://www.w3.org/TR/css-text-3/#characters) にある
基底文字などの追加調整を完全に実装するものではありません。Unicode18 EGC、
CSS 完全準拠、ブラウザ同等、BiDi、ファミリーフォールバック、複数行・タブ、
完全な OpenType には未対応です。[UAX #50 revision35](https://www.unicode.org/reports/tr50/tr50-35.html)
と [Unicode の出典・個別バージョン](../data/unicode/README.md) も参照してください。

MIT の固定テストフォントで漢字＋AV＋漢字、符号付き配置・両軸 advance、非対称 hhea、
line gap・stretch・MVAR、Tr の無置換/同一置換/vert、ccmp・複数置換・合字・マーク、
Unicode17/18 の `A + U+1D127` の境界差を検証します。既存 fixture で TT/CFF/CFF2、
縦メトリクス欠落、対応済み可変フォント、カラー拒否も検証します。再生成は
`python tests/fixtures/generate_mixed_vertical_ltr.py`（fontTools 4.65.0）。
通常テストに Python やインストール済みフォントは不要です。

## 明示指定の縦中横 LTR セル

`shape_tate_chu_yoko_ltr`、`measure_tate_chu_yoko_ltr`、
`measure_layout_tate_chu_yoko_ltr`、`render_svg_tate_chu_yoko_ltr` は、
呼び出し側が選んだ単一face・単一行の文字列を横組みし、縦送り1セルにまとめる。
既存の横組みGSUB/GPOS・style・locale・variant・対応済みvariationを維持し、
`vert`・`vrt2`・`vrtr`は選ばない。エンジンの方向や既存APIの挙動は変更しない。

セルの一辺は `units_per_em × 横組みoutlineのスケール` で求めた1em。
現行APIのfont_sizeは横組みの行高入力であり、フォントのemとは必ずしも一致しない。
MVAR対応メトリクスを含め横組みと同じスケールを一度だけ使い、stretchと明示的な
line_heightはセルサイズに影響しない。上中央を原点として送りは `(0, cell_size)`、
論理boundsは `[-cell_size/2, 0, cell_size/2, cell_size]`。各glyphのpath layerを順序・fill ruleを
保ったまま1つの合成glyphへまとめ、元のglyph境界は保持しない。合成font metricsは
ascent/descentが各半セル、line gapが0、flowがVertical、bearingが0となる。
混在縦書きAPIへの自動挿入ではなく、明示的に配置するための基本操作である。

以下のsizeはスケール済みem寸法を指す。横advance合計Wがセル幅を超える場合だけx方向に縮小し、狭い文字列は拡大しない。
x方向はadvanceを中心に配置し、baselineは `size/2 + (ascent-descent)/2` とする。
ascent/descentは横組み後の値で、descentは正。GPOS配置、layer offset、曲線の全点を
一度だけ変換する。line_heightはセルサイズや中央配置に影響しない。文字サイズと
stretchは横組み時に反映済みで、kerningは圧縮幅に反映する。内部のy送りは元の
glyph位置に反映するが、セル全体の送りは変えない。

幅を合わせる対象はadvanceであり、はみ出すinkやmarkは切り取らない。前後の空白も
幅と配置に含める。glyphを出力する空白や幅0のmarkは1セル、空文字や無視される
孤立selectorだけでglyphがない場合は0セル。ink boundsは保守的な曲線boundsを
変換し、論理セルと分離する。SVGは最終runのinkと通常の余白を使うため、inkがない
空白セルは、論理送りがあってもサイズ0の空SVGとなる。

対応layerはCurrentColor・clipなし・fill pathのみ。色、gradient、stroke、clip、
raster、raw SVGはUnsupported。TAB・改行control、不正option、負の横advance合計、
非有限の形状はInvalidInputで、部分出力は返さない。

[CSS Writing Modesの縦中横](https://www.w3.org/TR/css-writing-modes-4/#text-combine-upright)
は概念上の参考で、CSS準拠を表明するものではない。数字の自動選択、hwid/twid/qwid、
全角正規化、BiDi、fallback、BASE、複数行、禁則・折返しは対象外。
MIT合成fixture・再生成方法・SHA-256は `tests/fixtures/manifest.toml` に記載。
公開APIの回帰テストは `tests/tate_chu_yoko.rs` にあり、インストール済みfontは不要。

### Lookup ごとの PairPos filtering

PairPos Format 1/2 と、それらを包む Type 9 Extension は、各 Lookup 内で
隣接グリフを選択する。GDEF の IgnoreBaseGlyphs、IgnoreLigatures、IgnoreMarks、
MarkAttachmentType、MarkFilteringSet を反映し、フラグがゼロの場合はマークを
一律に飛ばさない。IgnoreMarks が mark set より優先し、mark set は attachment
class より優先する。GDEF がない場合に glyph class を推定しない。タブ・改行を
越えるペアは作らない。locale / script と kern / vkrn の選択は従来どおりであり、
完全な GPOS shaping の実装ではない。

ゼロ値や class 0 の行列要素も最初に一致したサブテーブルとして扱う。
異なる Lookup の両 ValueRecord は i32 で加算し、途中で i16 に飽和させない。
ValueFormat2 がゼロなら第 2 グリフから次のペアを検索する。第 2 レコードが
存在する場合は値が全てゼロでも第 2 グリフを消費する。SinglePos との加算と、
接続されたマークへの明示的な advance は維持する。既存の GDEF / GPOS mark
attachment 経路は変更せず、Lookup ごとの filtering も今回追加しない。
Device / VariationIndex の補正値は引き続き未評価。

再生成可能な MIT 合成フォント `pair-filtering.ttc` とバイト単体テストで、
各フラグと優先順位、GDEF なし、両形式と Extension、ゼロ値の最初の一致、
Lookup ごとに異なる隣接グリフ、広い範囲の加算、走査位置、locale、境界、
両 ValueRecord、横 LTR / RTL / 縦書き、サイズ・横倍率、計測、マーク接続を
含む正確な SVG viewBox を検証する。既存の pair-coordinate テストは内部
ヘルパー呼び出し以外変更しない。広範な実フォント / HarfBuzz 比較は未検証。

参照: [GPOS Pair Adjustment](https://learn.microsoft.com/en-us/typography/opentype/spec/gpos#lookup-type-2-pair-adjustment-positioning-subtable)、
[Lookup Flags](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#lookupFlags)。

## 明示的な合成フォント選択

開発中 API（公開済み 0.0.17 には未収録）の `CompositeFont`、`CompositeFontRule`、
`UnicodeRange` は private fields を持つ追加型です。既存 `FontRef` の variant や
`FontOptions`、descriptor、glyph の field は追加しません。

- 範囲は両端を含む Unicode scalar。逆順、空の範囲集合・フェイス列は `InvalidInput`。
- Unicode 17.0 EGC 境界と既存 parser の代表文字を使用。Prepend は代表文字のままです。
  範囲一致は代表文字だけで判定し、結合文字、Hangul/Indic、IVS、ZWJ のクラスタを分割しません。
- 最初に一致したルールが担当し、そのフェイス列でクラスタ全体の対応を順に確認します。
  全候補が不足すれば先頭フェイスの `.notdef` を残し、別ルール・既定列へは移りません。
  既存の対応判定は UVS mapping がない場合も基底グリフを許容します。
  後続の UVS/color フェイスを優先せず、完全な Indic/emoji shaping を保証しません。
- 同じ借用フェイスオブジェクトで倍率とベースライン移動量も完全に等しい隣接クラスタは、
  ルール境界を越えて一緒に shaping します。フェイスまたは調整値の切替で GSUB/GPOS 文脈を区切り、
  x/y 両 advance を次の span に引き継ぎます。
  clone したフェイスは別オブジェクトとして扱います。
- サイズは既存のフェイス別行メトリクス（ascender − descender + line gap、対応する可変値を含む）
  による正規化で、共通 UPEM への正規化ではありません。共通 stretch/locale/variant/可変軸を渡します。
  ルール別 `with_size_multiplier` は正の有限倍率（既定 `1.0`）を受け取り、
  要求した nominal size に掛けてから通常のエンジンで shaping します。
- `with_baseline_shift` はエンジン出力単位の符号付き有限移動量（既定 `0.0`）です。
  正値で y を減らして上へ移動します。サイズ倍率は掛けず、advance・後続カーソルは変えません。
  両 builder は `Result<Self, std::io::Error>` を返し、getter は `size_multiplier()` と
  `baseline_shift()` です。担当ルール内のフォールバック先にも同じ調整を適用します。
  未指定範囲は倍率 `1.0`・移動 `0.0` を維持します。
- 単一行の横書き LTR shape/measure/SVG 専用。TAB、CR、LF、VT、FF、NEL、U+2028、U+2029 は
  `InvalidInput`。縦書き、RTL/BiDi、折返し、混在方向、縦中横との合成は提供しません。
- 計測値は横 advance の合計で ink 幅ではありません。SVG は同じ合成 run を既存 exporter に渡します。
  feature なしでも利用でき、追加 shaping/color 対応は既存 feature に従います。

既存の MIT 合成 EGC・PairPos・ligature fixture で検証し、既存 family fallback と
横書き・縦書き API の動作は変更しません。

合成フォントでは不正・overflow した実効サイズをエラーにします。既定値以外の調整を使う run は、
変換後の非有限座標・advance などもエラーにし、値の飽和処理はしません。調整がすべて既定値の run は、
極端な入力による overflow を含む既存の座標処理を維持します。
既存フェイス・family エンジンの数値処理は変更しません。
縦書き、複数行、RTL の合成は今後の課題です。

実フォントのスポット確認は、全般的なシェーピング保証ではありません。
統合された文脈依存の列整形は、以下の範囲で LTR の `rclt` と方向別の文脈 lookup を
適用し、フォント作者の cleanup 順序を保持します。合成フォントのサイズ・基線調整は
その整形範囲を拡張せず、[Furifont](https://furifont.antfu.me/) の完全対応を意味しません。

### 文脈依存の列整形（未リリース）

LTR は ccmp・選択 variant・rlig/rclt/calt/clig/liga と方向別の単一 feature を、
重複を除いた LookupList 順で適用する。方向 feature は従来どおり全 Script/LangSys、
通常 feature は locale の選択範囲を使う。RTL の合成・variant → joining →
文脈・合字・方向の境界と、既存 locale 前処理は維持する。完全な script itemization
や HarfBuzz 互換を実装したという意味ではない。

Context/Chaining の format 1/2/3 と、その中の Single/Multiple/Ligature 置換は、
元グリフへの対応と合字の構成数を保持して GPOS へ渡す。format 1 の
SequenceLookupRecord は sequenceIndex と lookupListIndex の両方を読む。
一致した無操作ルールは後続ルールを抑止し、Extension 内の逆方向文脈置換も処理する。
改行・タブ・混在方向グループの境界は越えない。Alternate の選択や一般的な
グリフ並べ替えは追加しない。

Furifont v0.7.0 の IVS placeholder のように、文脈置換の後に ccmp cleanup を置く
フォントでは、その作者の LookupList 順に従う。vrt2 の「フォント内の最後の置換」
という推奨を理由に、実装側で cleanup の後へ移動しない。vrt2 があればそれだけ、
なければ vert、混在正立は vert のみ、横倒しは vrtr のみという選択は維持する。
根拠は [LookupList 処理](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2#lookuplist-table)
と [vrt2](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrt2)。

GDEF による除外グリフを飛び越すマッチングは未実装。宣言された GDEF class から
現在の run に除外対象がないと確認できる場合だけ、flag 付き lookup を許可する。
mark attachment/filter-set flag は mark がない場合だけ許可し、それ以外は入れ子も
含めて安全にスキップする。これにより mark のない通常の合字を維持しつつ flag を
無視しない。raw 単一グリフの縦置換は従来どおり該当 flag を全てスキップする。
再帰は32段、各ルート位置で4096回の入れ子処理、出力は100万グリフに制限する。
上限到達時は公開エラーを返さず、その置換を停止する。

## Unicode に基づく Arabic-style joining

明示的な RTL text shaping では、ccmp の前に元の論理順の文字列を固定した
Unicode 17.0.0 Joining_Type で解析する。両側・右・左の結合、透明な結合記号、
tatweel、ZWJ、ZWNJ から字形を選び、フォント内の字形 feature の有無では
接続可否を推定しない。ccmp による展開・合成後は source_index から選択を
引き継ぎ、指定された字形がなければ別の接続字形に切り替えず元の glyph を保つ。
透明文字と join-causing 文字自身には接続字形を適用しない。

これは基本的な OpenType 経路の修正であり、完全なアラビア語対応ではない。
接続字形の適用は従来の単一結果 helper のままで、一般的な文脈・複数 lookup
処理にはしていない。GDEF を用いた GSUB の記号飛ばし、記号を挟む必須合字、
筆記体接続、全ての記号配置、正規化、default-ignorable glyph の除去、
Syriac 固有の joining-group 規則は対象外。source_index の継承は一般的な
cluster 再構成ではない。ZWJ/ZWNJ は接続の意味だけを修正し、一部フォントでは
送り量ゼロでも制御文字の glyph が描画される場合がある。SFArabic の記号配置は
HarfBuzz との差が残るため、送り量の一致だけで描画位置の一致とはしない。
RTL は呼び出し側が指定した run の方向であり、
Latin・数字混在の Unicode bidi や script 分割は実装していない。
GSUB を持たない AAT 専用フォント（macOS Geeza Pro など）は改善対象外。
LTR と layout 無効時の経路は変更しない。

全スカラー値の property、字形 feature の欠落、記号・制御文字の境界、
実際の ccmp 展開・合成を通す合成フォントの公開 API を回帰検証する。
ローカルの Arial・SFArabic と HarfBuzz 15 で通常の単語・記号・tatweel を
限定比較した。全フォント・全位置の一致を主張せず、実フォントは配布しない。
