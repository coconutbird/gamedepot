use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use gamedepot::depot::{Depot, DepotError};
use gamedepot::steam::{Login, Platform, SteamDepot};

#[derive(Parser)]
#[command(name = "gamedepot", about = "Download Steam and GOG games")]
struct Cli {
    /// Download and install steamcmd to ~/steamcmd if not found.
    #[arg(long, global = true)]
    install_steamcmd: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download or update a game.
    Download {
        /// App ID to download.
        app_id: String,
        /// Directory to install into.
        #[arg(short, long)]
        dir: PathBuf,
        /// Validate existing files.
        #[arg(long, default_value_t = false)]
        validate: bool,
        /// Steam username (omit for anonymous).
        #[arg(short, long)]
        username: Option<String>,
        /// Steam password.
        #[arg(short, long)]
        password: Option<String>,
        /// Target platform override.
        #[arg(long)]
        platform: Option<String>,
    },
    /// Show remote app info.
    Info {
        /// App ID to query.
        app_id: String,
        /// Steam username (omit for anonymous).
        #[arg(short, long)]
        username: Option<String>,
        /// Steam password.
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Check local install status.
    Status {
        /// App ID to check.
        app_id: String,
        /// Steam username (omit for anonymous).
        #[arg(short, long)]
        username: Option<String>,
        /// Steam password.
        #[arg(short, long)]
        password: Option<String>,
    },
}

fn parse_login(username: Option<String>, password: Option<String>) -> Login {
    match (username, password) {
        (Some(u), Some(p)) => Login::Credentials {
            username: u,
            password: p,
        },
        _ => Login::Anonymous,
    }
}

fn parse_platform(platform: Option<String>) -> Option<Platform> {
    platform.map(|p| match p.to_lowercase().as_str() {
        "windows" | "win" => Platform::Windows,
        "macos" | "mac" | "osx" => Platform::MacOS,
        "linux" => Platform::Linux,
        other => {
            eprintln!("unknown platform: {other}, ignoring");
            Platform::Linux
        }
    })
}

fn get_depot(install: bool) -> Result<SteamDepot, DepotError> {
    if install {
        SteamDepot::install_or_locate()
    } else {
        SteamDepot::locate()
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Download {
            app_id,
            dir,
            validate,
            username,
            password,
            platform,
        } => {
            let login = parse_login(username, password);
            let mut depot = match get_depot(cli.install_steamcmd) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::FAILURE;
                }
            };

            depot = depot.with_login(login);

            if let Some(p) = parse_platform(platform) {
                depot = depot.with_platform(p);
            }

            depot.download(&app_id, &dir, validate)
        }
        Commands::Info {
            app_id,
            username,
            password,
        } => {
            let login = parse_login(username, password);
            let depot = match get_depot(cli.install_steamcmd) {
                Ok(d) => d.with_login(login),
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::FAILURE;
                }
            };

            match depot.app_info(&app_id) {
                Ok(info) => {
                    println!("App ID:    {}", info.app_id);
                    println!("Name:      {}", info.name.as_deref().unwrap_or("unknown"));
                    println!(
                        "Build ID:  {}",
                        info.build_id.as_deref().unwrap_or("unknown")
                    );
                    return ExitCode::SUCCESS;
                }
                Err(e) => Err(e),
            }
        }
        Commands::Status {
            app_id,
            username,
            password,
        } => {
            let login = parse_login(username, password);
            let depot = match get_depot(cli.install_steamcmd) {
                Ok(d) => d.with_login(login),
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::FAILURE;
                }
            };

            match depot.app_status(&app_id) {
                Ok(status) => {
                    println!("App ID:    {}", status.app_id);
                    println!("Name:      {}", status.name.as_deref().unwrap_or("unknown"));
                    println!("Installed: {}", status.installed);
                    println!(
                        "Build ID:  {}",
                        status.build_id.as_deref().unwrap_or("unknown")
                    );
                    if let Some(size) = status.size_on_disk {
                        println!("Size:      {size} bytes");
                    }
                    return ExitCode::SUCCESS;
                }
                Err(e) => Err(e),
            }
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
