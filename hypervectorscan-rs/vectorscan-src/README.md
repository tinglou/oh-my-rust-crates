# vectorscan-src

Bundled source of [VectorCamp/vectorscan](https://github.com/VectorCamp/vectorscan) v5.4.12 (BSD-3-Clause).

## Import source

```bash
git subtree add --prefix=hypervectorscan-rs/vectorscan-src/vectorscan \
  https://github.com/VectorCamp/vectorscan.git vectorscan/5.4.12 --squash
```

## Update source

```bash
git subtree pull --prefix=hypervectorscan-rs/vectorscan-src/vectorscan \
  https://github.com/VectorCamp/vectorscan.git vectorscan/5.4.13 --squash
```

Update `version` in `Cargo.toml` to match the new tag.
