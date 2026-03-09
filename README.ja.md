# Rust向け Fontloader

OpenType、TrueType、TTC、WOFF、および一部の WOFF2 フォントデータを読み込むための Rust ライブラリです。

English: [README.md](README.md)

## Layout 対応状況

`layout` feature は一部のみ実装されています。

### GSUB

- パース済み: `ScriptList`, `FeatureList`, `LookupList`
- 実装済み: 単一置換ベースの縦書き置換 `lookup_vertical()`
- 部分実装: `lookup_ccmp()` は存在するが、結果展開は未実装
- 未実装: `lookup_locale()`, `lookup_liga()`, `lookup_width()`, `lookup_number()`

### Lookup パース

- Type 1 Single Substitution: パース済み、展開可能
- Type 2 Multiple Substitution: パース済み、展開可能
- Type 3 Alternate Substitution: パース済み、展開可能
- Type 4 Ligature Substitution: パース済み、展開可能
- Type 5 Context Substitution:
  Format 1 は展開可能
  Format 2 と Format 3 はパースのみで、適用は未完成
- Type 6 Chaining Context Substitution:
  Format 1 は展開可能
  Format 2 は一部のみ適用
  Format 3 はパースのみで、適用は未実装
- Type 7 Extension Substitution: パース済み、適用は未実装
- Type 8 Reverse Chaining Contextual Single Substitution: パース済み、適用は未実装

### GDEF

- パース済み: glyph class definition, attach list, ligature caret list, mark attach class definition, mark glyph sets definition
- 現状: 読み込みとデバッグ出力は可能だが、上位の shaping 処理にはまだ統合されていません

## examples の実行方法

example は `examples/` 以下にあります。

通常の実行:

```bash
cargo run --example fontloader -- path/to/font.ttf
```

layout パースが必要な example:

```bash
cargo run --features layout --example fontgsub -- path/to/font.ttf
```

CFF 対応が必要な example:

```bash
cargo run --features cff --example fontsvg -- path/to/font.otf
```

feature をまとめて有効にする場合:

```bash
cargo run --features full --example fontgsub -- path/to/font.otf
```

フォントパスを省略すると、example によっては OS の既定フォントを使います。
