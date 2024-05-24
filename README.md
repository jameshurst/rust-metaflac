# rust-metaflac

[![Crates.io Version](https://img.shields.io/crates/v/metaflac)](https://crates.io/crates/metaflac)
[![docs.rs](https://img.shields.io/docsrs/metaflac)](https://docs.rs/metaflac)
[![Crates.io License](https://img.shields.io/crates/l/metaflac)](./LICENSE)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/jameshurst/rust-metaflac/ci.yaml)](https://github.com/jameshurst/rust-metaflac/actions/workflows/ci.yaml)

A library for reading and writing FLAC metadata.

## Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
metaflac = "0.2.6"
```

```rust
use metaflac::Tag;

fn main() {
  let tag = Tag::read_from_path("music.flac").unwrap();

  // Some things modifying the tag

  tag.save().unwrap();
}
```
