use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::{self, Stylize},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};

/// Prompt for a password (hidden input).
pub fn password(label: &str) -> PasswordPrompt {
    PasswordPrompt {
        label: label.to_string(),
        placeholder: String::new(),
        required: false,
        validate: None,
    }
}

pub struct PasswordPrompt {
    label: String,
    placeholder: String,
    required: bool,
    validate: Option<Box<dyn Fn(&str) -> Result<(), String>>>,
}

impl PasswordPrompt {
    pub fn placeholder(mut self, s: &str) -> Self {
        self.placeholder = s.to_string();
        self
    }

    pub fn required(mut self, yes: bool) -> Self {
        self.required = yes;
        self
    }

    pub fn validate<F: Fn(&str) -> Result<(), String> + 'static>(mut self, f: F) -> Self {
        self.validate = Some(Box::new(f));
        self
    }

    pub fn prompt(self) -> Result<String, String> {
        let mut stdout = stdout();
        let mut input = String::new();

        terminal::enable_raw_mode().map_err(|e| e.to_string())?;

        write!(stdout, "{} ", style::style(&self.label).cyan().bold()).ok();
        stdout.flush().ok();

        loop {
            execute!(stdout, cursor::Show).ok();
            stdout.flush().ok();

            match event::read().map_err(|e| e.to_string())? {
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if self.required && input.is_empty() {
                        execute!(
                            stdout,
                            cursor::MoveToColumn(0),
                            Clear(ClearType::CurrentLine),
                        )
                        .ok();
                        write!(
                            stdout,
                            "{} {}",
                            style::style("✘").red(),
                            style::style("This field is required").red().italic(),
                        )
                        .ok();
                        stdout.flush().ok();
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();
                        write!(stdout, "{} ", style::style(&self.label).cyan().bold()).ok();
                        stdout.flush().ok();
                        continue;
                    }

                    if let Some(ref validate) = self.validate {
                        if let Err(e) = validate(&input) {
                            execute!(
                                stdout,
                                cursor::MoveToColumn(0),
                                Clear(ClearType::CurrentLine),
                            )
                            .ok();
                            write!(
                                stdout,
                                "{} {}",
                                style::style("✘").red(),
                                style::style(&e).red().italic(),
                            )
                            .ok();
                            stdout.flush().ok();
                            std::thread::sleep(std::time::Duration::from_millis(1200));
                            execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();
                            write!(stdout, "{} ", style::style(&self.label).cyan().bold()).ok();
                            stdout.flush().ok();
                            continue;
                        }
                    }

                    terminal::disable_raw_mode().map_err(|e| e.to_string())?;
                    execute!(
                        stdout,
                        cursor::MoveToColumn(0),
                        Clear(ClearType::CurrentLine),
                    )
                    .ok();
                    println!(
                        "{} {} {}",
                        style::style("✔").green(),
                        style::style(&self.label).cyan(),
                        style::style("********").dark_grey(),
                    );
                    return Ok(input);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    input.push(c);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    input.pop();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    terminal::disable_raw_mode().map_err(|e| e.to_string())?;
                    execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();
                    println!("{} {}", style::style("✘").red(), style::style(&self.label).red());
                    return Err("Cancelled".to_string());
                }
                _ => {}
            }

            // Show asterisks for input
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine),
            )
            .ok();

            let masked = "*".repeat(input.len());
            let display = if input.is_empty() && !self.placeholder.is_empty() {
                style::style(&self.placeholder).dark_grey().italic().to_string()
            } else {
                masked
            };

            write!(stdout, "{} {}", style::style(&self.label).cyan().bold(), display).ok();
            stdout.flush().ok();
        }
    }
}
