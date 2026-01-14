use std::process::ExitCode;

mod git;

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

    // TODO: Launch TUI
    println!("rooms {} - Git worktree manager", env!("CARGO_PKG_VERSION"));
    println!("Repository: {}", repo_root.display());
    println!();
    println!("TUI not yet implemented. Run 'rooms --help' for usage.");

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
