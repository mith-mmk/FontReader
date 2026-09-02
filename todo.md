# FontReader / fontcore 0.0.12 実装状況

更新日: 2026-09-02
対象ブランチ: `codex-fontcore-remediation`
基準コミット: `819f677a2363f9e21b71b77a1f5bcf3d2f31b600`

## 判定方法

- `[x]` 実装と合成回帰テストが完了
- `[~]` 部分実装、外部コーパス依存、または既知の制限あり
- `[ ]` 未実装または延期
- OpenType 1.9.1、WOFF File Format 1.0、UAX #9 / #29を基準にする
- `todo.md`のチェック欄だけを実装状況の正本にしない。テスト名、対応規格、`doc/implementation-status.md`を根拠とする

## 完了

- [x] `FontFile` / `FontFace`の公開読み込み入口と`*_with_limits`、`DecodeLimits`
- [x] sfnt / TTCの表境界検証、空TTC拒否、TTC face選択時の安全なエラー処理
- [x] WOFFの宣言長、表範囲・重複、圧縮後長、チェックサム、metadata/private data、展開量の検証
- [x] WOFF2の入力長・復号後サイズに対する上限検証
- [x] `cmap` Format 4 / 12 / 13 / 14のidDelta、補助平面、既定UVS、非既定UVS、未対応形式の回帰
- [x] CPAL v0のパレット開始位置・範囲検証と`0xFFFF`のCurrentColor、COLR v0回帰
- [x] GDEFの相対基点・Offset32読解修正と合成パーサーテスト
- [x] `fvar`のhidden flag、軸・instance配列境界、`avar`の順序・必須点・軸数検証
- [x] GSUB / GPOSのfeature・lookup indexの安全なスキップ
- [x] GSUB Context / ChainingのSequenceLookupRecord保持、Multiple置換の上限、Reverse Chainingの右から左の適用
- [x] 同一GPOS lookup内の重複サブテーブル適用を防ぐ制御
- [x] SVG gzipのISIZE / CRC、呼び出し側の`max_svg_bytes`、構文ベースの保守的allowlist
- [x] script、イベント属性、外部URL、危険な要素を含むSVG payloadのfail-closed回帰
- [x] `shape()`由来の測定経路と既存のサイズ・stretch境界回帰
- [x] `FONTCORE_TEST_FONTS`指定時だけ外部フォントコーパスをコンパイルするテスト分離
- [x] push / pull request向けCIにtest、all-features、examples、doc、WASM checkを追加

## 部分実装・既知の制限

- [~] CIのstrict fmt / Clippyは既存のリポジトリ全体のformat・lint負債により未緑化。CIジョブは追加済みだが完了扱いにしない
- [~] 表単位の境界検証は導入済みだが、全表を統一`TableProvider` / readerへ移行する監査は未完了
- [~] GSUB / GPOSのlookup flag、MarkFilteringSet、FeatureVariationsの実行時置換、全Type 1–8 / 1–9共通実行器
- [~] GDEFのクラス・mark filteringの実行時結線、mark-to-ligatureのcomponent選択
- [~] Reverse Chainingは方向制御を修正済みだが、複数の実フォント差分回帰は未完了
- [~] `TextRun` / `GlyphBuffer`による完全なUAX #9 / #29 bidi・script itemizationは未導入
- [~] `FontFamily`は距離ベース選択のままで、家族列・Last Resort・クラスタ単位のCSS相当fallbackは未完了
- [~] SVGは安全な構文allowlistで拒否できる範囲を確保した段階。完全なscene化、ID・transform・clip・gradient座標の統一は未完了
- [~] COLR v0 / CPAL v0は対象。COLR v1 / CPAL v1は未実装
- [~] CFF2実装は存在するが、再配布可能な実フォントfixture、軸別outline署名、malformed corpus回帰が未完了
- [~] HarfBuzz差分比較とファジングは開発用手順であり、CIの定期検査には未統合

## Solレビューで確認した残件

- [ ] GSUB Multiple置換で同一lookupを自己参照する入力を含む、固定点反復・入力cursor・出力上限の包括的検証
- [ ] TTC / GSUB / GPOSの全公開経路について、任意のcount・index・offsetのパニック監査を完了
- [ ] WOFF表数上限とソート済み範囲検査を、コンテナdecoder共通層へ統合
- [ ] `DecodeLimits`をWOFF / SVG以外のすべての展開・確保経路へ接続
- [ ] OpenType SVGをXML allowlistまたはsceneへ変換し、文字列ブラックリスト依存を完全に排除
- [ ] fvar / avarのinstance範囲・map仕様を実フォントで追加検証

## 次の実装順

1. 境界reader / `DecodeLimits`の全表共通化と残存panic除去
2. GSUB / GPOS共通lookup実行器とGDEF検索フラグの統合
3. `TextRun` / `GlyphBuffer`、UAX #29 cluster、UAX #9 bidi、script / language itemization
4. `shape()`・`measure()`・`render_svg()`の詳細結果共有とクラスタ単位fallback
5. 安全なSVG scene、出力座標・bounds統一、COLR v1 / CPAL v1
6. CFF2実フォント回帰、loader / feature / 文書の整理

## テスト実行

外部フォントなしの必須テスト:

```text
cargo test --all-features --lib
```

外部コーパスを明示的に使う場合:

```text
FONTCORE_TEST_FONTS=<path> cargo test --all-features --lib
```

外部コーパスはライセンス、出典、SHA-256、対象featureをfixture manifestで管理し、通常のclean cloneの必須依存にはしない。

## 2026-09-02 検証結果

- clean相当の`cargo test --all-features --lib`: 108 passed
- `FONTCORE_TEST_FONTS`指定時の外部コーパス: 240 passed、3 ignored
- `cargo test --no-default-features`: 32 passed
- `cargo test --no-default-features --features "layout,cff"`: 39 passed
- `cargo check --examples --all-features`、WASM check、all-feature doc生成: 成功
- `cargo fmt --all -- --check`: 既存を含むformat差分により未成功
- `cargo clippy --all-targets --all-features -- -D warnings`: 既存を含むlint負債により未成功
