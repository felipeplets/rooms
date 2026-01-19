use std::process::ExitCode;

mod config;
mod git;
mod room;
mod state;
mod terminal;
mod ui;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut skip_post_create = false;
    let mut debug_pty = false;

    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-V" => {
                println!("rooms {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "--no-post-create" => {
                skip_post_create = true;
            }
            "--debug-pty" => {
                debug_pty = true;
            }
            arg => {
                eprintln!("error: unknown argument '{arg}'");
                eprintln!("run 'rooms --help' for usage");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    // Initialize PTY debug logging if requested
    if debug_pty {
        if let Err(e) = terminal::debug_log::init() {
            eprintln!("warning: failed to initialize PTY debug logging: {e}");
        } else {
            eprintln!("PTY debug logging enabled. Log file: ~/.rooms/debug.log");
        }
    }

    // Verify we're in a git repository
    let repo_root = match git::get_repo_root() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!();
            eprintln!("rooms must be run from within a git repository.");
            eprintln!("Navigate to a git repository and try again.");
            return ExitCode::FAILURE;
        }
    };

    // Load configuration
    let config = match config::Config::load_from_repo(&repo_root) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("warning: failed to load config, using defaults: {e}");
            config::Config::default()
        }
    };

    let primary_worktree = match git::get_primary_worktree_path_from(&repo_root) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("error: failed to detect primary worktree: {e}");
            eprintln!();
            eprintln!(
                "rooms must be run from a git repository with a detectable primary worktree."
            );
            eprintln!("Check your git worktree configuration and try again.");
            return ExitCode::FAILURE;
        }
    };
    let rooms_dir = config.rooms_path(&primary_worktree);

    // Launch TUI
    let mut app = ui::App::new(
        repo_root,
        rooms_dir,
        config,
        primary_worktree,
        skip_post_create,
    );

    if let Err(e) = app.run() {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn print_help() {
    println!(
        "rooms {} - Terminal UI for managing Git worktrees

USAGE:
    rooms [OPTIONS]

OPTIONS:
    -h, --help           Print help information
    -V, --version        Print version information
    --no-post-create     Skip post-create commands for this session
    --debug-pty          Enable PTY debug logging to ~/.rooms/debug.log
    --rooms-dir <PATH>   Override default rooms directory

DESCRIPTION:
    rooms provides a keyboard-driven terminal interface for creating and
    managing Git worktrees. Each worktree becomes a \"room\" with its own
    embedded shell.

    Privacy: No telemetry. No network calls. Everything stays local.

For more information, visit: https://github.com/felipeplets/rooms",
        env!("CARGO_PKG_VERSION")
    );
}
