use std::process::ExitCode;

mod config;
mod git;
mod room;
mod state;
mod ui;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-V" => {
                println!("rooms {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            arg => {
                eprintln!("error: unknown argument '{arg}'");
                eprintln!("run 'rooms --help' for usage");
                return ExitCode::FAILURE;
            }
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

    let rooms_dir = config.rooms_path(&repo_root);

    // Load state
    let mut rooms_state = match state::RoomsState::load_from_rooms_dir(&rooms_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("warning: failed to load state, starting fresh: {e}");
            state::RoomsState::default()
        }
    };

    // Discover git worktrees
    let worktrees = match git::list_worktrees_from(&repo_root) {
        Ok(wt) => wt,
        Err(e) => {
            eprintln!("warning: failed to list worktrees: {e}");
            Vec::new()
        }
    };

    // Validate room paths (mark missing as orphaned)
    let orphaned_count = rooms_state.validate_paths();
    if orphaned_count > 0 {
        eprintln!("warning: {orphaned_count} room(s) have missing worktree directories");
    }

    // Launch TUI
    let mut app = ui::App::new(repo_root, rooms_dir, config, rooms_state, worktrees);

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
    -h, --help       Print help information
    -V, --version    Print version information
    --no-post-create Skip post-create commands for this session
    --rooms-dir <PATH>  Override default rooms directory

DESCRIPTION:
    rooms provides a keyboard-driven terminal interface for creating and
    managing Git worktrees. Each worktree becomes a \"room\" with its own
    embedded shell.

    Privacy: No telemetry. No network calls. Everything stays local.

For more information, visit: https://github.com/felipeplets/rooms",
        env!("CARGO_PKG_VERSION")
    );
}
