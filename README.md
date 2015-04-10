#rust-metaflac 

[![Build Status](https://travis-ci.org/jamesrhurst/rust-metaflac.svg)](https://travis-ci.org/jamesrhurst/rust-metaflac)

A library for reading and writing FLAC metadata.

[Documentation](http://jamesrhurst.github.io/rust-metaflac/)

##Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies.metaflac]
git = "https://github.com/jamesrhurst/rust-metaflac"
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
