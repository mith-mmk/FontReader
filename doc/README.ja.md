# ドキュメント一覧

このディレクトリには、README から分離した少し長めの資料を置いています。

## 最初に見るもの

- まず使い始めたい場合は [../README.ja.md](../README.ja.md)
- 用途別の公開API例を見たい場合は [api-recipes.ja.md](api-recipes.ja.md)
- 実装状況や制限事項を見たい場合は [feature-status.ja.md](feature-status.ja.md)
- `svg-fonts` の現在仕様を見たい場合は [SVFONTSPEC.md](SVFONTSPEC.md)
- CFF2 まわりの調査メモは [cff2-investigation.ja.md](cff2-investigation.ja.md)

## おすすめの読む順

1. [../README.ja.md](../README.ja.md)
2. [api-recipes.ja.md](api-recipes.ja.md)
3. [feature-status.ja.md](feature-status.ja.md)
4. [SVFONTSPEC.md](SVFONTSPEC.md)

## `cargo doc` について

公開API には rustdoc を付けています。

```bash
cargo doc --no-deps
```

rustdoc で最初に見ると分かりやすい型:

- `fontloader::FontFile`
- `fontloader::FontFace`
- `fontloader::FontEngine`
- `fontloader::FontFamily`
- `fontloader::FontVariant`
- `fontloader::ShapingPolicy`
