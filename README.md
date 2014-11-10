#rust-metaflac

A FLAC metadata reader/writer. The `FlacTag` struct implements the [AudioTag](https://github.com/jamesrhurst/rust-audiotag) trait for reading, writing, and modification of common metadata elements.

Documentation is available at [https://jamesrhurst.github.io/docs/metaflac](https://jamesrhurst.github.io/docs/metaflac).

##Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies.metaflac]
git = "https://github.com/jamesrhurst/rust-metaflac"
```

```rust
extern crate metaflac;
use flac::{AudioTag, FlacTag};

fn main() {
	let tag = AudioTag::load("music.flac");

	// Some things modifying the tag

	tag.save();
}
```
