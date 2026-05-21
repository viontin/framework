use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success,
    Failure,
    InvalidArgs,
    Cancel,
}

impl ExitCode {
    pub fn to_code(self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::Failure => 1,
            ExitCode::InvalidArgs => 2,
            ExitCode::Cancel => 130,
        }
    }

    pub fn exit(self) -> ! {
        process::exit(self.to_code())
    }
}

impl From<i32> for ExitCode {
    fn from(code: i32) -> Self {
        match code {
            0 => ExitCode::Success,
            2 => ExitCode::InvalidArgs,
            130 => ExitCode::Cancel,
            _ => ExitCode::Failure,
        }
    }
}

impl<T, E: Into<Box<dyn std::error::Error>>> From<Result<T, E>> for ExitCode {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => ExitCode::Success,
            Err(_) => ExitCode::Failure,
        }
    }
}
