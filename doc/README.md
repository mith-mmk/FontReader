# Documentation Map

This directory keeps the longer documents that no longer fit well in the main README.

## Start Here

- If you want to use the crate right away, begin with [../README.md](../README.md)
- If you want API examples by task, read [api-recipes.md](api-recipes.md)
- If you want current implementation notes and limitations, read [feature-status.md](feature-status.md)
- If you want the current `svg-fonts` behavior and the `paintcore` handoff contract, read [SVFONTSPEC.md](SVFONTSPEC.md)
- If you want CFF2-specific investigation notes, read [cff2-investigation.md](cff2-investigation.md)

## Recommended Reading Order

1. [../README.md](../README.md)
2. [api-recipes.md](api-recipes.md)
3. [feature-status.md](feature-status.md)
4. [SVFONTSPEC.md](SVFONTSPEC.md)

## About `cargo doc`

Public rustdoc is available from the crate root and the main public types.

```bash
cargo doc --no-deps
```

Good entry points inside rustdoc:

- `fontcore::FontFile`
- `fontcore::FontFace`
- `fontcore::FontEngine`
- `fontcore::FontFamily`
- `fontcore::FontVariant`
- `fontcore::ShapingPolicy`
