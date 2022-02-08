/*!
Reload `dylib` crates at runtime

[`libloading`] has some [issue] for reloading dynamic libraries on macOS. [`HotCrate`] automatically
handles it under the hood.

Credit: `hot_crate` is basically a fork of [`hotlib`].

[issue]: https://github.com/nagisa/rust_libloading/issues/59
[`hotlib`]: https://github.com/mitchmindtree/hotlib
*/

pub extern crate cargo_metadata;
pub extern crate libloading;

pub use camino::{self, Utf8Path, Utf8PathBuf};
pub use libloading::Symbol;

use cargo_metadata::{Metadata, MetadataCommand, Package, Target};
use libloading::Library;

use std::{fs, time::SystemTime};

/// TODO: create error type
pub type Error = Box<dyn std::error::Error>;

/// TODO: create error type
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// macOS: `dylib`, Linux: `so`, Windows: `dll`
#[cfg(target_os = "macos")]
const DYLIB_EXTENSION: &'static str = "dylib";

/// macOS: `dylib`, Linux: `so`, Windows: `dll`
#[cfg(target_os = "linux")]
const DYLIB_EXTENSION: &'static str = "so";

/// macOS: `dylib`, Linux: `so`, Windows: `dll`
#[cfg(target_os = "window")]
const DYLIB_EXTENSION: &'static str = "dll";

/// A reloadable dynamic [`Library`]
#[derive(Debug)]
pub struct HotCrate {
    main_metadata: Metadata,
    dylib_toml: Utf8PathBuf,
    /// API to load symbols from the target `dylib` crate
    lib: Library,
    lib_path: Utf8PathBuf,
    /// See [`fs::Metadata::modified`][f]
    ///
    /// [f]: https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.modified
    lib_timestamp: Option<SystemTime>,
    /// TODO: remove counter and use something like uuid?
    reload_counter: usize,
}

unsafe impl Send for HotCrate {}
unsafe impl Sync for HotCrate {}

impl HotCrate {
    /// Loads a `dylib` crate
    ///
    /// See [`Library::new`] for thread safety. Arguments are in absolute paths.
    pub fn load(main_toml: impl AsRef<Utf8Path>, dylib_toml: impl AsRef<Utf8Path>) -> Result<Self> {
        let main_toml = main_toml.as_ref();
        let dylib_toml = dylib_toml.as_ref();

        let main_metadata = MetadataCommand::new().manifest_path(main_toml).exec()?;
        let lib_path = self::find_dylib_path(&main_metadata, dylib_toml)?;
        let lib = unsafe { Library::new(&lib_path)? };
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

    /// See [`libloading::Library::close`]
    pub fn unload(self) -> std::result::Result<(), libloading::Error> {
        self.lib.close()
    }

    pub unsafe fn get<'lib, T>(
        &'lib self,
        symbol: &[u8],
    ) -> std::result::Result<libloading::Symbol<'lib, T>, libloading::Error> {
        self.lib.get(symbol)
    }

    fn tmp_dylib_path(&mut self) -> Result<Utf8PathBuf> {
        let pkg = self::find_dylib_pkg(&self.main_metadata, &self.dylib_toml)?;
        let target = self::find_dylib_target(&self.main_metadata, &self.dylib_toml)?;

        // ${TMP_DIR}/hot_crate/lib${plugin}-${counter}.${ext}
        let tmp = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|p| format!("unable to create UTF8 path from {}", p.display()))?;
        let tmp = tmp.join("hot_crate").join(format!("{}", pkg.name));
        let tmp = tmp.join(format!(
            "lib{}-{}.{}",
            target.name, self.reload_counter, DYLIB_EXTENSION,
        ));

        self.reload_counter += 1;

        Ok(tmp)
    }

    pub fn lib(&self) -> &Library {
        &self.lib
    }

    /// Reloads the dylib if it's outdated. Returns true if succeed in reloading.
    pub fn try_reload(&mut self) -> Result<bool> {
        let timestamp = fs::metadata(&self.lib_path)?.modified().ok();

        if timestamp == self.lib_timestamp {
            Ok(false)
        } else {
            self.force_reload()?;
            Ok(true)
        }
    }

    /// Reloads the dylib anyways
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
                .arg(tmp_dylib_path.file_name().unwrap())
                .output()
                .expect("`install_name_tool` failed to start");
        }

        self.lib = unsafe { Library::new(&tmp_dylib_path)? };
        self.lib_path = dylib_path;
        self.lib_timestamp = fs::metadata(&self.lib_path)?.modified().ok();

        Ok(())
    }
}

fn find_dylib_pkg<'a>(main_metadata: &'a Metadata, dylib_toml: &Utf8Path) -> Result<&'a Package> {
    let dylib_toml = dylib_toml.canonicalize()?;

    let dylib_pkg = main_metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path == dylib_toml)
        .ok_or_else(|| format!("Unable to find dylib package"))?;

    Ok(dylib_pkg)
}

fn find_dylib_target<'a>(main_metadata: &'a Metadata, dylib_toml: &Utf8Path) -> Result<&'a Target> {
    let dylib_pkg = self::find_dylib_pkg(main_metadata, dylib_toml)?;

    let target = dylib_pkg
        .targets
        .iter()
        // TODO: allow `cdylib`?
        .find(|target| target.crate_types.iter().any(|t| t == "dylib"))
        .ok_or_else(|| format!("Unable to find `dylib` target from {}", dylib_toml))?;

    Ok(target)
}

fn find_dylib_path(main_metadata: &Metadata, dylib_toml: &Utf8Path) -> Result<Utf8PathBuf> {
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
