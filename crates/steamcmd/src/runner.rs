// Process execution — spawns steamcmd and captures output.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::error::SteamCmdError;
use crate::{Login, Platform};

/// The prompt string steamcmd prints when ready for input.
const PROMPT: &str = "Steam>";

/// A long-lived steamcmd session.
///
/// Spawns the steamcmd process once, handles login, and then accepts
/// commands via stdin. Output is read byte-by-byte until the `Steam>`
/// prompt appears, indicating the command has finished.
pub struct Session {
    child: Child,
    stdout: std::process::ChildStdout,
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

        let mut session = Self { child, stdout };

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
        self.send_command(command)?;
        self.read_until_prompt()
    }

    /// Send a command and stream each output line to a callback as it
    /// arrives. Returns the full collected output when the prompt appears.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to stdin or reading from stdout fails.
    pub fn run_command_with_callback(
        &mut self,
        command: &str,
        mut on_line: impl FnMut(&str),
    ) -> Result<String, SteamCmdError> {
        self.send_command(command)?;
        self.read_until_prompt_with_callback(&mut on_line)
    }

    /// Write a command to the child's stdin.
    fn send_command(&mut self, command: &str) -> Result<(), SteamCmdError> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| SteamCmdError::Other("stdin not available".into()))?;

        writeln!(stdin, "{command}").map_err(SteamCmdError::Io)?;
        stdin.flush().map_err(SteamCmdError::Io)?;
        Ok(())
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

    /// Read bytes from stdout until we see the `Steam>` prompt.
    ///
    /// Returns everything before the prompt. The prompt itself is
    /// consumed but not included in the output.
    fn read_until_prompt(&mut self) -> Result<String, SteamCmdError> {
        self.read_until_prompt_with_callback(&mut |_| {})
    }

    /// Read bytes from stdout until the `Steam>` prompt, calling
    /// `on_line` for each complete line as it arrives.
    fn read_until_prompt_with_callback(
        &mut self,
        on_line: &mut impl FnMut(&str),
    ) -> Result<String, SteamCmdError> {
        let mut buf = Vec::new();
        let mut output = String::new();
        let mut byte = [0u8; 1];
        let prompt_bytes = PROMPT.as_bytes();

        loop {
            let n = self.stdout.read(&mut byte).map_err(SteamCmdError::Io)?;
            if n == 0 {
                // EOF — flush any remaining partial line.
                if !buf.is_empty() {
                    let line = String::from_utf8_lossy(&buf);
                    on_line(&line);
                    output.push_str(&line);
                }
                break;
            }
            buf.push(byte[0]);

            // Check if the buffer ends with the prompt.
            if buf.len() >= prompt_bytes.len()
                && buf[buf.len() - prompt_bytes.len()..] == *prompt_bytes
            {
                // Remove the prompt from the buffer.
                buf.truncate(buf.len() - prompt_bytes.len());
                // Flush any remaining partial line before the prompt.
                if !buf.is_empty() {
                    let line = String::from_utf8_lossy(&buf);
                    on_line(&line);
                    output.push_str(&line);
                }
                break;
            }

            // When we see a newline, flush the completed line.
            if byte[0] == b'\n' {
                let line = String::from_utf8_lossy(&buf);
                on_line(&line);
                output.push_str(&line);
                buf.clear();
            }
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
