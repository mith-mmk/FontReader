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
BASE と下記の静的COLR v0以外の色layer原点は別対応です。向きは従来の原点補正で判定せず、明示混在 API が指定します。
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
選択対象に含めない。この従来の正立配置経路では描画側でグリフを回転しないため、
`vrtr` は適用しない。
[OpenType vrt2 仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/features_uz#tag-vrt2)
の回転済み字形とフィーチャーの排他性に従う。

既存の探索範囲は維持する。全 Script の既定・名前付き LangSys と必須
フィーチャーを含み、DFLT 以外だけを持つフォントも対象となる。
Script run ごとの選択や Unicode の混在方向は未対応。
高水準のテキスト整形では cmap/UVS を先に解決し、通常の GSUB 段階
（合成、locale/variant、RTL 字形、合字）の後に選択した縦書きフィーチャーを
適用する。メトリクス、輪郭の原点、GPOS は最終的なグリフ ID を使う。
raw の `get_glyph_with_uvs_axis()` はテキスト整形を
行わず、従来どおり直接縦書き字形へ置換する。ソースクラスタ表現は変更しない。
縦書きフィーチャー内の Lookup 連続適用は次節で説明する。
横書きと `layout` 無効時は変更なし。MIT の合成フォントで選択結果、
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
グリフ単位で `vert` へ戻ることもない。全 Script/LangSys の探索範囲と
縦書き以外の共通ヘルパーは変更しない。完全な文脈依存縦書き処理ではない。

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

`layout` 有効時は通常の横書き GSUB の後に `vrtr` を適用し、最終グリフ ID を
メトリクスと GPOS に使う。この順序は実装方針であり、`vrtr` を最後に適用するという
仕様上の義務ではない。最後に適用するという明示的な要件を持つのは `vrt2` である。
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

正立グループは従来の縦メトリクス・原点を使い、通常 GSUB の後に **vert のみ**を
適用します。vrt2/vrtr は選びません。Tr に vert 代替がない場合や同一字形への
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
