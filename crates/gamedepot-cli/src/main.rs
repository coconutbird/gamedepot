mod session;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password};
use gamedepot::depot::{Depot, DepotError};
use gamedepot::gog::GogDepot;
use gamedepot::manifest::{DepotKind, Install, Manifest};
use gamedepot::steam::{Login, Platform, SteamDepot, UpdateResult};

use session::Session;

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
    /// Steam commands.
    Steam {
        #[command(subcommand)]
        command: SteamCommands,
    },
    /// GOG commands.
    Gog {
        #[command(subcommand)]
        command: GogCommands,
    },
}

#[derive(Subcommand)]
enum SteamCommands {
    /// Save Steam credentials (API key, username/password, Steam ID).
    Login,
    /// Download or update a game.
    Download {
        /// App ID to download.
        app_id: String,
        /// Directory to install into (defaults to ~/gamedepot/steam/<name>/).
        #[arg(short, long)]
        dir: Option<PathBuf>,
        /// Target platform override.
        #[arg(long)]
        platform: Option<String>,
    },
    /// Show remote app info.
    Info {
        /// ID to query.
        id: String,
    },
    /// Check local install status.
    Status {
        /// App ID to check.
        id: String,
    },
    /// Search the Steam store for games.
    Search {
        /// Search query (game name).
        query: String,
    },
    /// Validate an installed game's files.
    Validate {
        /// App ID to validate.
        app_id: String,
        /// Directory where the game is installed.
        #[arg(short, long)]
        dir: PathBuf,
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
        /// Target platform override.
        #[arg(long)]
        platform: Option<String>,
    },
    /// List games you own (requires login).
    Owned {
        /// Steam ID or vanity URL name (uses saved ID if omitted).
        steam_id: Option<String>,
        /// Optional search filter.
        #[arg(short, long)]
        search: Option<String>,
    },
    /// Download and install `SteamCMD` to ~/steamcmd.
    InstallSteamcmd,
}

#[derive(Subcommand)]
enum GogCommands {
    /// Log in to GOG via browser-based `OAuth2`.
    Login,
    /// Search the GOG catalog.
    Search {
        /// Search query (game name).
        query: String,
    },
    /// Show product info.
    Info {
        /// ID to query.
        id: String,
    },
    /// Check local install status.
    Status {
        /// Product ID to check.
        id: String,
    },
    /// List games you own (requires login).
    Owned {
        /// Optional search filter.
        #[arg(short, long)]
        search: Option<String>,
        /// Page number (1-based).
        #[arg(short, long, default_value = "1")]
        page: u32,
    },
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

/// Build a `SteamDepot` with credentials from the session file.
fn build_depot(install: bool, platform: Option<String>) -> Result<SteamDepot, DepotError> {
    let session =
        Session::load().map_err(|e| DepotError::Other(format!("failed to load session: {e}")))?;
    let login = match (session.steam.username, session.steam.password) {
        (Some(u), Some(p)) => Login::Credentials {
            username: u,
            password: p,
        },
        _ => Login::Anonymous,
    };
    let mut depot = get_depot(install)?
        .with_login(login)
        .with_auth_handler(|_| {
            println!("Steam Guard: please confirm the login in your Steam Mobile app.");
        });
    if let Some(p) = parse_platform(platform) {
        depot = depot.with_platform(p);
    }
    Ok(depot)
}

/// Prompt the user for a line of input.
fn cmd_steam_login() -> ExitCode {
    let theme = ColorfulTheme::default();

    println!("Steam login — saves credentials to ~/.gamedepot/session.toml\n");

    let api_key: String = Input::with_theme(&theme)
        .with_prompt("Steam Web API key (https://steamcommunity.com/dev/apikey)")
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();

    let username: String = Input::with_theme(&theme)
        .with_prompt("Steam username")
        .allow_empty(true)
        .interact_text()
        .unwrap_or_default();

    let password: String = Password::with_theme(&theme)
        .with_prompt("Steam password")
        .allow_empty_password(true)
        .interact()
        .unwrap_or_default();

    // Resolve the 64-bit Steam ID by doing a quick steamcmd login.
    let steam_id = if !username.is_empty() && !password.is_empty() {
        println!("Resolving Steam ID via steamcmd...");
        let login = Login::Credentials {
            username: username.clone(),
            password: password.clone(),
        };
        match SteamDepot::install_or_locate() {
            Ok(depot) => {
                let mut depot = depot.with_login(login).with_auth_handler(|_| {
                    println!("Steam Guard: please confirm the login in your Steam Mobile app.");
                });

                match depot.steam_id() {
                    Ok(id) => {
                        println!("Resolved Steam ID: {id}");
                        Some(id)
                    }
                    Err(e) => {
                        eprintln!("warning: could not resolve Steam ID: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("warning: steamcmd not available, skipping Steam ID resolution: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut session = Session::load().unwrap_or_default();
    session.steam.api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };
    session.steam.username = if username.is_empty() {
        None
    } else {
        Some(username)
    };
    session.steam.password = if password.is_empty() {
        None
    } else {
        Some(password)
    };
    session.steam.steam_id = steam_id;

    match session.save() {
        Ok(()) => {
            let path = Session::path().map_or_else(
                |_| "~/.gamedepot/session.toml".into(),
                |p| p.display().to_string(),
            );
            println!("\nSaved to {path}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error saving session: {e}");
            ExitCode::FAILURE
        }
    }
}

fn progress_bar() -> indicatif::ProgressBar {
    let bar = indicatif::ProgressBar::new(0);
    bar.set_style(
        indicatif::ProgressStyle::with_template(
            "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        )
        .expect("valid template")
        .progress_chars("━╸━"),
    );
    bar
}

/// Build the default install directory: ~/gamedepot/<depot>/<name>/
/// Falls back to the app ID if the name can't be determined or the
/// home directory is unavailable.
fn default_install_dir(depot_name: &str, game_name: &str) -> PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_or_else(|_| PathBuf::from("."), PathBuf::from);
    base.join("gamedepot").join(depot_name).join(game_name)
}

fn print_app_info(depot: &mut SteamDepot, app_id: &str) {
    if let Ok(info) = depot.app_info(app_id) {
        println!(
            "{} (app {}, build {})",
            info.name.as_deref().unwrap_or("unknown"),
            info.app_id,
            info.build_id.as_deref().unwrap_or("?"),
        );
    }
}

/// Return the current time as an ISO 8601 string (UTC, second precision).
fn now_iso8601() -> String {
    // Use the `date` command to avoid pulling in a datetime crate.
    std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

/// Record (or update) an install in the manifest.
fn save_manifest_entry(
    app_id: &str,
    name: Option<&str>,
    build_id: Option<&str>,
    depot: DepotKind,
    path: &Path,
) {
    let now = now_iso8601();
    let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut manifest = Manifest::load().unwrap_or_default();
    let installed_at = manifest
        .find_by_path(&abs_path)
        .map_or_else(|| now.clone(), |i| i.installed_at.clone());
    manifest.upsert(Install {
        app_id: app_id.to_owned(),
        name: name.map(String::from),
        build_id: build_id.map(String::from),
        depot,
        path: abs_path,
        installed_at,
        updated_at: now,
    });
    if let Err(e) = manifest.save() {
        eprintln!("warning: could not save manifest: {e}");
    }
}

fn cmd_download(depot: &mut SteamDepot, app_id: &str, dir: &Path) -> ExitCode {
    // Pre-create the directory so steamcmd uses our casing instead
    // of lowercasing the folder name.
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("error: could not create install directory: {e}");
        return ExitCode::FAILURE;
    }
    print_app_info(depot, app_id);
    let bar = progress_bar();
    match depot.download_with_progress(app_id, dir, |p| {
        bar.set_length(p.total_bytes);
        bar.set_position(p.current_bytes);
        bar.set_message(p.state.to_string());
    }) {
        Ok(result) => {
            match result {
                UpdateResult::AlreadyUpToDate => {
                    bar.finish_and_clear();
                    println!("Already up to date.");
                }
                UpdateResult::Updated => {
                    bar.finish_with_message("done");
                }
            }
            // Try to get name/build_id for the manifest.
            let (name, build_id) = depot
                .app_info(app_id)
                .map(|i| (i.name, i.build_id))
                .unwrap_or((None, None));
            save_manifest_entry(
                app_id,
                name.as_deref(),
                build_id.as_deref(),
                DepotKind::Steam,
                dir,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            bar.abandon_with_message("failed");
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_validate(depot: &mut SteamDepot, app_id: &str, dir: &Path) -> ExitCode {
    print_app_info(depot, app_id);
    let bar = progress_bar();
    match depot.validate_with_progress(app_id, dir, |p| {
        bar.set_length(p.total_bytes);
        bar.set_position(p.current_bytes);
        bar.set_message(p.state.to_string());
    }) {
        Ok(result) => {
            match result {
                UpdateResult::AlreadyUpToDate => {
                    bar.finish_and_clear();
                    println!("Already up to date.");
                }
                UpdateResult::Updated => {
                    bar.finish_with_message("done");
                }
            }
            let (name, build_id) = depot
                .app_info(app_id)
                .map(|i| (i.name, i.build_id))
                .unwrap_or((None, None));
            save_manifest_entry(
                app_id,
                name.as_deref(),
                build_id.as_deref(),
                DepotKind::Steam,
                dir,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            bar.abandon_with_message("failed");
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_update(depot: &mut SteamDepot, app_id: &str, dir: &Path) -> ExitCode {
    print_app_info(depot, app_id);
    let bar = progress_bar();
    match depot.download_with_progress(app_id, dir, |p| {
        bar.set_length(p.total_bytes);
        bar.set_position(p.current_bytes);
        bar.set_message(p.state.to_string());
    }) {
        Ok(result) => {
            match result {
                UpdateResult::AlreadyUpToDate => {
                    bar.finish_and_clear();
                    println!("Already up to date.");
                }
                UpdateResult::Updated => {
                    bar.finish_with_message("done");
                }
            }
            let (name, build_id) = depot
                .app_info(app_id)
                .map(|i| (i.name, i.build_id))
                .unwrap_or((None, None));
            save_manifest_entry(
                app_id,
                name.as_deref(),
                build_id.as_deref(),
                DepotKind::Steam,
                dir,
            );
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
            println!("App ID:      {}", info.app_id);
            println!("Name:        {}", info.name.as_deref().unwrap_or("unknown"));
            println!(
                "Build ID:    {}",
                info.build_id.as_deref().unwrap_or("unknown")
            );
            println!(
                "Install Dir: {}",
                info.install_dir.as_deref().unwrap_or("unknown")
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_status(app_id: &str, depot: &DepotKind) -> ExitCode {
    let manifest = match Manifest::load() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error loading manifest: {e}");
            return ExitCode::FAILURE;
        }
    };
    let installs: Vec<_> = manifest
        .find_by_id(app_id)
        .into_iter()
        .filter(|i| i.depot == *depot)
        .collect();
    if installs.is_empty() {
        println!("No tracked {depot} installs for {app_id}.");
        return ExitCode::SUCCESS;
    }
    for install in installs {
        println!("App ID:       {}", install.app_id);
        println!(
            "Name:         {}",
            install.name.as_deref().unwrap_or("unknown")
        );
        println!(
            "Build ID:     {}",
            install.build_id.as_deref().unwrap_or("unknown")
        );
        println!("Depot:        {}", install.depot);
        println!("Path:         {}", install.path.display());
        println!("Installed at: {}", install.installed_at);
        println!("Updated at:   {}", install.updated_at);
        println!();
    }
    ExitCode::SUCCESS
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

// ── GOG commands ────────────────────────────────────────────────────

fn cmd_gog_login() -> ExitCode {
    let theme = ColorfulTheme::default();
    let url = GogDepot::login_url();

    println!("Open this link in your browser and log in to GOG:\n");
    println!("  {url}\n");
    println!("After logging in you'll be redirected to a page.");
    println!("Copy the URL from your browser's address bar and paste it here.\n");

    let input: String = Input::with_theme(&theme)
        .with_prompt("URL or code")
        .interact_text()
        .unwrap_or_default();

    let mut gog = GogDepot::new();
    if let Err(e) = gog.login_with_code(&input) {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }

    let token = match gog.refresh_token() {
        Ok(t) => t.to_owned(),
        Err(e) => {
            eprintln!("error retrieving token: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Persist to ~/.gamedepot/session.toml.
    let mut session = Session::load().unwrap_or_default();
    session.gog.refresh_token = Some(token);
    if let Err(e) = session.save() {
        eprintln!("warning: could not save session: {e}");
    }

    match Session::path() {
        Ok(p) => println!("\nLogin successful! Session saved to {}", p.display()),
        Err(_) => println!("\nLogin successful!"),
    }
    ExitCode::SUCCESS
}

fn cmd_gog_search(query: &str) -> ExitCode {
    let gog = GogDepot::new();
    match gog.search(query) {
        Ok(products) => {
            if products.is_empty() {
                println!("No results found.");
            } else {
                for p in &products {
                    let platforms = p.operating_systems.join(", ");
                    println!("{:<12} {} [{}]", p.id, p.title, platforms);
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

fn cmd_gog_info(product_id: &str) -> ExitCode {
    let gog = GogDepot::new();
    match gog.app_info(product_id) {
        Ok(info) => {
            println!("Product ID: {}", info.product_id);
            println!("Name:       {}", info.name.as_deref().unwrap_or("unknown"));
            println!(
                "Build ID:   {}",
                info.build_id.as_deref().unwrap_or("unknown")
            );
            let platforms = format_platforms(info.windows, info.macos, info.linux);
            println!("Platforms:  {platforms}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_gog_owned(search: Option<&str>, page: u32, refresh_token: &str) -> ExitCode {
    let mut gog = GogDepot::new().with_refresh_token(refresh_token);
    match gog.owned_products(search, page) {
        Ok(products) => {
            // Persist the rotated refresh token.
            save_gog_token(&gog);

            if products.is_empty() {
                println!("No owned products found.");
            } else {
                for p in &products {
                    let platforms = p
                        .works_on
                        .as_ref()
                        .map_or_else(String::new, |w| format_platforms(w.windows, w.mac, w.linux));
                    println!("{:<12} {} [{}]", p.id, p.title, platforms);
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

/// Persist the (possibly rotated) GOG refresh token back to the session.
fn save_gog_token(gog: &GogDepot) {
    if let Ok(token) = gog.refresh_token() {
        let mut session = Session::load().unwrap_or_default();
        session.gog.refresh_token = Some(token.to_owned());
        if let Err(e) = session.save() {
            eprintln!("warning: could not save session: {e}");
        }
    }
}

/// Read the GOG refresh token from the session file, falling back to
/// the `GOG_REFRESH_TOKEN` environment variable.
fn read_gog_token() -> Option<String> {
    // Env var takes priority over session file.
    if let Ok(token) = std::env::var("GOG_REFRESH_TOKEN") {
        return Some(token);
    }
    // Fall back to session file.
    Session::load().ok().and_then(|s| s.gog.refresh_token)
}

/// Read the Steam Web API key from `STEAM_API_KEY` env var, falling
/// back to the session file.
fn read_steam_api_key() -> Option<String> {
    if let Ok(key) = std::env::var("STEAM_API_KEY") {
        return Some(key);
    }
    Session::load().ok().and_then(|s| s.steam.api_key)
}

/// Read the saved Steam ID from the session file.
fn read_steam_id() -> Option<String> {
    Session::load().ok().and_then(|s| s.steam.steam_id)
}

fn cmd_steam_owned(api_key: &str, steam_id_or_vanity: &str, search: Option<&str>) -> ExitCode {
    let depot = SteamDepot::api_only().with_api_key(api_key);

    // If the input looks like a 64-bit Steam ID, use it directly.
    // Otherwise try to resolve it as a vanity URL.
    let steam_id = if steam_id_or_vanity.len() == 17
        && steam_id_or_vanity.chars().all(|c| c.is_ascii_digit())
    {
        steam_id_or_vanity.to_owned()
    } else {
        match depot.resolve_vanity_url(steam_id_or_vanity) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("error resolving vanity URL: {e}");
                return ExitCode::FAILURE;
            }
        }
    };

    match depot.owned_games(&steam_id, true) {
        Ok(games) => {
            // Apply optional search filter (case-insensitive).
            let games: Vec<_> = if let Some(q) = search {
                let q = q.to_lowercase();
                games
                    .into_iter()
                    .filter(|g| g.name.as_deref().unwrap_or("").to_lowercase().contains(&q))
                    .collect()
            } else {
                games
            };

            if games.is_empty() {
                println!("No owned games found (profile may be private).");
            } else {
                println!("{} games owned:\n", games.len());
                for g in &games {
                    let name = g.name.as_deref().unwrap_or("?");
                    let hours = g.playtime_forever / 60;
                    let mins = g.playtime_forever % 60;
                    println!("{:<12} {} [{hours}h {mins}m]", g.appid, name);
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

// ── main ────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Steam { command } => run_steam_command(command, cli.install_steamcmd),
        Commands::Gog { command } => run_gog_command(command),
    }
}

fn run_steam_command(command: SteamCommands, install: bool) -> ExitCode {
    match command {
        SteamCommands::Login => cmd_steam_login(),
        SteamCommands::Download {
            app_id,
            dir,
            platform,
        } => match build_depot(install, platform) {
            Ok(mut depot) => {
                let dir = dir.unwrap_or_else(|| {
                    let info = depot.app_info(&app_id).ok();
                    let dir_name = info
                        .as_ref()
                        .and_then(|i| i.install_dir.clone())
                        .or_else(|| info.as_ref().and_then(|i| i.name.clone()))
                        .unwrap_or_else(|| app_id.clone());
                    default_install_dir("steam", &dir_name)
                });
                cmd_download(&mut depot, &app_id, &dir)
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        SteamCommands::Info { id } => match build_depot(install, None) {
            Ok(mut depot) => cmd_info(&mut depot, &id),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        SteamCommands::Status { id } => cmd_status(&id, &DepotKind::Steam),
        SteamCommands::Search { query } => cmd_search(&query),
        SteamCommands::Validate {
            app_id,
            dir,
            platform,
        } => match build_depot(install, platform) {
            Ok(mut depot) => cmd_validate(&mut depot, &app_id, &dir),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        SteamCommands::Update {
            app_id,
            dir,
            platform,
        } => match build_depot(install, platform) {
            Ok(mut depot) => cmd_update(&mut depot, &app_id, &dir),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        SteamCommands::Owned { steam_id, search } => {
            let Some(api_key) = read_steam_api_key() else {
                eprintln!("error: no API key. Run `gamedepot steam login` or set STEAM_API_KEY.");
                return ExitCode::FAILURE;
            };
            let steam_id = steam_id.or_else(read_steam_id);
            let Some(steam_id) = steam_id else {
                eprintln!("error: no Steam ID. Run `gamedepot steam login` or pass a Steam ID.");
                return ExitCode::FAILURE;
            };
            cmd_steam_owned(&api_key, &steam_id, search.as_deref())
        }
        SteamCommands::InstallSteamcmd => cmd_install_steamcmd(),
    }
}

fn run_gog_command(command: GogCommands) -> ExitCode {
    match command {
        GogCommands::Login => cmd_gog_login(),
        GogCommands::Search { query } => cmd_gog_search(&query),
        GogCommands::Info { id } => cmd_gog_info(&id),
        GogCommands::Status { id } => cmd_status(&id, &DepotKind::Gog),
        GogCommands::Owned { search, page } => {
            let Some(token) = read_gog_token() else {
                eprintln!("error: not logged in. Run `gamedepot gog login` first.");
                return ExitCode::FAILURE;
            };
            cmd_gog_owned(search.as_deref(), page, &token)
        }
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
