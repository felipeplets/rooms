use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use portable_pty::{CommandBuilder, PtyPair, PtySize, native_pty_system};
use thiserror::Error;

use super::debug_log;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("failed to open PTY: {0}")]
    PtyOpen(String),

    #[error("failed to spawn shell: {0}")]
    SpawnShell(String),

    #[error("failed to write to PTY: {0}")]
    Write(String),
}

/// A PTY session for a room.
pub struct PtySession {
    pair: PtyPair,
    writer: Box<dyn Write + Send>,
    output_rx: Receiver<Vec<u8>>,
    /// The vt100 parser maintains complete terminal state
    pub parser: vt100::Parser,
    _reader_thread: thread::JoinHandle<()>,
}

impl PtySession {
    /// Create a new PTY session with the given size and working directory.
    pub fn new<P: AsRef<Path>>(cols: u16, rows: u16, cwd: P) -> Result<Self, SessionError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| SessionError::PtyOpen(e.to_string()))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd.as_ref());

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| SessionError::SpawnShell(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| SessionError::PtyOpen(e.to_string()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| SessionError::PtyOpen(e.to_string()))?;

        // Channel for output from reader thread
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

        // Spawn reader thread
        let reader_thread = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        debug_log::log_debug(&format!("SESSION_NEW: cols={} rows={}", cols, rows));

        Ok(Self {
            pair,
            writer,
            output_rx: rx,
            parser: vt100::Parser::new(rows, cols, 1000), // rows, cols, scrollback
            _reader_thread: reader_thread,
        })
    }

    /// Process any pending output from the PTY.
    pub fn process_output(&mut self) {
        while let Ok(data) = self.output_rx.try_recv() {
            debug_log::log_pty_input(&data);
            self.parser.process(&data);
        }
    }

    /// Get the screen from the parser.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Get mutable access to the screen from the parser.
    pub fn screen_mut(&mut self) -> &mut vt100::Screen {
        self.parser.screen_mut()
    }

    /// Write input to the PTY.
    pub fn write(&mut self, data: &[u8]) -> Result<(), SessionError> {
        self.writer
            .write_all(data)
            .map_err(|e| SessionError::Write(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| SessionError::Write(e.to_string()))?;
        Ok(())
    }

    /// Resize the PTY.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let screen = self.parser.screen();
        let old_size = (screen.size().1 as usize, screen.size().0 as usize);
        if old_size.0 != cols as usize || old_size.1 != rows as usize {
            debug_log::log_pty_resize(old_size, (cols, rows));
        }
        let _ = self.pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser.screen_mut().set_size(rows, cols);
    }
}
