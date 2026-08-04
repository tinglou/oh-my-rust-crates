# hyperscan-src

Bundled source of [intel/hyperscan](https://github.com/intel/hyperscan) v5.4.2 (BSD-3-Clause).

## Import source

```bash
git subtree add --prefix=hypervectorscan-rs/hyperscan-src/hyperscan \
  https://github.com/intel/hyperscan.git v5.4.2 --squash
```

## Update source

```bash
git subtree pull --prefix=hypervectorscan-rs/hyperscan-src/hyperscan \
  https://github.com/intel/hyperscan.git v5.4.3 --squash
```

Update `version` in `Cargo.toml` to match the new tag.
