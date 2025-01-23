//! FsTesterError is used to detect errors in the construction of Fs Tester.
use std::fmt::{Debug, Display};
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::{fmt, result as std_result};

/// This type represents configuration parse and test directory creation errors
pub struct FsTesterError {
    err: Box<ErrorImpl>,
}

/// Customized result type to handle config parse error
pub type Result<T> = std_result::Result<T, FsTesterError>;

impl FsTesterError {
    /// Construct error instance in empty configuration case
    pub fn empty_config() -> Self {
        FsTesterError {
            err: Box::new(ErrorImpl {
                code: ErrorCode::EmptyConfig,
                line: 0,
                column: 0,
            }),
        }
    }

    /// Construct error instance in case when configuration does not have root directory
    pub fn should_start_from_directory() -> Self {
        FsTesterError {
            err: Box::new(ErrorImpl {
                code: ErrorCode::ShouldStartFromDirectory,
                line: 0,
                column: 0,
            }),
        }
    }

    pub fn io_error(err: std::io::Error) -> Self {
        FsTesterError {
            err: Box::new(ErrorImpl {
                code: ErrorCode::Io(err),
                line: 0,
                column: 0,
            }),
        }
    }

    /// One-based line at which the error was detected.
    pub fn line(&self) -> usize {
        self.err.line
    }

    /// One-based column number at witch the error was detected
    pub fn column(&self) -> usize {
        self.err.column
    }

    /// Categorizes the cause of error.
    ///
    /// - `Category::ConfigFormat` - expected configuration format is not satisfied
    /// - `Category::Syntax` - Json or Yaml parsers are encountered error when parsed config
    /// - `Category::Io` - failure to read or write data
    pub fn classify(&self) -> Category {
        match self.err.code {
            ErrorCode::EmptyConfig | ErrorCode::ShouldStartFromDirectory => Category::ConfigFormat,
            ErrorCode::JsonSyntax(_) | ErrorCode::YamlSyntax(_) => Category::Syntax,
            ErrorCode::Io(_) => Category::Io,
        }
    }

    /// Returns true if this error was caused by io error
    pub fn is_io(&self) -> bool {
        self.classify() == Category::Io
    }

    /// Returns true if this error was caused in configuration parsing
    pub fn is_syntax(&self) -> bool {
        self.classify() == Category::Syntax
    }

    /// Returns true if this error was caused in analyzing config format
    pub fn is_config_format(&self) -> bool {
        self.classify() == Category::ConfigFormat
    }

    pub fn io_error_kind(&self) -> Option<ErrorKind> {
        if let ErrorCode::Io(io_error) = &self.err.code {
            Some(io_error.kind())
        } else {
            None
        }
    }

    pub fn is_empty_config(&self) -> bool {
        match self.err.code {
            ErrorCode::EmptyConfig => true,
            _ => false,
        }
    }

    pub fn is_should_start_from_directory(&self) -> bool {
        match self.err.code {
            ErrorCode::ShouldStartFromDirectory => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Category {
    /// The error was caused by a failure configuration format.
    ConfigFormat,

    /// The error was caused when configuration was parsed.
    Syntax,

    /// The error was caused by a failure to read data sources or permission
    /// denied or some IO errors
    Io,
}

#[derive(Debug)]
pub(crate) enum ErrorCode {
    /// Empty configuration is not allowed.
    EmptyConfig,

    /// The configuration should start from the containing directory.
    ShouldStartFromDirectory,

    /// Yaml parser encountered error.
    YamlSyntax(serde_yaml::Error),

    /// Json parser encountered error.
    JsonSyntax(serde_json::Error),

    /// Some I/O error occurred while serializing or deserializing.
    Io(std::io::Error),
}

#[derive(Debug)]
struct ErrorImpl {
    code: ErrorCode,
    line: usize,
    column: usize,
}

impl Display for ErrorCode {
    /// fmt implementation for FsTesterError
    /// handle different cases
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorCode::EmptyConfig => write!(f, "The configuration should not be empty."),
            ErrorCode::ShouldStartFromDirectory => {
                write!(
                    f,
                    "The configuration should start from the containing directory."
                )
            }
            ErrorCode::Io(err) => write!(f, "IO error: {}", err),
            ErrorCode::JsonSyntax(err) => write!(f, "JSON syntax error: {}", err),
            ErrorCode::YamlSyntax(err) => write!(f, "YAML syntax error: {}", err),
        }
    }
}

impl Display for FsTesterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&*self.err, f)
    }
}

impl Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            Display::fmt(&self.code, f)
        } else {
            write!(
                f,
                "{} at line {} column {}",
                self.code, self.line, self.column
            )
        }
    }
}

impl Debug for FsTesterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error({:?}, line: {}, column: {}",
            self.err.code, self.err.line, self.err.column,
        )
    }
}

impl std::error::Error for FsTesterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.err.code {
            ErrorCode::Io(err) => Some(err),
            ErrorCode::JsonSyntax(err) => Some(err),
            ErrorCode::YamlSyntax(err) => Some(err),
            _ => None,
        }
    }
}

impl From<FsTesterError> for IoError {
    fn from(error: FsTesterError) -> Self {
        if let ErrorCode::Io(err) = error.err.code {
            err
        } else {
            match error.classify() {
                Category::Io => unreachable!(),
                Category::Syntax | Category::ConfigFormat => {
                    IoError::new(ErrorKind::InvalidData, error)
                }
            }
        }
    }
}

impl From<serde_json::Error> for FsTesterError {
    /// from implementation for wrapped Error structs
    fn from(err: serde_json::Error) -> FsTesterError {
        use serde_json::error::Category as JsonCategory;

        let line = err.line();
        let column = err.column();
        match err.classify() {
            JsonCategory::Io => FsTesterError {
                err: Box::new(ErrorImpl {
                    code: ErrorCode::Io(err.into()),
                    line,
                    column,
                }),
            },
            JsonCategory::Syntax | JsonCategory::Data | JsonCategory::Eof => FsTesterError {
                err: Box::new(ErrorImpl {
                    code: ErrorCode::JsonSyntax(err.into()),
                    line,
                    column,
                }),
            },
        }
    }
}

impl From<serde_yaml::Error> for FsTesterError {
    fn from(err: serde_yaml::Error) -> Self {
        let line = err.location().map(|loc| loc.line()).unwrap_or(0);
        let column = err.location().map(|loc| loc.column()).unwrap_or(0);

        FsTesterError {
            err: Box::new(ErrorImpl {
                code: ErrorCode::YamlSyntax(err),
                line,
                column,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_empty_config_error() {
        let error = FsTesterError::empty_config();
        assert!(error.is_config_format());
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
    }

    #[test]
    fn test_io_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = FsTesterError::io_error(io_error);
        assert!(error.is_io());
        assert_eq!(error.io_error_kind(), Some(io::ErrorKind::NotFound));
    }
}
