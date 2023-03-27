use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::process::ExitStatus;

use anyhow::anyhow;
use anyhow::ensure;
use anyhow::Context;

use crate::util::os_str_from_bytes;
use crate::util::EnvVar;

mod util;

type RustcWrapperEnvVar = EnvVar<PathBuf>;
type SysrootEnvVar = EnvVar<PathBuf>;
type ToolchainEnvVar = EnvVar<String>;

const RUSTC_WRAPPER_VAR: &str = "RUSTC_WRAPPER";
const SYSROOT_VAR: &str = "RUST_SYSROOT";
const TOOLCHAIN_VAR: &str = "RUSTUP_TOOLCHAIN";

fn exit_with_status(status: ExitStatus) {
    process::exit(status.code().unwrap_or(1))
}

struct WrappedCommand {
    path: PathBuf,
}

impl WrappedCommand {
    pub fn new(program: impl Into<PathBuf>, env_var: impl AsRef<OsStr>) -> Self {
        let path = env::var_os(env_var)
            .map(PathBuf::from)
            .unwrap_or_else(|| program.into());
        Self { path }
    }

    pub fn command(&self) -> Command {
        Command::new(&self.path)
    }

    pub fn run(&self, f: impl FnOnce(&mut Command) -> anyhow::Result<()>) -> anyhow::Result<()> {
        let mut cmd = self.command();
        f(&mut cmd)?;
        let status = cmd.status()?;
        if !status.success() {
            eprintln!("error ({status}) running: {cmd:?}");
            exit_with_status(status);
        }
        Ok(())
    }

    pub fn cargo() -> Self {
        Self::new("cargo", "CARGO")
    }

    pub fn rustc() -> Self {
        Self::new("rustc", "RUSTC")
    }
}

fn resolve_sysroot() -> anyhow::Result<PathBuf> {
    let rustc = WrappedCommand::rustc();
    let output = rustc
        .command()
        .args(&["--print", "sysroot"])
        .output()
        .context("could not invoke `rustc` to find rust sysroot")?;
    let path = output
        .stdout
        .as_slice()
        // .lines() // can't use `.lines()` here since that enforces UTF-8
        .split(|c| c.is_ascii_whitespace())
        .next()
        .unwrap_or_default();
    let path = os_str_from_bytes(path)?;
    let path = Path::new(path).to_owned();
    // `rustc` reports a million errors if the sysroot is wrong, so try to check first.
    ensure!(
        path.is_dir(),
        "invalid sysroot (not a dir): {}",
        path.display()
    );
    Ok(path)
}

pub struct CargoWrapper {
    rustc_wrapper: RustcWrapperEnvVar,
    sysroot: SysrootEnvVar,
    toolchain: Option<ToolchainEnvVar>,
}

impl CargoWrapper {
    fn new(rustc_wrapper: RustcWrapperEnvVar) -> anyhow::Result<Self> {
        Ok(Self {
            rustc_wrapper,
            sysroot: SysrootEnvVar {
                key: SYSROOT_VAR,
                value: resolve_sysroot()?,
            },
            toolchain: None,
        })
    }

    /// Set `$RUSTUP_TOOLCHAIN` to the toolchain channel specified in `rust-toolchain.toml`.
    /// This ensures that we use a toolchain compatible with the `rustc` private crates that we linked to.
    pub fn set_rustup_toolchain(&mut self, rust_toolchain_toml_str: &str) -> anyhow::Result<()> {
        let doc = rust_toolchain_toml_str.parse::<toml_edit::Document>()?;
        let channel = doc["toolchain"]["channel"].as_str();
        if let Some(toolchain) = channel {
            self.toolchain = Some(ToolchainEnvVar {
                key: TOOLCHAIN_VAR,
                value: toolchain.to_owned(),
            })
        }
        Ok(())
    }

    pub fn run_cargo(
        &self,
        f: impl FnOnce(&mut Command) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        WrappedCommand::cargo().run(|cmd| {
            if let Some(toolchain) = &self.toolchain {
                toolchain.set_on(cmd);
            }
            f(cmd)?;
            Ok(())
        })
    }

    pub fn run_cargo_with_rustc_wrapper(
        &self,
        f: impl FnOnce(&mut Command) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        self.run_cargo(|cmd| {
            self.rustc_wrapper.set_on(cmd);
            self.sysroot.set_on(cmd);
            f(cmd)
        })
    }
}

fn os_string_utf8_error(s: OsString) -> anyhow::Error {
    anyhow!("non-UTF-8 OsString: {s:?}")
}

pub struct RustcWrapper {
    args: Vec<OsString>,
    sysroot: EnvVar<PathBuf>,
}

impl RustcWrapper {
    fn new() -> anyhow::Result<Self> {
        let args = env::args_os().skip(1).collect::<Vec<_>>();
        let sysroot = SysrootEnvVar::get_path(SYSROOT_VAR).ok_or_else(|| {
            anyhow!("the `cargo` wrapper should've set `${SYSROOT_VAR}` for the `rustc` wrapper")
        })?;
        Ok(Self { args, sysroot })
    }

    pub fn is_primary_package(&self) -> bool {
        EnvVar::get_os("CARGO_PRIMARY_PACKAGE").is_some()
    }

    pub fn is_bin_crate(&self) -> anyhow::Result<bool> {
        todo!()
    }

    pub fn bin_crate_name(&self) -> Option<PathBuf> {
        EnvVar::get_path("CARGO_BIN_NAME").map(|var| var.value)
    }

    pub fn is_build_script(&self) -> anyhow::Result<bool> {
        Ok(self.bin_crate_name().is_none() && self.is_bin_crate()?)
    }

    pub fn rustc_args_os(self) -> Vec<OsString> {
        let Self { mut args, sysroot } = self;
        let sysroot = sysroot.value;
        args.extend(["--sysroot".into(), sysroot.into()]);
        args
    }

    pub fn rustc_args(self) -> anyhow::Result<Vec<String>> {
        let Self { args, sysroot } = self;
        let mut args = args
            .into_iter()
            .map(|arg| arg.into_string())
            .collect::<Result<Vec<_>, _>>()
            .map_err(os_string_utf8_error)?;
        let sysroot = sysroot
            .value
            .into_os_string()
            .into_string()
            .map_err(os_string_utf8_error)?;
        args.extend(["--sysroot".into(), sysroot.into()]);
        Ok(args)
    }

    pub fn run_rustc(self) -> anyhow::Result<()> {
        todo!()
    }
}

pub trait CargoRustcWrapper {
    /// Run as a `cargo` wrapper/plugin, the default invocation.
    fn wrap_cargo(&self, wrapper: CargoWrapper) -> anyhow::Result<()>;

    /// Run as a `rustc` wrapper (a la `$RUSTC_WRAPPER`/[`RUSTC_WRAPPER_VAR`]).
    fn wrap_rustc(&self, wrapper: RustcWrapper) -> anyhow::Result<()>;
}

/// Run the current binary as either a `cargo` or `rustc` wrapper.
pub fn wrap_cargo_or_rustc(wrapper: impl CargoRustcWrapper) -> anyhow::Result<()> {
    let own_rustc_wrapper = RustcWrapperEnvVar {
        key: RUSTC_WRAPPER_VAR,
        value: env::current_exe()?,
    };
    let current_rustc_wrapper = EnvVar::get_path(own_rustc_wrapper.key);

    let wrapping_rustc = current_rustc_wrapper.as_ref() == Some(&own_rustc_wrapper);
    if wrapping_rustc {
        wrapper.wrap_rustc(RustcWrapper::new()?)
    } else {
        wrapper.wrap_cargo(CargoWrapper::new(own_rustc_wrapper)?)
    }
}
