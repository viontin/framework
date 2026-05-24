use std::process::Command as CargoCmd;
use std::sync::mpsc;
use std::time::Duration;
use notify::Watcher;
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct DevCommand;

impl Command for DevCommand {
    fn signature(&self) -> &str { "dev {--port=3000}" }
    fn description(&self) -> &str { "Run the project in development mode with file watching" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let _port: u16 = input.option("port")
            .and_then(|r: Result<String, String>| r.ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000u16);

        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        output.title("Dev");
        output.info("Watching src/ for changes...");
        output.line("");

        // File watcher — only watch src/ to avoid target/ noise
        let (tx, rx) = mpsc::channel::<String>();

        let watch_dir = current_dir.join("src");
        if !watch_dir.is_dir() {
            output.error("src/ directory not found");
            return ExitCode::Failure;
        }

        std::thread::spawn(move || {
            let (watcher_tx, watcher_rx) = mpsc::channel::<notify::DebouncedEvent>();
            let mut watcher = match notify::watcher(watcher_tx, Duration::from_millis(500)) {
                Ok(w) => w,
                Err(_) => return,
            };
            if watcher.watch(&watch_dir, notify::RecursiveMode::Recursive).is_err() {
                return;
            }
            loop {
                match watcher_rx.recv() {
                    Ok(notify::DebouncedEvent::Write(p))
                    | Ok(notify::DebouncedEvent::Create(p))
                    | Ok(notify::DebouncedEvent::Remove(p)) => {
                        if p.extension().is_some_and(|e| e == "rs") {
                            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let _ = tx.send(name);
                        }
                    }
                    Err(_) => break,
                    _ => {}
                }
            }
        });

        loop {
            let mut child = match CargoCmd::new("cargo").arg("run").current_dir(&current_dir).spawn() {
                Ok(c) => c,
                Err(e) => {
                    output.error(&format!("Failed to start cargo: {}", e));
                    return ExitCode::Failure;
                }
            };

            let exit_code = loop {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(file) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        output.line("");
                        output.info(&format!("[change] {} modified, restarting...", file));
                        output.line("");
                        break None;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if let Some(status) = child.try_wait().ok().flatten() {
                            break Some(status.code());
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = child.wait();
                        return ExitCode::Failure;
                    }
                }
            };

            // Drain any stale events from the channel before the next wait
            while rx.try_recv().is_ok() {}

            match exit_code {
                None => continue,
                Some(code) => {
                    if code != Some(0) {
                        output.error(&format!("Process exited with code {:?}", code));
                    }
                    output.info("Watching for changes...");
                    match rx.recv() {
                        Ok(file) => {
                            output.info(&format!("[change] {} modified, restarting...", file));
                            continue;
                        }
                        Err(_) => return ExitCode::Failure,
                    }
                }
            }
        }
    }
}
