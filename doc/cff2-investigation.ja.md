# CFF2 調査メモ

## 目的

`src/opentype/outline/cff.rs` をそのまま拡張する前に、CFF2 実装でどこまで既存 CFF 実装を共有できるかを整理する。

## 共有しやすい層

CFF / CFF2 で比較的そのまま共有しやすいのは次の部分です。

- INDEX 解析
- DICT 解析
- operand 解析
- charstring token 解析
- subr bias 計算と subroutine dispatch
- path command の生成

つまり共有しやすいのは「compact font bytecode の解析実行層」であり、現在の `CFF::new()` 全体ではありません。

## CFF1 固有の層

以下は CFF1 側に閉じ込めた方が安全です。

- Name INDEX / String INDEX 前提
- CFF1 固有の Top DICT operator 解釈
- Encodings / charsets / FDSelect / FDArray の現在の結線
- Private DICT 由来の width / default width 処理
- CFF1 前提のトップレベル table 構造

## CFF2 固有の層

CFF2 では次の処理が追加で必要です。

- CFF2 Top DICT 差分
- variation store
- `vsindex`
- `blend`
- 正規化座標に基づく variation 評価
- CFF2 charstring 実行ルール

## 推奨分割

分け方としては次の 3 層が自然です。

1. `cff_shared`
   - INDEX / DICT / operand / charstring 解析
   - subroutine 実行 helper
   - path 構築
2. `cff1`
   - 現在の CFF 読み込み
   - Private DICT の width 処理
3. `cff2`
   - variation store
   - `vsindex` / `blend`
   - 正規化座標の適用

## 現時点の結論

CFF と CFF2 で内部ライブラリ共有は可能です。

ただし、現在の単一ファイルの `CFF::new()` をそのまま共有化の中心にするのは不向きです。先に bytecode / DICT / subroutine の共有層を切り出してから、その上に CFF2 固有の variation 経路を載せるのが安全です。
