use std::io;
use std::process::Command;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        copy_with_command("pbcopy", &[], text)?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        copy_with_command("clip", &[], text)?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        copy_with_command("xclip", &["-selection", "clipboard"], text)?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Clipboard not supported on this platform".to_string())
}

pub fn paste_from_clipboard() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let output = run_command("pbpaste", &[])?;
        return Ok(output);
    }

    #[cfg(target_os = "windows")]
    {
        let output = run_command("powershell", &["-NoProfile", "-Command", "Get-Clipboard"])?;
        return Ok(output);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let output = run_command("xclip", &["-selection", "clipboard", "-o"])?;
        return Ok(output);
    }

    #[allow(unreachable_code)]
    Err("Clipboard not supported on this platform".to_string())
}

fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| map_command_error(command, e))?;
    if !output.status.success() {
        return Err(format!("Clipboard command '{command}' failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn copy_with_command(command: &str, args: &[&str], text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = Command::new(command)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| map_command_error(command, e))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    child.wait().map_err(|e| e.to_string()).and_then(|status| {
        if status.success() {
            Ok(())
        } else {
            Err(format!("Clipboard command '{command}' failed"))
        }
    })
}

fn map_command_error(command: &str, error: io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        return format!("Clipboard tool '{command}' not found. Install it or configure your PATH.");
    }
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::map_command_error;
    use std::io;

    #[test]
    fn test_missing_clipboard_tool_message() {
        let err = io::Error::new(io::ErrorKind::NotFound, "missing");
        let message = map_command_error("pbcopy", err);
        assert!(message.contains("pbcopy"));
        assert!(message.contains("not found"));
    }
}
