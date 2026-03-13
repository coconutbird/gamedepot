// Process execution — spawns steamcmd and captures output.

use std::path::Path;
use std::process::Command;

use crate::error::SteamCmdError;

/// Raw output from a steamcmd invocation.
#[derive(Debug, Clone)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

/// Run steamcmd at the given path with the provided arguments.
pub fn run(steamcmd_path: &Path, args: &[String]) -> Result<Output, SteamCmdError> {
    let output = Command::new(steamcmd_path)
        .args(args)
        .output()
        .map_err(SteamCmdError::Io)?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(SteamCmdError::NonZeroExit { code, stderr });
    }

    Ok(Output {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
