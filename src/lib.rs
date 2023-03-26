use std::{env, path::Path};

const RUSTC_WRAPPER_VAR: &str = "RUSTC_WRAPPER";

/// Run as a `rustc` wrapper (a la `$RUSTC_WRAPPER`/[`RUSTC_WRAPPER_VAR`]).
fn rustc_wrapper() -> anyhow::Result<()> {
    todo!()
}

/// Run as a `cargo` wrapper/plugin, the default invocation.
fn cargo_wrapper(_rustc_wrapper: &Path) -> anyhow::Result<()> {
    todo!()
}

/// Run the current binary as either a `cargo` or `rustc` wrapper.
pub fn cargo_rustc_wrapper() -> anyhow::Result<()> {
    let own_exe = env::current_exe()?;

    let wrapping_rustc = env::var_os(RUSTC_WRAPPER_VAR).as_deref() == Some(own_exe.as_os_str());
    if wrapping_rustc {
        rustc_wrapper()
    } else {
        cargo_wrapper(&own_exe)
    }
}
