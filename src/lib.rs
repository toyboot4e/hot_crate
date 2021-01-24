/*!

Reload `dylib` crates at runtime

[`libloading`] has some [issue] for reloading dynamic libraries on macOS. [`HotLibrary`]
automatically handles it under the hood.

[issue]: https://github.com/nagisa/rust_libloading/issues/59

Credit: `hot_crate` is basically a fork of [`hotlib`].

[`hotlib`]: https://github.com/mitchmindtree/hotlib

*/

pub extern crate cargo_metadata;
pub extern crate libloading;

use cargo_metadata::{Metadata, MetadataCommand, Package, Target};

use libloading::Library;

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// TODO: create error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type Error = Box<dyn std::error::Error>;

/// macOS: `dylib`, Linux: `so`, Windows: `dll`
#[cfg(target_os = "macos")]
pub const DYLIB_EXTENSION: &'static str = "dylib";

/// macOS: `dylib`, Linux: `so`, Windows: `dll`
#[cfg(target_os = "linux")]
pub const DYLIB_EXTENSION: &'static str = "so";

/// macOS: `dylib`, Linux: `so`, Windows: `dll`
#[cfg(target_os = "window")]
pub const DYLIB_EXTENSION: &'static str = "dll";

/// A reloadable dynamic [`Library`]
#[derive(Debug)]
pub struct HotLibrary {
    main_metadata: Metadata,
    dylib_toml: PathBuf,
    /// API to load symbols from the target `dylib` crate
    lib: Library,
    lib_path: PathBuf,
    /// See [`fs::Metadata::modified`][f]
    ///
    /// [f]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.modified
    lib_timestamp: Option<SystemTime>,
    /// TODO: remove counter and use something like uuid?
    reload_counter: usize,
}

impl HotLibrary {
    /// Loads a crate. NOTE: It loads **outdated version** of [`Library`] if you re-compiled your
    /// library.
    pub fn load(main_toml: impl AsRef<Path>, dylib_toml: impl AsRef<Path>) -> Result<Self> {
        let main_toml = main_toml.as_ref();
        let dylib_toml = dylib_toml.as_ref();

        let main_metadata = MetadataCommand::new().manifest_path(main_toml).exec()?;
        let lib_path = self::find_dylib_path(&main_metadata, dylib_toml)?;
        let lib = Library::new(&lib_path)?;
        let lib_timestamp = fs::metadata(&lib_path)?.modified().ok();

        Ok(Self {
            main_metadata,
            dylib_toml: dylib_toml.to_path_buf(),
            lib,
            lib_path,
            lib_timestamp,
            reload_counter: 0,
        })
    }

    pub unsafe fn get<'lib, T>(
        &'lib self,
        symbol: &[u8],
    ) -> std::result::Result<libloading::Symbol<'lib, T>, libloading::Error> {
        self.lib.get(symbol)
    }

    fn tmp_dylib_path(&mut self) -> Result<PathBuf> {
        let pkg = self::find_dylib_pkg(&self.main_metadata, &self.dylib_toml)?;
        let target = self::find_dylib_target(&self.main_metadata, &self.dylib_toml)?;

        // ${TMP_DIR}/hot_crate/${plugin}/lib${plugin}-${counter}.${ext}
        let tmp = std::env::temp_dir();
        let tmp = tmp.join("hot_crate").join(format!("{}", pkg.name));
        let tmp = tmp.join(format!(
            "lib{}-{}.{}",
            target.name, self.reload_counter, DYLIB_EXTENSION,
        ));

        self.reload_counter += 1;

        Ok(tmp)
    }

    /// Reloads dylib if it's outdated. Returns true if succeed in reloading.
    pub fn reload(&mut self) -> Result<bool> {
        let timestamp = fs::metadata(&self.lib_path)?.modified().ok();
        if timestamp == self.lib_timestamp {
            Ok(false)
        } else {
            self.force_reload()?;
            Ok(true)
        }
    }

    pub fn force_reload(&mut self) -> Result<()> {
        {
            let dylib_pkg = self::find_dylib_pkg(&self.main_metadata, &self.dylib_toml)?;
            log::info!("reloading library `{}`..", dylib_pkg.name);
        }

        let dylib_path = self::find_dylib_path(&self.main_metadata, &self.dylib_toml)?;
        let tmp_dylib_path = self.tmp_dylib_path()?;
        let tmp_dir = tmp_dylib_path.parent().unwrap();

        // Copy the dylib to the tmp location.
        fs::create_dir_all(&tmp_dir)?;
        fs::copy(&dylib_path, &tmp_dylib_path)?;

        if cfg!(target_os = "macos") {
            std::process::Command::new("install_name_tool")
                .current_dir(&tmp_dir)
                .arg("-id")
                .arg("''")
                .arg(
                    tmp_dylib_path
                        .file_name()
                        .expect("temp dylib path has no file name"),
                )
                .output()
                .expect("ls command failed to start");
        }

        self.lib = Library::new(&tmp_dylib_path)?;
        self.lib_path = dylib_path;
        self.lib_timestamp = fs::metadata(&self.lib_path)?.modified().ok();

        Ok(())
    }
}

fn find_dylib_pkg<'a>(main_metadata: &'a Metadata, dylib_toml: &Path) -> Result<&'a Package> {
    let dylib_toml = dylib_toml.canonicalize()?;

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
        .ok_or_else(|| format!("Unable to find dylib package"))?;

    Ok(dylib_pkg)
}

fn find_dylib_target<'a>(main_metadata: &'a Metadata, dylib_toml: &Path) -> Result<&'a Target> {
    let dylib_pkg = self::find_dylib_pkg(main_metadata, dylib_toml)?;

    let target = dylib_pkg
        .targets
        .iter()
        .find(|target| target.crate_types.iter().any(|t| t == "dylib"))
        .ok_or_else(|| {
            format!(
                "Unable to find `dylib` target from {}",
                dylib_toml.display()
            )
        })?;

    Ok(target)
}

fn find_dylib_path(main_metadata: &Metadata, dylib_toml: &Path) -> Result<PathBuf> {
    let target = self::find_dylib_target(main_metadata, dylib_toml)?;

    let debug_or_release = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    Ok(main_metadata.target_directory.join(format!(
        "{}/lib{}.{}",
        debug_or_release, target.name, DYLIB_EXTENSION
    )))
}
