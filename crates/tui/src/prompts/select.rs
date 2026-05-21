use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::{self, Stylize},
    terminal::{self, Clear, ClearType},
};
use std::io::{stdout, Write};

/// Prompt the user to select one option from a list.
pub fn select(label: &str, options: Vec<&str>) -> SelectPrompt {
    SelectPrompt {
        label: label.to_string(),
        options: options.iter().map(|s| s.to_string()).collect(),
        default: None,
        required: false,
    }
}

pub struct SelectPrompt {
    label: String,
    options: Vec<String>,
    default: Option<usize>,
    required: bool,
}

impl SelectPrompt {
    pub fn default(mut self, s: &str) -> Self {
        if let Some(pos) = self.options.iter().position(|o| o == s) {
            self.default = Some(pos);
        }
        self
    }

    pub fn required(mut self, yes: bool) -> Self {
        self.required = yes;
        self
    }

    pub fn prompt(self) -> Result<String, String> {
        let mut stdout = stdout();
        let mut selected = self.default.unwrap_or(0);

        terminal::enable_raw_mode().map_err(|e| e.to_string())?;

        // Render label and options
        write!(stdout, "{} \n", style::style(&self.label).cyan().bold()).ok();

        loop {
            // Re-render options
            for (i, option) in self.options.iter().enumerate() {
                execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();

                if i == selected {
                    write!(
                        stdout,
                        " {} {}",
                        style::style("❯").cyan(),
                        style::style(option).cyan().bold(),
                    )
                    .ok();
                } else {
                    write!(stdout, "   {}", style::style(option).white()).ok();
                }

                // Move cursor down
                if i < self.options.len() - 1 {
                    write!(stdout, "\n").ok();
                }
            }

            // Move cursor back to start of list
            stdout.flush().ok();

            match event::read().map_err(|e| e.to_string())? {
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if selected + 1 < self.options.len() {
                        selected += 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    terminal::disable_raw_mode().map_err(|e| e.to_string())?;

                    if self.required && self.options.is_empty() {
                        return Err("No options available".to_string());
                    }

                    // Clear rendered options
                    for _ in 0..self.options.len() {
                        execute!(stdout, cursor::MoveToPreviousLine(1), Clear(ClearType::CurrentLine)).ok();
                    }
                    execute!(stdout, Clear(ClearType::CurrentLine)).ok();

                    let value = &self.options[selected];
                    println!(
                        "{} {} {}",
                        style::style("✔").green(),
                        style::style(&self.label).cyan(),
                        style::style(value).white(),
                    );
                    return Ok(value.clone());
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    terminal::disable_raw_mode().map_err(|e| e.to_string())?;
                    for _ in 0..self.options.len() {
                        execute!(stdout, cursor::MoveToPreviousLine(1), Clear(ClearType::CurrentLine)).ok();
                    }
                    execute!(stdout, Clear(ClearType::CurrentLine)).ok();
                    println!("{} {}", style::style("✘").red(), style::style(&self.label).red());
                    return Err("Cancelled".to_string());
                }
                _ => {}
            }

            // Move cursor back up to re-render
            if self.options.len() > 1 {
                execute!(
                    stdout,
                    cursor::MoveToPreviousLine(self.options.len() as u16 - 1),
                )
                .ok();
            }
        }
    }
}
