use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

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
