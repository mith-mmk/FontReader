# `svg-fonts` 仕様

この文書は `features = ["svg-fonts"]` 有効時の、OpenType `SVG ` テーブル対応の現状仕様を固定するためのものです。実装コードは主に [../src/svgparse.rs](../src/svgparse.rs), [../src/commands.rs](../src/commands.rs), [../src/fontengine.rs](../src/fontengine.rs) にあります。

## 目的

- OpenType `SVG ` テーブルから glyph 単位の SVG payload を取り出す
- glyph run 上で `GlyphLayer::Svg` を保持する
- 単純な SVG 要素は `GlyphLayer::Path` にも変換し、fallback 境界や簡易描画で使えるようにする
- 複雑な SVG 機能は未対応として明示し、将来の拡張点を固定する

## 有効化条件

- Cargo feature: `svg-fonts`
- `svg-fonts` 無効時は OpenType `SVG ` テーブル由来 glyph は従来どおり未対応扱い

## レイヤモデル

`svg-fonts` 有効時、SVG glyph は次のいずれか、または両方を持てます。

- `GlyphLayer::Svg`
  - glyph から切り出した元の SVG document 断片
  - `FontEngine::render_svg()` / `FontFamily::text2svg()` では nested SVG として出力する
- `GlyphLayer::Path`
  - parser が解釈できた単純 shape/path を path command に落としたもの
  - fill と stroke は別 layer として保持する

`GlyphLayer::Svg` は常に「元 payload の保持」が目的で、`GlyphLayer::Path` は「一部 SVG の簡易変換」です。完全互換レンダラではありません。

## ノードモデル

SVG parser は文字列置換ではなく簡易ノード木を構築してから flatten します。

- `SvgNode`
  - `Element`
  - `Text`
- `SvgElement`
  - `name`
  - `attrs`
  - `children`

処理段階は次の 3 段です。

1. `parse_svg_document()`
   - XML 断片を簡易ノード木へ変換
2. `collect_definitions()`
   - `<defs>` 配下の `id` 付き要素を収集
3. `flatten_node()`
   - 継承属性、`use`、transform を解決しながら `PathGlyphLayer` 群へ落とす

## 対応要素

現状で `PathGlyphLayer` に変換する対象:

- `path`
- `rect`
- `circle`
- `ellipse`
- `line`
- `polyline`
- `polygon`

コンテナとして処理する対象:

- `svg`
- `g`
- `symbol`
- `defs`
- `use`

## 対応属性

現状で解釈する属性:

- `fill`
- `fill-rule`
- `stroke`
- `stroke-width`
- `transform`
  - `translate(...)`
  - `scale(...)`
  - `matrix(a b c d e f)`
- `x`
- `y`
- `href`
- `xlink:href`
- `style`
  - 上記属性を style 文字列からも読む

### 継承ルール

- `fill` は親から継承する
- 初期値は `currentColor`
- `fill="none"` は fill layer を生成しない
- `stroke` は親から継承する
- 初期値は `none`
- `stroke-width` は親から継承する
- 初期値は `1.0`
- `fill-rule` は親から継承する
- 初期値は `nonzero`

### `use` の解決

- `href="#id"` または `xlink:href="#id"` を解決する
- `x` / `y` は呼び出し側 transform として加算する
- `use` 側の `fill` / `fill-rule` / `stroke` / `stroke-width` は参照先要素に上書き適用する

## Path 変換ルール

### `PathGlyphLayer`

`PathGlyphLayer` は次の paint mode を持ちます。

- `PathPaintMode::Fill`
- `PathPaintMode::Stroke`

追加フィールド:

- `paint_mode`
- `stroke_width`

### fill / stroke の扱い

- 閉じた shape は fill layer と stroke layer を別々に出せる
- `line` は fill を持たず、stroke がある場合のみ stroke layer を生成する
- `polyline` は現状 fill layer 生成対象に含める
  - ただし SVG 仕様との厳密一致よりも、現在の command 化を優先している
  - 将来見直す可能性がある

### `path d` の対応コマンド

現状対応する path command:

- `M` / `m`
- `L` / `l`
- `H` / `h`
- `V` / `v`
- `Q` / `q`
- `C` / `c`
- `Z` / `z`

未対応:

- `S` / `s`
- `T` / `t`
- `A` / `a`

## SVG 出力

`FontEngine::render_svg()` / `glyph_run_to_svg()` の振る舞い:

- `GlyphLayer::Svg`
  - nested `<svg>` として元 payload を埋め込む
- `GlyphLayer::Path` + `PathPaintMode::Fill`
  - `<path fill="...">`
- `GlyphLayer::Path` + `PathPaintMode::Stroke`
  - `<path fill="none" stroke="..." stroke-width="...">`

stroke layer の bounds は `stroke_width / 2` 分だけ拡張して viewBox 計算に反映する。

## 未対応

現状、次は未対応または部分対応です。

- gradient
- pattern
- clipPath
- mask
- filter
- opacity の完全対応
- stroke-linecap
- stroke-linejoin
- stroke-dasharray
- stroke-dashoffset
- rotate / skewX / skewY を含む transform
- SVG path の arc (`A` / `a`)
- CSS class / selector ベースの style 解決
- 外部参照
- 厳密な SVG 仕様準拠の継承と presentation attributes 全般
- paintcore 側での stroke 実描画

## 非目標

この機能の現段階の非目標:

- SVG レンダラ完全実装
- ブラウザ互換の CSS/DOM/SVG 仕様完全再現
- OpenType `SVG ` テーブルの全 payload を command 化すること

## 互換性メモ

- 既存の `GlyphLayer::Svg` は維持する
- 単純 SVG は `GlyphLayer::Path` が追加で増えることがある
- そのため `layers.len()` や layer 順序に依存する呼び出し側は `Svg` だけでなく `Path` の共存を考慮すること
