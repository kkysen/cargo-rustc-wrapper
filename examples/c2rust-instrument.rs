use std::borrow::Borrow;
use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::iter;
use std::mem;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use clap::Parser;
use tempfile::NamedTempFile;

use cargo_rustc_wrapper::wrap_cargo_or_rustc;
use cargo_rustc_wrapper::CargoRustcWrapper;
use cargo_rustc_wrapper::CargoWrapper;
use cargo_rustc_wrapper::RustcWrapper;

const METADATA_VAR: &str = "C2RUST_INSTRUMENT_METADATA_PATH";

fn instrument(at_args: &[String]) -> anyhow::Result<()> {
    println!("instrument: {at_args:?}");
    Ok(())
}

fn finalize(path: &Path) -> anyhow::Result<()> {
    println!("finalize: {path:?}");
    Ok(())
}

pub struct MetadataFile {
    path: PathBuf,

    file: NamedTempFile,
}

impl MetadataFile {
    pub fn final_path(&self) -> &Path {
        &self.path
    }

    pub fn temp_path(&self) -> &Path {
        self.file.path()
    }

    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let metadata_file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("--metadata has no file name: {}", path.display()))?;
        let metadata_dir = path.parent();

        if let Some(metadata_dir) = metadata_dir {
            fs_err::create_dir_all(metadata_dir)?;
        }

        let metadata_dir = metadata_dir.unwrap_or_else(|| Path::new("."));

        let prefix = {
            let mut prefix = metadata_file_name.to_owned();
            prefix.push(".");
            prefix
        };
        let file = tempfile::Builder::new()
            .prefix(&prefix)
            .suffix(".new")
            .tempfile_in(metadata_dir)
            .context("create new (temp) metadata file")?;
        Ok(Self { path, file })
    }

    pub fn close(self) -> anyhow::Result<()> {
        if self.file.as_file().metadata()?.len() > 0 {
            fs_err::rename(self.file.path(), &self.path)?;
        } else {
            self.file.close()?;
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Instrument {
    #[clap(long, value_parser)]
    metadata: PathBuf,

    #[clap(long, value_parser)]
    runtime_path: Option<PathBuf>,

    #[clap(long)]
    set_runtime: bool,

    #[clap(long)]
    rustflags: Option<OsString>,

    /// `cargo` args.
    cargo_args: Vec<OsString>,
}

fn add_feature(cargo_args: &mut Vec<OsString>, features: &[&str]) {
    let insertion_point = 1;
    if cargo_args.len() >= insertion_point {
        cargo_args.splice(
            insertion_point..insertion_point,
            iter::once(&"--features").chain(features).map(|s| s.into()),
        );
    }
}

trait OsStringJoin {
    fn join(&mut self, sep: &OsStr) -> OsString;
}

impl<I, T> OsStringJoin for I
where
    I: Iterator<Item = T>,
    T: Borrow<OsStr>,
{
    fn join(&mut self, sep: &OsStr) -> OsString {
        match self.next() {
            None => OsString::new(),
            Some(first_elt) => {
                // estimate lower bound of capacity needed
                let (lower, _) = self.size_hint();
                let mut result = OsString::with_capacity(sep.len() * lower);
                result.push(first_elt.borrow());
                self.for_each(|elt| {
                    result.push(sep);
                    result.push(elt.borrow());
                });
                result
            }
        }
    }
}

fn env_path_from_wrapper(var: &str) -> anyhow::Result<PathBuf> {
    let path = env::var_os(var)
        .ok_or_else(|| anyhow!("the `cargo` wrapper should've `${var}` for the `rustc` wrapper"))?;
    Ok(path.into())
}

impl CargoRustcWrapper for Instrument {
    fn take_cargo_args(&mut self) -> Vec<OsString> {
        mem::take(&mut self.cargo_args)
    }

    fn wrap_cargo(self, mut wrapper: CargoWrapper) -> anyhow::Result<()> {
        let Self {
            metadata: metadata_path,
            runtime_path,
            set_runtime,
            rustflags,
            mut cargo_args,
        } = self;

        wrapper.set_rustup_toolchain(include_str!("../rust-toolchain.toml"))?;

        let manifest_path = wrapper.manifest_path();
        let manifest_dir = manifest_path.and_then(|path| path.parent());

        if set_runtime {
            wrapper.run_cargo(|cmd| {
                cmd.args(&["add", "--optional", "c2rust-analysis-rt"]);
                if let Some(mut runtime) = runtime_path {
                    if manifest_dir.is_some() {
                        runtime = fs_err::canonicalize(runtime)?;
                    }
                    cmd.args(&["--offline", "--path"]).arg(runtime);
                }
                if let Some(manifest_path) = manifest_path {
                    cmd.arg("--manifest-path").arg(manifest_path);
                }
                Ok(())
            })?;
        }

        let metadata_file = MetadataFile::new(metadata_path)?;

        wrapper.run_cargo_with_rustc_wrapper(|cmd| {
            let cargo_target_dir = manifest_dir
                .unwrap_or_else(|| Path::new("."))
                .join("instrument.target");

            let metadata_path = metadata_file.temp_path();
            let metadata_path = if !metadata_path.is_absolute() && manifest_dir.is_some() {
                Cow::Owned(fs_err::canonicalize(metadata_path)?)
            } else {
                Cow::Borrowed(metadata_path)
            };

            let rustflags = [
                env::var_os("RUSTFLAGS"),
                Some("-A warnings".into()),
                rustflags,
            ]
            .into_iter()
            .flatten()
            .join(OsStr::new(" "));

            add_feature(&mut cargo_args, &["c2rust-analysis-rt"]);

            cmd.args(cargo_args)
                .env("CARGO_TARGET_DIR", &cargo_target_dir)
                .env("RUSTFLAGS", &rustflags)
                .env(METADATA_VAR, metadata_path.as_ref());
            Ok(())
        })?;
        Ok(())
    }

    fn wrap_rustc(wrapper: RustcWrapper) -> anyhow::Result<()> {
        let should_instrument = wrapper.is_primary_package() && !wrapper.is_build_script()?;
        if should_instrument {
            instrument(&wrapper.rustc_args()?)?;
        } else {
            wrapper.run_rustc()?;
        }
        if should_instrument {
            finalize(&env_path_from_wrapper(METADATA_VAR)?)?;
        }
        Ok(())
    }
}

pub fn main() -> anyhow::Result<()> {
    wrap_cargo_or_rustc::<Instrument>()
}
