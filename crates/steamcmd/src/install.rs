// Auto-install steamcmd by downloading from Valve's CDN and extracting to ~/steamcmd.

use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

use crate::error::SteamCmdError;

/// URL for the macOS steamcmd archive.
const URL_MACOS: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_osx.tar.gz";
/// URL for the Linux steamcmd archive.
const URL_LINUX: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz";
/// URL for the Windows steamcmd archive.
const URL_WINDOWS: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";

/// Returns the default install directory: `~/steamcmd`.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
pub fn default_install_dir() -> Result<PathBuf, SteamCmdError> {
    let home = home_dir()?;
    Ok(home.join("steamcmd"))
}

/// Returns the expected binary path inside the install directory.
#[must_use]
pub fn binary_path(install_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        install_dir.join("steamcmd.exe")
    } else {
        install_dir.join("steamcmd.sh")
    }
}

/// Download and extract steamcmd into `install_dir`.
///
/// Returns the path to the steamcmd binary. If the binary already exists,
/// this is a no-op.
///
/// # Errors
///
/// Returns an error if the download or extraction fails.
pub fn install(install_dir: &Path) -> Result<PathBuf, SteamCmdError> {
    let bin = binary_path(install_dir);
    if bin.exists() {
        return Ok(bin);
    }

    fs::create_dir_all(install_dir).map_err(SteamCmdError::Io)?;

    let url = download_url();
    eprintln!("Downloading steamcmd from {url}...");

    let bytes = download(url)?;

    if cfg!(target_os = "windows") {
        extract_zip(&bytes, install_dir)?;
    } else {
        extract_tar_gz(&bytes, install_dir)?;
    }

    if !bin.exists() {
        return Err(SteamCmdError::InstallFailed(
            "binary not found after extraction".into(),
        ));
    }

    // Ensure the binary is executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&bin, perms).map_err(SteamCmdError::Io)?;
    }

    eprintln!("Installed steamcmd to {}", bin.display());
    Ok(bin)
}

fn download_url() -> &'static str {
    if cfg!(target_os = "macos") {
        URL_MACOS
    } else if cfg!(target_os = "windows") {
        URL_WINDOWS
    } else {
        URL_LINUX
    }
}

fn download(url: &str) -> Result<Vec<u8>, SteamCmdError> {
    let bytes = reqwest::blocking::get(url)
        .map_err(|e| SteamCmdError::InstallFailed(format!("download failed: {e}")))?
        .bytes()
        .map_err(|e| SteamCmdError::InstallFailed(format!("download failed: {e}")))?
        .to_vec();

    Ok(bytes)
}

fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<(), SteamCmdError> {
    let gz = flate2::read::GzDecoder::new(Cursor::new(data));
    let mut archive = tar::Archive::new(gz);
    archive.unpack(dest).map_err(SteamCmdError::Io)?;
    Ok(())
}

fn extract_zip(data: &[u8], dest: &Path) -> Result<(), SteamCmdError> {
    let reader = Cursor::new(data);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| SteamCmdError::InstallFailed(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| SteamCmdError::InstallFailed(e.to_string()))?;

        let Some(path) = file.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(path);

        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(SteamCmdError::Io)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(SteamCmdError::Io)?;
            }
            let mut out_file = fs::File::create(&out_path).map_err(SteamCmdError::Io)?;
            io::copy(&mut file, &mut out_file).map_err(SteamCmdError::Io)?;
        }
    }
    Ok(())
}

fn home_dir() -> Result<PathBuf, SteamCmdError> {
    #[allow(deprecated)]
    std::env::home_dir()
        .ok_or_else(|| SteamCmdError::InstallFailed("could not determine home directory".into()))
}
