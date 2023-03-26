use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

pub struct CargoWrapper {
    rustc_wrapper: PathBuf,
    sysroot: PathBuf,
}

impl CargoWrapper {
    fn new(rustc_wrapper: PathBuf) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn set_rust_toolchain(rust_toolchain_toml_str: &str) {
        todo!()
    }

    pub fn run_cargo(
        &self,
        f: impl FnOnce(&mut Command) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        todo!()
    }

    pub fn run_cargo_with_rustc_wrapper(
        &self,
        f: impl FnOnce(&mut Command) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

pub struct RustcWrapper {
    args: Vec<OsString>,
    sysroot: PathBuf,
}

impl RustcWrapper {
    fn new() -> anyhow::Result<Self> {
        todo!()
    }

    pub fn is_primary_package(&self) -> bool {
        todo!()
    }

    pub fn is_bin_crate(&self) -> anyhow::Result<bool> {
        todo!()
    }

    pub fn bin_crate_name(&self) -> Option<PathBuf> {
        todo!()
    }

    pub fn is_build_script(&self) -> anyhow::Result<bool> {
        todo!()
    }

    pub fn run_rustc(&self) {
        todo!()
    }
}

pub trait CargoRustcWrapper {
    /// Run as a `cargo` wrapper/plugin, the default invocation.
    fn wrap_cargo(&self, wrapper: CargoWrapper) -> anyhow::Result<()>;

    /// Run as a `rustc` wrapper (a la `$RUSTC_WRAPPER`/[`RUSTC_WRAPPER_VAR`]).
    fn wrap_rustc(&self, wrapper: RustcWrapper) -> anyhow::Result<()>;
}

const RUSTC_WRAPPER_VAR: &str = "RUSTC_WRAPPER";

/// Run the current binary as either a `cargo` or `rustc` wrapper.
pub fn wrap_cargo_or_rustc(wrapper: impl CargoRustcWrapper) -> anyhow::Result<()> {
    let own_exe = env::current_exe()?;

    let wrapping_rustc = env::var_os(RUSTC_WRAPPER_VAR).as_deref() == Some(own_exe.as_os_str());
    if wrapping_rustc {
        wrapper.wrap_rustc(RustcWrapper::new()?)
    } else {
        wrapper.wrap_cargo(CargoWrapper::new(own_exe)?)
    }
}
