use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::process::ExitStatus;

#[derive(PartialEq, Eq)]
struct EnvVar<V>
where
    V: AsRef<OsStr>,
{
    pub key: &'static str,
    pub value: V,
}

impl<V> EnvVar<V>
where
    V: AsRef<OsStr>,
{
    pub fn set_on(&self, cmd: &mut Command) {
        cmd.env(self.key, self.value.as_ref());
    }

    pub fn set(&self) {
        env::set_var(self.key, self.value.as_ref());
    }
}

impl EnvVar<OsString> {
    pub fn get_os(key: &'static str) -> Option<Self> {
        Some(Self {
            key,
            value: env::var_os(key)?,
        })
    }
}

impl EnvVar<String> {
    pub fn get(key: &'static str) -> Result<Self, env::VarError> {
        Ok(Self {
            key,
            value: env::var(key)?,
        })
    }
}

impl EnvVar<PathBuf> {
    pub fn get_path(key: &'static str) -> Option<Self> {
        let EnvVar { key, value } = EnvVar::get_os(key)?;
        Some(Self {
            key,
            value: PathBuf::from(value),
        })
    }
}

type RustcWrapperEnvVar = EnvVar<PathBuf>;
type SysrootEnvVar = EnvVar<PathBuf>;
type ToolchainEnvVar = EnvVar<String>;

fn exit_with_status(status: ExitStatus) {
    process::exit(status.code().unwrap_or(1))
}

pub struct CargoWrapper {
    rustc_wrapper: RustcWrapperEnvVar,
    sysroot: SysrootEnvVar,
    toolchain: Option<ToolchainEnvVar>,
}

impl CargoWrapper {
    fn new(rustc_wrapper: PathBuf) -> anyhow::Result<Self> {
        todo!()
    }

    /// Set `$RUSTUP_TOOLCHAIN` to the toolchain channel specified in `rust-toolchain.toml`.
    /// This ensures that we use a toolchain compatible with the `rustc` private crates that we linked to.
    pub fn set_rustup_toolchain(&mut self, rust_toolchain_toml_str: &str) -> anyhow::Result<()> {
        let doc = rust_toolchain_toml_str.parse::<toml_edit::Document>()?;
        let channel = doc["toolchain"]["channel"].as_str();
        if let Some(toolchain) = channel {
            self.toolchain = Some(ToolchainEnvVar {
                key: "RUSTUP_TOOLCHAIN",
                value: toolchain.to_owned(),
            })
        }
        Ok(())
    }

    pub fn run_cargo(
        &self,
        f: impl FnOnce(&mut Command) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let path: PathBuf = env::var_os("CARGO")
            .unwrap_or_else(|| "cargo".into())
            .into();
        let mut cmd = Command::new(path);
        let cmd = &mut cmd;
        if let Some(toolchain) = &self.toolchain {
            toolchain.set_on(cmd);
        }
        f(cmd)?;
        let status = cmd.status()?;
        if !status.success() {
            eprintln!("error ({status}) running: {cmd:?}");
            exit_with_status(status);
        }
        Ok(())
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
