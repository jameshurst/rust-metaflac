# rust-metaflac

[![Build Status](https://travis-ci.org/jameshurst/rust-metaflac.svg)](https://travis-ci.org/jameshurst/rust-metaflac)
[![](http://meritbadge.herokuapp.com/metaflac)](https://crates.io/crates/metaflac)

A library for reading and writing FLAC metadata.

[Documentation](http://jameshurst.github.io/rust-metaflac/)

## Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
metaflac = "0.2.6"
```

```rust
extern crate metaflac;

use metaflac::Tag;

fn main() {
	let tag = Tag::read_from_path("music.flac").unwrap();

	// Some things modifying the tag

	tag.save().unwrap();
}
```
