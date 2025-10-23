use crate::common::config::Command;
use miette::IntoDiagnostic;
use miette::Result;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use std::{
    fmt::{self, Display},
    fs::{set_permissions, File},
    io::{Error, Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Script(String);
impl Script {
    fn checksum(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.0.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub(crate) fn script_to_path(&self) -> Result<PathBuf> {
        let checksum = self.checksum();
        let mut path = std::env::temp_dir();
        path.push(format!("laio-{checksum}"));

        if path.exists() {
            let mut file = File::open(&path).into_diagnostic()?;
            let mut existing = Vec::new();
            file.read_to_end(&mut existing).into_diagnostic()?;

            let mut hasher = Sha256::new();
            hasher.update(&existing);
            let existing_checksum = format!("{:x}", hasher.finalize());

            (existing_checksum == checksum)
                .then_some(())
                .ok_or_else(|| {
                    Error::other(format!(
                        "Checksum mismatch for cached script at {}",
                        path.display()
                    ))
                })
                .into_diagnostic()?;
        } else {
            let mut file = File::create(&path).into_diagnostic()?;
            file.write_all(self.0.as_bytes()).into_diagnostic()?;
            set_permissions(&path, PermissionsExt::from_mode(0o700)).into_diagnostic()?;
        }

        Ok(path)
    }

    pub(crate) fn to_cmd(&self) -> Result<Command> {
        Ok(Command {
            command: self.script_to_path()?.to_string_lossy().to_string(),
            args: vec![],
        })
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
