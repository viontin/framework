use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    style::{self, Stylize},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};

/// Prompt for text input with optional placeholder, default value,
/// required validation, and custom validation.
pub fn text(label: &str) -> TextPrompt {
    TextPrompt {
        label: label.to_string(),
        placeholder: String::new(),
        default: String::new(),
        required: false,
        validate: None,
    }
}

pub struct TextPrompt {
    label: String,
    placeholder: String,
    default: String,
    required: bool,
    validate: Option<Box<dyn Fn(&str) -> Result<(), String>>>,
}

impl TextPrompt {
    pub fn placeholder(mut self, s: &str) -> Self {
        self.placeholder = s.to_string();
        self
    }

    pub fn default(mut self, s: &str) -> Self {
        self.default = s.to_string();
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

        // Render label
        write!(stdout, "{} ", style::style(&self.label).cyan().bold()).ok();
        stdout.flush().ok();

        loop {
            queue!(stdout, cursor::Show).ok();
            stdout.flush().ok();

            match event::read().map_err(|e| e.to_string())? {
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    let value = if input.is_empty() {
                        self.default.clone()
                    } else {
                        input.clone()
                    };

                    if self.required && value.is_empty() {
                        // Show error and retry
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
                        // Re-render
                        execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();
                        write!(stdout, "{} ", style::style(&self.label).cyan().bold()).ok();
                        stdout.flush().ok();
                        continue;
                    }

                    if let Some(ref validate) = self.validate {
                        if let Err(e) = validate(&value) {
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
                            // Re-render
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
                        style::style(&value).white(),
                    );
                    return Ok(value);
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

            // Render current input
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine),
            )
            .ok();

            let display = if input.is_empty() && !self.default.is_empty() {
                format!("{} ({})", input, style::style(&self.default).dark_grey().italic())
            } else if input.is_empty() && !self.placeholder.is_empty() {
                style::style(&self.placeholder).dark_grey().italic().to_string()
            } else {
                input.clone()
            };

            write!(stdout, "{} {}", style::style(&self.label).cyan().bold(), display).ok();
            stdout.flush().ok();
        }
    }
}
