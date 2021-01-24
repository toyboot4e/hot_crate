/*!

Reload `dylib` crates at runtime

There ara some crates for `dylib` loading and they are based on [`libloading`], but it doesn't work
well for hot reloading Rust code on macOS out of the box, so `hot_crate` does the job.

The requirement on macOS is to change the location of the dylib every time we build new `dylib`.
[`HotCrate::reload`] does it under the hood.

`hot_crate` is basically a clone of [`hotlib`].

[`hotlib`]: https://github.com/mitchmindtree/hotlib

*/

pub extern crate libloading;

use cargo_metadata::{Metadata, MetadataCommand, Package};

use libloading::{Library, Symbol};

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

/// TODO: create error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type Error = Box<dyn std::error::Error>;

/// macOS: `dylib`, Linux: `so`, Windows: `dll`, else: `<unknown-platform>`
pub const DYLIB_EXTENSION: &'static str = if cfg!(target_os = "linux") {
    "so"
} else if cfg!(any(target_os = "macos", target_os = "ios")) {
    "dylib"
} else if cfg!(target_os = "windows") {
    "dll"
} else {
    "<unknown-platform>"
};

/// A reloadable `dylib` crate
pub struct HotCrate {
    pub main_metadata: Metadata,
    /// API to load symbols from the `dylib` crate
    pub lib: Library,
    // pub lib_timestamp: SystemTime,
}

impl HotCrate {
    pub fn load(main_toml: &Path, dylib_toml: &Path) -> Result<Self> {
        let main_metadata = MetadataCommand::new().manifest_path(main_toml).exec()?;

        let lib = {
            let dylib_path = Self::find_dylib_path(&main_metadata, dylib_toml)?;
            Library::new(&dylib_path)?
        };

        Ok(Self { main_metadata, lib })
    }

    fn find_dylib_path(main_metadata: &Metadata, dylib_toml: &Path) -> Result<PathBuf> {
        let dylib_pkg = main_metadata
            .packages
            .iter()
            .find_map(|pkg| {
                if pkg.manifest_path == dylib_toml {
                    Some(pkg)
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("Unable to find package"))?;

        let dylib_target = dylib_pkg
            .targets
            .iter()
            .find(|target| target.crate_types.iter().any(|t| t == "dylib"))
            .ok_or_else(|| {
                format!(
                    "Unable to find `dylib` target from {}",
                    dylib_toml.display()
                )
            })?;

        Ok(main_metadata.target_directory.join(format!(
            "debug/lib{}.{}",
            dylib_target.name, DYLIB_EXTENSION
        )))
    }

    pub fn reload(&mut self) -> Result<()> {
        Ok(())
    }
}
