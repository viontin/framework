use crossterm::{
    cursor,
    execute,
    style::{self, Stylize},
    terminal::{Clear, ClearType},
};
use std::io::{stdout, Write};

/// Prompt the user for a yes/no confirmation.
pub fn confirm(label: &str) -> ConfirmPrompt {
    ConfirmPrompt {
        label: label.to_string(),
        default: true,
        required: false,
    }
}

pub struct ConfirmPrompt {
    label: String,
    default: bool,
    required: bool,
}

impl ConfirmPrompt {
    pub fn default(mut self, val: bool) -> Self {
        self.default = val;
        self
    }

    pub fn required(mut self, yes: bool) -> Self {
        self.required = yes;
        self
    }

    pub fn prompt(self) -> Result<bool, String> {
        let mut stdout = stdout();

        let hint = if self.default { "(Y/n)" } else { "(y/N)" };
        print!(
            "{} {} {} ",
            style::style("?").cyan().bold(),
            style::style(&self.label).white().bold(),
            style::style(hint).dark_grey(),
        );
        stdout.flush().ok();

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        let input = input.trim().to_lowercase();

        let result = match input.as_str() {
            "y" | "yes" => true,
            "n" | "no" => false,
            "" => self.default,
            _ => {
                if self.required {
                    return Err("Invalid input, expected y/n".to_string());
                }
                self.default
            }
        };

        execute!(stdout, cursor::MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();
        let indicator = if result { "✔" } else { "✘" };
        let indicator_style = if result { style::style(indicator).green() } else { style::style(indicator).red() };
        println!(
            "{} {} {}",
            indicator_style,
            style::style(&self.label).cyan(),
            style::style(if result { "Yes" } else { "No" }).white(),
        );

        Ok(result)
    }
}
