# `hot_crate`

_Reload `dylib` crates at runtime_

[`libloading`](https://github.com/nagisa/rust_libloading) has some [issue](https://github.com/nagisa/rust_libloading/issues/59) for reloading dynamic libraries on macOS. `hot_crate::HotLibrary` automatically handles it under the hood.

Credit: `hot_crate` is basically a fork of [`hotlib`](https://github.com/mitchmindtree/hotlib).

