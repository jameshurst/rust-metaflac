#rust-metaflac [![Build Status](https://travis-ci.org/jamesrhurst/rust-metaflac.svg)](https://travis-ci.org/jamesrhurst/rust-metaflac)

A FLAC metadata reader/writer. The `FlacTag` struct implements the [AudioTag](https://github.com/jamesrhurst/rust-audiotag) trait for reading, writing, and modification of common metadata elements.

Documentation is available at [https://jamesrhurst.github.io/doc/metaflac](https://jamesrhurst.github.io/doc/metaflac).

##Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies.metaflac]
git = "https://github.com/jamesrhurst/rust-metaflac"
```

```rust
extern crate metaflac;

use metaflac::{AudioTag, FlacTag};

fn main() {
	let tag = AudioTag::read_from_path(&Path::new("music.flac")).unwrap();

	// Some things modifying the tag

	tag.save().unwrap();
}
```

##TODO

  * Writing to padding
  * Add tests
