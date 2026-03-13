use std::path::{Path, PathBuf};
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
    /// Search the Steam store for games.
    Search {
        /// Search query (game name).
        query: String,
    },
    /// List locally installed games.
    List,
    /// Validate an installed game's files.
    Validate {
        /// App ID to validate.
        app_id: String,
        /// Directory where the game is installed.
        #[arg(short, long)]
        dir: PathBuf,
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
    /// Update an installed game to the latest version.
    Update {
        /// App ID to update.
        app_id: String,
        /// Directory where the game is installed.
        #[arg(short, long)]
        dir: PathBuf,
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
    /// Download and install `SteamCMD` to ~/steamcmd.
    InstallSteamcmd,
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

fn build_depot(
    install: bool,
    username: Option<String>,
    password: Option<String>,
    platform: Option<String>,
) -> Result<SteamDepot, DepotError> {
    let login = parse_login(username, password);
    let mut depot = get_depot(install)?.with_login(login);
    if let Some(p) = parse_platform(platform) {
        depot = depot.with_platform(p);
    }
    Ok(depot)
}

fn cmd_download(depot: &mut SteamDepot, app_id: &str, dir: &Path, validate: bool) -> ExitCode {
    let bar = indicatif::ProgressBar::new(0);
    bar.set_style(
        indicatif::ProgressStyle::with_template(
            "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        )
        .expect("valid template")
        .progress_chars("━╸━"),
    );

    match depot.download_with_progress(
        app_id,
        dir,
        validate,
        |info| {
            bar.println(format!(
                "{} (app {}, build {})",
                info.name.as_deref().unwrap_or("unknown"),
                info.app_id,
                info.build_id.as_deref().unwrap_or("?"),
            ));
        },
        |p| {
            bar.set_length(p.total_bytes);
            bar.set_position(p.current_bytes);
            bar.set_message(p.state.to_string());
        },
    ) {
        Ok(_) => {
            bar.finish_with_message("done");
            ExitCode::SUCCESS
        }
        Err(e) => {
            bar.abandon_with_message("failed");
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_info(depot: &mut SteamDepot, app_id: &str) -> ExitCode {
    match depot.app_info(app_id) {
        Ok(info) => {
            println!("App ID:    {}", info.app_id);
            println!("Name:      {}", info.name.as_deref().unwrap_or("unknown"));
            println!(
                "Build ID:  {}",
                info.build_id.as_deref().unwrap_or("unknown")
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_status(depot: &mut SteamDepot, app_id: &str) -> ExitCode {
    match depot.app_status(app_id) {
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
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_search(query: &str) -> ExitCode {
    match SteamDepot::search_store(query) {
        Ok(results) => {
            if results.is_empty() {
                println!("No results found.");
            } else {
                for r in &results {
                    let platforms = format_platforms(r.windows, r.macos, r.linux);
                    println!("{:<10} {} [{}]", r.app_id, r.name, platforms);
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_list() -> ExitCode {
    match SteamDepot::list_installed() {
        Ok(apps) => {
            if apps.is_empty() {
                println!("No installed apps found.");
            } else {
                for app in &apps {
                    let name = app.name.as_deref().unwrap_or("unknown");
                    let status = if app.installed {
                        "installed"
                    } else {
                        "incomplete"
                    };
                    println!("{:<10} {} [{}]", app.app_id, name, status);
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_install_steamcmd() -> ExitCode {
    match SteamDepot::install() {
        Ok(_) => {
            println!("steamcmd installed to ~/steamcmd");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Download {
            app_id,
            dir,
            validate,
            username,
            password,
            platform,
        } => match build_depot(cli.install_steamcmd, username, password, platform) {
            Ok(mut depot) => cmd_download(&mut depot, &app_id, &dir, validate),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::Info {
            app_id,
            username,
            password,
        } => match build_depot(cli.install_steamcmd, username, password, None) {
            Ok(mut depot) => cmd_info(&mut depot, &app_id),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::Status {
            app_id,
            username,
            password,
        } => match build_depot(cli.install_steamcmd, username, password, None) {
            Ok(mut depot) => cmd_status(&mut depot, &app_id),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::Search { query } => cmd_search(&query),
        Commands::List => cmd_list(),
        Commands::Validate {
            app_id,
            dir,
            username,
            password,
            platform,
        } => match build_depot(cli.install_steamcmd, username, password, platform) {
            Ok(mut depot) => cmd_download(&mut depot, &app_id, &dir, true),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::Update {
            app_id,
            dir,
            username,
            password,
            platform,
        } => match build_depot(cli.install_steamcmd, username, password, platform) {
            Ok(mut depot) => cmd_download(&mut depot, &app_id, &dir, false),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::InstallSteamcmd => cmd_install_steamcmd(),
    }
}

#[must_use]
fn format_platforms(windows: bool, macos: bool, linux: bool) -> String {
    let mut parts = Vec::new();
    if windows {
        parts.push("Win");
    }

    if macos {
        parts.push("Mac");
    }

    if linux {
        parts.push("Linux");
    }

    if parts.is_empty() {
        return "none".to_string();
    }

    parts.join(", ")
}
