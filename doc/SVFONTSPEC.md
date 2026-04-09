# SVFONTSPEC

この文書は、`FontReader` が `paintcore` に渡すべき SVG glyph 情報を固定するための受け渡し仕様です。あわせて、`fontloader` 側で `features = ["svg-fonts"]` 有効時にどこまで解決してから渡すかも定義します。

目的は 2 つです。

1. `FontReader` と `paintcore` の責務境界を明確にする
2. SVG glyph 対応を `Command` の拡張ではなく layer / paint モデルで扱う

この文書は `FontReader` 側の parser 実装仕様そのものではなく、`paintcore` が受け取るべき最終データ契約を定義します。

## 基本方針

- SVG の解釈は原則として `FontReader` 側の責務
- `paintcore` は parser ではなく renderer として振る舞う
- `paintcore` は `GlyphLayer` 群を受け取り、それぞれを描画する
- path 化できない、または別 backend に委譲したい場合だけ raw SVG payload を `GlyphLayer::Svg` として保持する
- simple SVG の path 化は `FontReader` 側で完了していること
- `linearGradient` / `radialGradient` / `stop` のような単純 gradient は `GlyphPaint` に解決してから渡すこと

## `Command` の責務

`Command` は幾何学的な path command だけを表す。

含めるもの:

- `MoveTo`
- `Line`
- `Bezier`
- `CubicBezier`
- `Close`

含めないもの:

- fill / stroke
- gradient
- style
- transform の継承状態
- `use` / `defs` の参照情報
- raw SVG payload

つまり、`Command` は「何を描くか」の形状だけを持ち、「どう塗るか」は持たない。

## `FontReader` が解決してから渡すべき情報

`FontReader` は SVG glyph を解釈した後、少なくとも次のいずれかを `paintcore` に渡すこと。

### 1. `GlyphLayer::Path`

simple SVG を path 化できた場合は `PathGlyphLayer` を渡す。

`PathGlyphLayer` に必要な情報:

- `commands`
- `paint`
- `paint_mode`
- `fill_rule`
- `stroke_width`
- `offset_x`
- `offset_y`

#### `paint_mode`

`FontReader` は path layer ごとに paint mode を確定して渡す。

- `PathPaintMode::Fill`
- `PathPaintMode::Stroke`

同じ shape が fill と stroke の両方を持つ場合は、1 layer に混在させず、別 layer に分けて渡すこと。

#### `paint`

`FontReader` は path layer ごとに paint を確定して渡す。

最低限:

- `GlyphPaint::CurrentColor`
- `GlyphPaint::Solid(u32)`
- `GlyphPaint::LinearGradient(...)`
- `GlyphPaint::RadialGradient(...)`

SVG gradient を path layer として `paintcore` に描かせる場合は、次も渡す。

- `GlyphPaint::LinearGradient(...)`
- `GlyphPaint::RadialGradient(...)`

#### `fill_rule`

`fill-rule` の継承結果を layer ごとに確定して渡す。

- `NonZero`
- `EvenOdd`

#### `stroke_width`

stroke layer の見た目に必要な線幅を layer ごとに確定して渡す。

### 2. `GlyphLayer::Raster`

SVG を `FontReader` 側で rasterize した場合は `RasterGlyphLayer` を渡す。

必要な情報:

- RGBA buffer または encoded bitmap
- width / height
- offset

### 3. `GlyphLayer::Svg`

raw SVG payload を保持したい場合は `SvgGlyphLayer` を渡す。

必要な情報:

- `document`
- `view_box_min_x`
- `view_box_min_y`
- `view_box_width`
- `view_box_height`
- `width`
- `height`
- `offset_x`
- `offset_y`

`GlyphLayer::Svg` は「simple SVG を path 化できなかった時の生データ保持」および「別 renderer への委譲」のために存在する。path 化できる glyph について raw SVG を常に重ね持ちする必要はない。

## `FontReader` が自前で解決しておくべき事項

`paintcore` に渡す前に、`FontReader` は次を解決済みであること。

- `<defs>` の収集
- `<use>` の参照解決
- `href` / `xlink:href` の適用
- `x` / `y` の合成
- presentation attributes と `style` の統合
- `fill` / `stroke` / `fill-rule` / `stroke-width` の継承
- 対応 transform の適用結果
- shape 要素の path 化

現状の `fontloader` 実装では、少なくとも次の対応範囲については `paintcore` に未解決状態で渡さないこと。

- 要素
  - `path`
  - `rect`
  - `circle`
  - `ellipse`
  - `line`
  - `polyline`
  - `polygon`
- コンテナ
  - `svg`
  - `g`
  - `symbol`
  - `defs`
  - `use`
- 属性
  - `fill`
  - `fill-rule`
  - `stroke`
  - `stroke-width`
  - `style`
  - `x`
  - `y`
  - `href`
  - `xlink:href`
- transform
  - `translate(...)`
  - `scale(...)`
  - `matrix(a b c d e f)`

## `paintcore` が担当する責務

`paintcore` は SVG parser を持たず、次だけを担当する。

- `PathGlyphLayer` の fill 描画
- `PathGlyphLayer` の stroke 描画
- gradient paint の rasterization
- `RasterGlyphLayer` の描画
- `SvgGlyphLayer` の renderer / adapter への委譲

## `paintcore` に渡してはいけない曖昧な状態

次のような中途半端な情報は渡さないこと。

- path はあるが fill/stroke の別が不明
- gradient 参照 ID だけあり、stop 情報が未解決
- `use` の参照先解決前のノード
- 親からの継承が未適用の style
- transform 適用前の座標系

`paintcore` は renderer であり、SVG DOM 解決器ではない。

## 推奨データフロー

1. `FontReader` が OpenType `SVG ` テーブルから glyph payload を抽出する
2. `FontReader` が SVG を解釈し、simple SVG を `GlyphLayer::Path` に落とす
3. path 化できない、または保持が必要な payload は `GlyphLayer::Svg` で残す
4. `paintcore` は `GlyphLayer::Path` を直接描画する
5. `paintcore` は `GlyphLayer::Svg` を必要に応じて rasterizer adapter に委譲する

## gradient の扱い

gradient は `Command` ではなく `GlyphPaint` の責務とする。

理由:

- 同じ path geometry に対して複数の paint を適用できるため
- stroke/fill と同じく「描画スタイル」であり「形状」ではないため
- raw SVG と path 化 SVG の両方で同じ paint モデルを共有しやすいため

現状の `fontloader` 実装では、少なくとも `linearGradient` / `radialGradient` / `stop` を path layer の paint として解決できる。`FontReader` が gradient を path layer として渡す場合、少なくとも次を確定済みで渡すこと。

- gradient 種別
- gradient units
- gradient transform
- endpoint / center / radius
- spread mode
- stop 一覧
- stop color
- stop offset

## stroke の扱い

stroke は `Command` ではなく `PathGlyphLayer` の責務とする。

理由:

- 同じ path に対して fill と stroke が同時に存在し得るため
- line width は geometry そのものではないため
- renderer 側の antialias / rasterization 実装と直結するため

従って `FontReader` は stroke を見つけたら:

- `paint_mode = Stroke`
- `stroke_width = resolved width`

を設定した別 layer を渡すこと。

## raw SVG renderer / adapter への期待値

`paintcore` 側で `GlyphLayer::Svg` を描く場合でも、SVG parser の責務をこちらに戻さないこと。

望ましい実装は:

- `SvgGlyphLayer` を受け取る
- 別 backend が rasterize して `RasterGlyphLayer` に変換する
- `paintcore` はその raster を描画する

つまり `paintcore` 本体に必要なのは parser ではなく adapter 境界である。

## 非推奨

次の設計は非推奨。

- SVG 対応のために `Command` を style 情報込みで肥大化させる
- `paintcore` 本体に `defs` / `use` / style 継承付きの SVG parser を持ち込む
- `FontReader` と `paintcore` の両方で同じ SVG 解釈ロジックを重複実装する

## まとめ

`FontReader` が本来 `paintcore` に渡すべき情報は、SVG の未解決ノード列ではなく、描画可能な layer 群である。

最小構成では次が必要。

- `GlyphLayer::Path`
  - `commands`
  - `paint`
  - `paint_mode`
  - `fill_rule`
  - `stroke_width`
  - `offset`
- `GlyphLayer::Raster`
  - decoded or encoded bitmap
- `GlyphLayer::Svg`
  - raw payload と viewBox 情報

この境界を守ることで、`FontReader` は parser、`paintcore` は renderer として責務を分離できる。

## 現状の未対応

`fontloader` / `paintcore` 境界の現状未対応は次です。

- gradient
- pattern
- `clipPath`
- `mask`
- filter
- gradientTransform
- gradientUnits の shape bounds まで含む完全解決
- `stroke-linecap`
- `stroke-linejoin`
- `stroke-dasharray`
- `stroke-dashoffset`
- `rotate` / `skewX` / `skewY`
- SVG path arc (`A` / `a`)
- CSS class / selector ベースの style 解決
- 外部参照
- `paintcore` 側の stroke 実描画
