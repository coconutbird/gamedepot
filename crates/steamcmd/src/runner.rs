// Process execution — spawns steamcmd inside a pseudo-terminal so
// that its output is line-buffered (same as a real terminal).

use std::io::{Read, Write};
use std::path::Path;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::error::SteamCmdError;
use crate::{Login, Platform};

/// The prompt string steamcmd prints when ready for input.
const PROMPT: &str = "Steam>";

/// Substrings that indicate Steam Guard is active during login.
const STEAM_GUARD_HINTS: &[&str] = &["steam guard", "mobile authenticator", "confirm the login"];

/// A long-lived steamcmd session.
///
/// Spawns the steamcmd process inside a pseudo-terminal so that its
/// output arrives with the same buffering as a real terminal. Commands
/// are sent via the pty writer and output is read byte-by-byte until
/// the `Steam>` prompt appears.
pub struct Session {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
}

impl Session {
    /// Spawn steamcmd and log in.
    ///
    /// The child process runs inside a pseudo-terminal so that its
    /// output is line-buffered (matching real-terminal behaviour).
    /// If Steam Guard is active, `on_auth_prompt` is called with the
    /// prompt text so the caller can inform the user (e.g. "confirm
    /// on your phone"). Steamcmd then waits for the user to approve
    /// via the Steam Mobile app and continues automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be spawned or login fails.
    pub fn start(
        steamcmd_path: &Path,
        login: &Login,
        platform: Option<Platform>,
        on_auth_prompt: Option<&mut dyn FnMut(&str)>,
    ) -> Result<Self, SteamCmdError> {
        let mut cmd = CommandBuilder::new(steamcmd_path);

        // Platform override must come before anything else.
        if let Some(p) = platform {
            cmd.arg("+@sSteamCmdForcePlatformType");
            cmd.arg(p.as_steamcmd_str());
        }

        // Login as part of the initial launch args so steamcmd
        // handles the authentication flow before presenting a prompt.
        cmd.arg("+login");
        match login {
            Login::Anonymous => {
                cmd.arg("anonymous");
            }
            Login::Credentials { username, password } => {
                cmd.arg(username);
                cmd.arg(password);
            }
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 200,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| SteamCmdError::Other(format!("failed to open pty: {e}")))?;

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| SteamCmdError::Other(format!("failed to spawn steamcmd: {e}")))?;

        // Drop the slave side — we only need the master.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| SteamCmdError::Other(format!("failed to clone pty reader: {e}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| SteamCmdError::Other(format!("failed to take pty writer: {e}")))?;

        let mut session = Self {
            child,
            reader,
            writer,
        };

        // Read until the first prompt — this consumes the login output.
        // If Steam Guard is required, handle the auth prompt.
        session.read_login_output(on_auth_prompt)?;

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

    /// Write a command to the pty.
    fn send_command(&mut self, command: &str) -> Result<(), SteamCmdError> {
        writeln!(self.writer, "{command}").map_err(SteamCmdError::Io)?;
        self.writer.flush().map_err(SteamCmdError::Io)?;
        Ok(())
    }

    /// Send `quit` and wait for the process to exit.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be waited on.
    pub fn quit(mut self) -> Result<(), SteamCmdError> {
        let _ = writeln!(self.writer, "quit");
        let _ = self.writer.flush();
        // Drain the pty reader so steamcmd doesn't block writing its
        // shutdown output to a full buffer.
        let mut sink = [0u8; 1024];
        while self.reader.read(&mut sink).unwrap_or(0) > 0 {}
        self.child
            .wait()
            .map_err(|e| SteamCmdError::Other(format!("failed to wait on child: {e}")))?;
        Ok(())
    }

    /// Read login output, detecting Steam Guard prompts. Reads
    /// byte-by-byte looking for the `Steam>` prompt (login succeeded).
    /// If a Steam Guard message is detected, the `on_auth_prompt`
    /// callback is invoked so the caller can inform the user. No stdin
    /// input is sent — steamcmd waits for the user to approve via the
    /// Steam Mobile app and then continues automatically.
    fn read_login_output(
        &mut self,
        mut on_auth_prompt: Option<&mut dyn FnMut(&str)>,
    ) -> Result<String, SteamCmdError> {
        let mut buf = Vec::new();
        let mut output = String::new();
        let mut byte = [0u8; 1];
        let prompt_bytes = PROMPT.as_bytes();
        let mut notified = false;

        loop {
            let n = self.reader.read(&mut byte).map_err(SteamCmdError::Io)?;
            if n == 0 {
                if !buf.is_empty() {
                    output.push_str(&String::from_utf8_lossy(&buf));
                }
                break;
            }
            buf.push(byte[0]);

            // Check if we've reached the `Steam>` prompt.
            if buf.len() >= prompt_bytes.len()
                && buf[buf.len() - prompt_bytes.len()..] == *prompt_bytes
            {
                buf.truncate(buf.len() - prompt_bytes.len());
                if !buf.is_empty() {
                    output.push_str(&String::from_utf8_lossy(&buf));
                }
                break;
            }

            // On each newline, flush the buffer and check for Steam
            // Guard messages.
            if byte[0] == b'\n' {
                let line = String::from_utf8_lossy(&buf).to_string();
                output.push_str(&line);

                // Notify the caller once when we see a Steam Guard hint.
                if !notified {
                    let lower = line.to_lowercase();
                    let is_guard = STEAM_GUARD_HINTS.iter().any(|h| lower.contains(h));
                    if is_guard {
                        notified = true;
                        if let Some(ref mut handler) = on_auth_prompt {
                            handler(line.trim());
                        }
                    }
                }

                buf.clear();
            }
        }

        Ok(output)
    }

    /// Read bytes from stdout until we see the `Steam>` prompt.
    ///
    /// Returns everything before the prompt. The prompt itself is
    /// consumed but not included in the output.
    fn read_until_prompt(&mut self) -> Result<String, SteamCmdError> {
        self.read_until_prompt_with_callback(&mut |_| {})
    }

    /// Read bytes from the pty until the `Steam>` prompt, calling
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
            let n = self.reader.read(&mut byte).map_err(SteamCmdError::Io)?;
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
        let _ = writeln!(self.writer, "quit");
        let _ = self.writer.flush();
        // Drain the pty reader so steamcmd doesn't block writing its
        // shutdown output to a full buffer.
        let mut sink = [0u8; 1024];
        while self.reader.read(&mut sink).unwrap_or(0) > 0 {}
        let _ = self.child.wait();
    }
}
