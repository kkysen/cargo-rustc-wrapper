use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::str::Utf8Error;

#[derive(PartialEq, Eq)]
pub struct EnvVar<V>
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

/// Create an [`OsStr`] from bytes.
///
/// Where possible (i.e. `cfg(unix)`), do an `O(1)` unchecked conversion,
/// and fallback to checked conversion through UTF-8.
pub fn os_str_from_bytes(bytes: &[u8]) -> Result<&OsStr, Utf8Error> {
    #[cfg(not(unix))]
    fn convert(it: &[u8]) -> Result<&OsStr, Utf8Error> {
        let it = std::str::from_utf8(it)?;
        let it = OsStr::new(it);
        Ok(it)
    }

    #[cfg(unix)]
    fn convert(it: &[u8]) -> Result<&OsStr, Utf8Error> {
        use std::os::unix::ffi::OsStrExt;

        let it = OsStr::from_bytes(it);
        Ok(it)
    }

    convert(bytes)
}
