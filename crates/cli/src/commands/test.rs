use std::io::BufRead;
use std::process::{Command as CargoCmd, Stdio};
use std::time::{Duration, Instant};
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct TestCommand;

impl Command for TestCommand {
    fn signature(&self) -> &str { "test {--filter} {--watch} {--no-run}" }
    fn description(&self) -> &str { "Run tests with industry-standard TUI output" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        let filter = input.option::<String>("filter").and_then(|r| r.ok());
        let watch = input.flag("watch");
        let no_run = input.flag("no-run");

        let mut args = vec!["test".to_string()];

        if no_run {
            args.push("--no-run".to_string());
        }

        if let Some(f) = &filter {
            args.push("--".to_string());
            args.push(f.clone());
        }

        if watch {
            loop {
                run_tests(&current_dir, &args, output);
                output.line("");
                output.info("Watching for changes...");

                // Simple file watch loop
                let watcher_interval = Duration::from_millis(500);
                let mut last_mtime = get_src_mtime(&current_dir);
                loop {
                    std::thread::sleep(watcher_interval);
                    let new_mtime = get_src_mtime(&current_dir);
                    if new_mtime != last_mtime {
                        last_mtime = new_mtime;
                        output.line("");
                        output.info("Files changed, re-running tests...");
                        output.line("");
                        break;
                    }
                }
            }
        }

        run_tests(&current_dir, &args, output)
    }
}

fn get_src_mtime(dir: &std::path::Path) -> u64 {
    let src = dir.join("src");
    let mut latest = 0u64;
    if let Ok(entries) = std::fs::read_dir(&src) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata()
                && let Ok(m) = meta.modified()
                    && let Ok(d) = m.duration_since(std::time::UNIX_EPOCH)
                        && d.as_secs() > latest {
                            latest = d.as_secs();
                        }
        }
    }
    latest
}

fn run_tests(dir: &std::path::Path, args: &[String], _output: &Output) -> ExitCode {
    use viontest::runner::{TestRunner, TestResult, ConsoleTestReporter};

    let reporter = ConsoleTestReporter::new();
    let mut runner = TestRunner::new();
    let start = Instant::now();

    let mut child = match CargoCmd::new("cargo")
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            reporter.print_error(&format!("Failed to run cargo test: {}", e));
            return ExitCode::Failure;
        }
    };

    let stdout = child.stdout.take().expect("no stdout");
    let reader = std::io::BufReader::new(stdout);

    // Regex patterns for parsing cargo test output
    let re_test = regex::Regex::new(
        r"^test\s+(.+?)\s+\.\.\.\s+(\w+)"
    ).unwrap();

    let re_running = regex::Regex::new(r"^running\s+(\d+)").unwrap();

    let mut build_failed = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        // Detect test file/suite from output like "running 3 tests"
        if re_running.captures(&line).is_some() {
            continue;
        }

        // Parse test results
        if let Some(cap) = re_test.captures(&line) {
            let test_name = cap.get(1).unwrap().as_str().to_string();
            let status_str = cap.get(2).unwrap().as_str();

            // Extract suite from test name (e.g. "tests::add" → suite="tests", name="add")
            let (suite, name) = if let Some(idx) = test_name.rfind("::") {
                (test_name[..idx].to_string(), test_name[idx+2..].to_string())
            } else {
                ("test".into(), test_name.clone())
            };

            let elapsed = start.elapsed();

            let result = match status_str {
                "ok" => TestResult::pass(&name, elapsed),
                "FAILED" => TestResult::fail(&name, &format!("Test '{}' failed", test_name), elapsed),
                "ignored" | _ => TestResult::skip(&name),
            };

            runner.add_result(&suite, result);
            continue;
        }

        // Detect build failures / errors
        if line.contains("error[") || line.contains("error:") || line.contains("aborting") {
            build_failed = true;
        }

        // Detect test failure details
        if line.starts_with("----") || line.starts_with("thread '") {
            build_failed = true;
        }
    }

    let _ = child.wait();

    runner.print();

    let summary = runner.summary();

    if build_failed || summary.has_failures() {
        reporter.print_footer(&summary);
        ExitCode::Failure
    } else {
        reporter.print_footer(&summary);
        ExitCode::Success
    }
}
