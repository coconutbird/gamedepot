// Process execution — spawns steamcmd and captures output.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::error::SteamCmdError;
use crate::{Login, Platform};

/// A long-lived steamcmd session.
///
/// Spawns the steamcmd process once, handles login, and then accepts
/// commands via stdin. Output is read from stdout line-by-line until
/// the `Steam>` prompt reappears, indicating the command has finished.
pub struct Session {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Session {
    /// Spawn steamcmd and log in.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be spawned or login fails.
    pub fn start(
        steamcmd_path: &Path,
        login: &Login,
        platform: Option<Platform>,
    ) -> Result<Self, SteamCmdError> {
        let mut initial_args: Vec<String> = Vec::new();

        // Platform override must come before anything else.
        if let Some(p) = platform {
            initial_args.push("+@sSteamCmdForcePlatformType".into());
            initial_args.push(p.as_steamcmd_str().into());
        }

        // Login as part of the initial launch args so steamcmd
        // handles the authentication flow before presenting a prompt.
        initial_args.push("+login".into());
        match login {
            Login::Anonymous => initial_args.push("anonymous".into()),
            Login::Credentials { username, password } => {
                initial_args.push(username.clone());
                initial_args.push(password.clone());
            }
        }

        let mut child = Command::new(steamcmd_path)
            .args(&initial_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(SteamCmdError::Io)?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SteamCmdError::Other("failed to capture stdout".into()))?;

        let reader = BufReader::new(stdout);

        let mut session = Self { child, reader };

        // Read until the first prompt — this consumes the login output.
        let _login_output = session.read_until_prompt()?;

        Ok(session)
    }

    /// Send a command to the running steamcmd session and return its
    /// output up to the next `Steam>` prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to stdin or reading from stdout fails.
    pub fn run_command(&mut self, command: &str) -> Result<String, SteamCmdError> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| SteamCmdError::Other("stdin not available".into()))?;

        writeln!(stdin, "{command}").map_err(SteamCmdError::Io)?;
        stdin.flush().map_err(SteamCmdError::Io)?;

        self.read_until_prompt()
    }

    /// Send `quit` and wait for the process to exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be waited on.
    pub fn quit(mut self) -> Result<(), SteamCmdError> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = writeln!(stdin, "quit");
            let _ = stdin.flush();
        }
        // Drop stdin so the process sees EOF.
        drop(self.child.stdin.take());

        self.child.wait().map_err(SteamCmdError::Io)?;
        Ok(())
    }

    /// Read lines from stdout until we see the `Steam>` prompt.
    fn read_until_prompt(&mut self) -> Result<String, SteamCmdError> {
        let mut output = String::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self
                .reader
                .read_line(&mut line)
                .map_err(SteamCmdError::Io)?;

            if bytes_read == 0 {
                // EOF — process exited.
                break;
            }

            let trimmed = line.trim();

            // The steamcmd prompt is "Steam>" at the start of a line.
            if trimmed == "Steam>" || trimmed.ends_with("Steam>") {
                break;
            }

            output.push_str(&line);
        }

        Ok(output)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Best-effort: send quit if stdin is still open.
        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = writeln!(stdin, "quit");
            let _ = stdin.flush();
        }
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}
