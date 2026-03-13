// Command builder — assembles steamcmd argument lists without running anything.

use std::path::Path;

use crate::{Login, Platform};

/// A builder for steamcmd command-line arguments.
#[derive(Debug, Clone)]
#[must_use]
pub struct CommandBuilder {
    args: Vec<String>,
}

impl CommandBuilder {
    /// Create a new empty command builder.
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    /// Add a platform override. Should be called first.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.args.push("+@sSteamCmdForcePlatformType".into());
        self.args.push(platform.as_steamcmd_str().into());
        self
    }

    /// Add an optional platform override.
    pub fn maybe_platform(self, platform: Option<Platform>) -> Self {
        match platform {
            Some(p) => self.platform(p),
            None => self,
        }
    }

    /// Set the install directory. Should come before login.
    pub fn force_install_dir(mut self, dir: &Path) -> Self {
        self.args.push("+force_install_dir".into());
        self.args.push(dir.to_string_lossy().into_owned());
        self
    }

    /// Add login arguments.
    pub fn login(mut self, login: &Login) -> Self {
        self.args.push("+login".into());
        match login {
            Login::Anonymous => {
                self.args.push("anonymous".into());
            }
            Login::Credentials { username, password } => {
                self.args.push(username.clone());
                self.args.push(password.clone());
            }
        }
        self
    }

    /// Add `app_update` with optional validate.
    pub fn app_update(mut self, app_id: &str, validate: bool) -> Self {
        self.args.push("+app_update".into());
        self.args.push(app_id.into());
        if validate {
            self.args.push("-validate".into());
        }
        self
    }

    /// Add `app_info_update` to refresh the local app info cache.
    pub fn app_info_update(mut self) -> Self {
        self.args.push("+app_info_update".into());
        self.args.push("1".into());
        self
    }

    /// Add `app_info_print` for the given app ID.
    pub fn app_info_print(mut self, app_id: &str) -> Self {
        self.args.push("+app_info_print".into());
        self.args.push(app_id.into());
        self
    }

    /// Add `app_status` for the given app ID.
    pub fn app_status(mut self, app_id: &str) -> Self {
        self.args.push("+app_status".into());
        self.args.push(app_id.into());
        self
    }

    /// Add `+quit` to terminate steamcmd.
    pub fn quit(mut self) -> Self {
        self.args.push("+quit".into());
        self
    }

    /// Consume the builder and return the argument list.
    #[must_use]
    pub fn build(self) -> Vec<String> {
        self.args
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}
