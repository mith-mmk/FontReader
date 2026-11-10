# APIレシピ

公開APIの実行例を、用途ごとにまとめたドキュメントです。

## Metadata を読む

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
println!("{}", face.family());
println!("{}", face.full_name());
println!("{}", face.weight().0);
println!("{}", face.is_italic());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 基本の shaping

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face.engine().with_font_size(32.0).shape("Hello")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 文字幅を測る

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let width = face.engine().with_font_size(32.0).measure("Hello")?;
assert!(width > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 縦書き SVG 出力

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let svg = face
    .engine()
    .with_font_size(32.0)
    .with_vertical_flow()
    .render_svg_vertical("縦書き")?;
assert!(svg.contains("<svg"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Unicode の縦書きプロパティを調べる

フォントや機能フラグを必要とせず、Unicode スカラー値のプロパティを取得できます。
向きと縦書き用字形の違いを保持するため、戻り値は次の 4 値です。

```rust
use fontcore::{
    vertical_orientation, VerticalOrientation as Vo, VERTICAL_ORIENTATION_UNICODE_VERSION,
};

assert_eq!(VERTICAL_ORIENTATION_UNICODE_VERSION, "18.0.0");
assert_eq!(vertical_orientation('漢'), Vo::Upright); // U
assert_eq!(vertical_orientation('A'), Vo::Rotated); // R
assert_eq!(vertical_orientation('、'), Vo::TransformWithUprightFallback); // Tu
assert_eq!(vertical_orientation('ー'), Vo::TransformWithRotatedFallback); // Tr
```

`U` はコード表と同じ正立、`R` は時計回りに 90 度回転する値です。`Tu` と `Tr` は
縦書き用の字形を優先し、その字形がないときに、それぞれ正立または回転したコード表の
字形を用います。特に `Tr` は「常に回転」ではなく、正立の縦書き用字形を優先します。

Unicode 18.0.0 に固定したデータを使い、未割当文字・私用文字の範囲も保持しています。
書記素クラスタ、結合文字、異体字セレクタ、絵文字列、字形置換、混在縦書きは解決せず、
既存の shaping API の動作も変えません。文字単位のプロパティは、最終的なクラスタや
グリフの向きではありません。[データの出典と再生成手順](../data/unicode/README.md)。

## Sideways LTR の整形・計測・SVG 出力

左から右へ横書きした一行全体を時計回りに回転する場合は、専用の sideways
メソッドを使います。正立した文字を上から下へ配置する縦書きとは異なります。

```rust
use fontcore::{FontFile, ShapingPolicy};

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let engine = face
    .engine()
    .with_font_size(32.0)
    .with_vertical_flow()
    .with_svg_unit("px");

let run = engine.shape_sideways_ltr("Hello")?;
let metrics = engine.measure_layout_sideways_ltr("Hello")?;
let advance = engine.measure_sideways_ltr("Hello")?;
let svg = engine.render_svg_sideways_ltr("Hello")?;

assert!(!run.glyphs.is_empty());
assert_eq!(advance, metrics.inline_advance);
assert!(svg.contains("<svg"));
assert_eq!(engine.shaping_policy(), ShapingPolicy::TopToBottom);
# Ok::<(), Box<dyn std::error::Error>>(())
```

LTR・RTL・上から下のいずれの設定でも、各呼び出しの間だけ LTR として整形します。
engine と options は変更せず、style・locale・variant・対応済みの可変軸・
サイズ・横伸縮・SVG 単位を維持します。通常の `shape`・`measure`・
`measure_layout`・SVG メソッドの動作は変わりません。

横メトリクスで整形した run を、ベースライン原点 `(0, 0)` の周りに時計回りに
90 度回転します。em 中央への移動は行いません。計測も同じ回転後の run を使い、
`inline_advance` は y 方向送り量の合計の絶対値、`ink_bounds` は配置済みの描画範囲です。
`line_bounds` は各配置原点と、送り量ベクトル全体を足した終点を含みます。
`block_extent` は `line_bounds` の x 方向の幅であり、フォントの行高ではありません。
空文字列の bounds はなく、各 extent は 0 です。空白は横送り量を回転した縦方向の
送り量を保持します。

対応するのは、クリップなし・current-color の塗りつぶし path のみです。
単色指定やグラデーションを含むほかの paint、stroke、clip、raster、raw SVG layer は
`std::io::ErrorKind::Unsupported` となり、部分的な結果は返しません。
TAB・CR・LF・VT・FF・NEL・U+2028・U+2029 は `std::io::ErrorKind::InvalidInput`
です。複数行への分割と配置は呼び出し側で行ってください。単一 face の API であり、
family fallback・BiDi・向きが混在する文字列の自動判定・正立 CJK のベースラインとの
位置合わせは追加しません。`vrtr`、座標、整形の対応範囲は
[feature-status.ja.md](feature-status.ja.md#sideways-ltr-の-run) を参照してください。

## RTL shaping

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/YourFont.ttf")?.current_face()?;
let run = face
    .engine()
    .with_font_size(32.0)
    .with_right_to_left()
    .shape("مرحبا")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## GSUB variant 切り替え

```rust
use fontcore::{FontFile, FontVariant};

let face = FontFile::from_file("fonts/YourFont.otf")?.current_face()?;
let run = face
    .engine()
    .with_font_size(32.0)
    .with_locale("ja-JP")
    .with_font_variant(FontVariant::Jis78)
    .shape("辻")?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Variable font axis 指定

```rust
use fontcore::FontFile;

let face = FontFile::from_file("fonts/VariableFont.ttf")?.current_face()?;
for axis in face.variation_axes() {
    println!(
        "{} {}..{} (default {})",
        axis.tag, axis.min_value, axis.max_value, axis.default_value
    );
}

let width = face
    .engine()
    .with_font_size(32.0)
    .with_variation("wdth", 75.0)
    .measure("Hello")?;
assert!(width > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## TTC / collection の face 選択

```rust
use fontcore::FontFile;

let file = FontFile::from_file("fonts/YourCollection.ttc")?;
let face = file.face(1)?;
println!("{}", face.full_name());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## FontFamily fallback

```rust
use fontcore::{FontFamily, FontFile, FontWeight};

let regular = FontFile::from_file("fonts/FiraSans-Regular.ttf")?.current_face()?;
let bold = FontFile::from_file("fonts/FiraSans-Bold.ttf")?.current_face()?;

let mut family = FontFamily::new("Fira Sans");
family.add_font_face(regular);
family.add_font_face(bold);

let run = family.text2glyph_run(
    "Hello",
    family.options().with_font_weight(FontWeight::BOLD),
)?;
assert!(!run.glyphs.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 分割 WOFF2 読み込み

```rust
use fontcore::ChunkedFontBuffer;

let mut buffer = ChunkedFontBuffer::new(total_size)?;
buffer.append(0, first_chunk)?;
buffer.append(second_offset, second_chunk)?;

if buffer.is_complete() {
    let face = buffer.into_font_face()?;
    assert!(face.measure("Hello")? > 0.0);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## メモ

- shaping direction、locale、variant、variable-font axis は `FontEngine` を主な入口にしています。
- `FontOptions` は低レイヤ寄りの制御や `FontFamily` 連携用として残しています。
- 実装状況と制限事項は [feature-status.ja.md](feature-status.ja.md) にまとめています。
- CFF2 調査メモは [cff2-investigation.ja.md](cff2-investigation.ja.md) にまとめています。
- ドキュメントの入口は [README.ja.md](README.ja.md) です。
