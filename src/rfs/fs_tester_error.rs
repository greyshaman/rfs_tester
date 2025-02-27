//! FsTesterError is used to detect errors in the construction of Fs Tester.
use std::fmt::{Debug, Display};
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::{fmt, result as std_result};

use tokio::sync::AcquireError;
use tokio::task::JoinError;

/// This type represents configuration parse and test directory creation errors
pub struct FsTesterError {
    err: Box<ErrorImpl>,

    /// The path created a sandbox directory. If it was not created, should be None.
    sandbox_dir: Option<String>,
}

/// Customized result type to handle config parse error
pub type Result<T> = std_result::Result<T, FsTesterError>;

macro_rules! fs_tester_error {
    ($code:expr, $line:expr, $column:expr, $sandbox_dir:expr) => {
        FsTesterError {
            err: Box::new(ErrorImpl {
                code: $code,
                line: $line,
                column: $column,
            }),
            sandbox_dir: $sandbox_dir,
        }
    };
    ($code:expr, $line:expr, $column:expr) => {
        fs_tester_error!($code, $line, $column, None)
    };
    ($code:expr) => {
        fs_tester_error!($code, 0, 0, None)
    };
}

impl FsTesterError {
    /// Construct error instance in empty configuration case
    pub fn empty_config() -> Self {
        fs_tester_error!(ErrorCode::EmptyConfig)
    }

    /// Construct error instance in case when configuration does not have root directory
    pub fn should_start_from_directory() -> Self {
        fs_tester_error!(ErrorCode::ShouldStartFromDirectory)
    }

    /// If any non-allowed settings are found in the configuration, an error instance will be created.
    pub fn not_allowed_settings() -> Self {
        fs_tester_error!(ErrorCode::LinksNotAllowed)
    }

    /// An error instance is created when an input/output error occurs.
    pub fn io_error(err: std::io::Error) -> Self {
        fs_tester_error!(ErrorCode::Io(err))
    }

    /// An error instance is created when an walkdir error occurs.
    pub fn walkdir_error(err: walkdir::Error) -> Self {
        fs_tester_error!(ErrorCode::WalkDir(err))
    }

    /// One-based line at which the error was detected.
    pub fn line(&self) -> usize {
        self.err.line
    }

    /// One-based column number at witch the error was detected
    pub fn column(&self) -> usize {
        self.err.column
    }

    /// The sandbox_dir getter
    pub fn sandbox_dir(&self) -> Option<String> {
        self.sandbox_dir.clone()
    }

    /// The sandbox_dir setter
    pub fn set_sandbox_dir(&mut self, sandbox_dir: Option<String>) {
        self.sandbox_dir = sandbox_dir;
    }

    /// Categorizes the cause of error.
    ///
    /// - `Category::ConfigFormat` - expected configuration format is not satisfied
    /// - `Category::NotAllowedSettings` - used not activated configuration features
    /// - `Category::Syntax` - Json or Yaml parsers are encountered error when parsed config
    /// - `Category::Io` - failure to read or write data
    pub fn classify(&self) -> Category {
        match self.err.code {
            ErrorCode::EmptyConfig | ErrorCode::ShouldStartFromDirectory => Category::ConfigFormat,
            ErrorCode::LinksNotAllowed => Category::NotAllowedSettings,
            ErrorCode::JsonSyntax(_) | ErrorCode::YamlSyntax(_) => Category::Syntax,
            ErrorCode::Io(_) | ErrorCode::WalkDir(_) => Category::Io,
            ErrorCode::AcquireError(_) | ErrorCode::JoinError(_) => Category::Multitasking,
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

    pub fn is_multitasking(&self) -> bool {
        self.classify() == Category::Multitasking
    }

    /// Returns true if this error was caused of usage not activated restricted features
    pub fn is_not_allowed_settings(&self) -> bool {
        self.classify() == Category::NotAllowedSettings
    }

    pub fn io_error_kind(&self) -> Option<ErrorKind> {
        if let ErrorCode::Io(io_error) = &self.err.code {
            Some(io_error.kind())
        } else {
            None
        }
    }

    pub fn is_empty_config(&self) -> bool {
        matches!(self.err.code, ErrorCode::EmptyConfig)
    }

    pub fn is_should_start_from_directory(&self) -> bool {
        matches!(self.err.code, ErrorCode::ShouldStartFromDirectory)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Category {
    /// The error was caused by a failure configuration format.
    ConfigFormat,

    /// Not allowed settings
    NotAllowedSettings,

    /// The error was caused when configuration was parsed.
    Syntax,

    /// The error was caused by a failure to read data sources or permission
    /// denied or some IO errors
    Io,

    /// The error was caused by the failure of multitasking.
    Multitasking,
}

#[derive(Debug)]
pub(crate) enum ErrorCode {
    /// Empty configuration is not allowed.
    EmptyConfig,

    /// The configuration should start from the containing directory.
    ShouldStartFromDirectory,

    /// If user not set LINKS_ALLOWED env variable and configuration
    /// has links entries notify this error
    LinksNotAllowed,

    /// Yaml parser encountered error.
    YamlSyntax(serde_yaml::Error),

    /// Json parser encountered error.
    JsonSyntax(serde_json::Error),

    /// Some Walkdir error occurred while walking thru directory entry hierarchy
    WalkDir(walkdir::Error),

    /// Some I/O error occurred while serializing or deserializing.
    Io(std::io::Error),

    /// An error occurred while attempting to acquire a semaphore.
    AcquireError(AcquireError),

    /// An error occurred while trying to work with the joined task handle.
    JoinError(JoinError),
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
            ErrorCode::LinksNotAllowed => {
                write!(
                    f,
                    r#"
                    The use of links has been disabled!
                    !!! Be warned that the contents of linked files may be corrupted !!!
                    If you want to enable the use of links, you can do so at your own risk
                    by setting the LINKS_ALLOWED environment variable to "Y".
                    "#
                )
            }
            ErrorCode::WalkDir(err) => write!(f, "Walkdir error: {}", err),
            ErrorCode::Io(err) => write!(f, "IO error: {}", err),
            ErrorCode::JsonSyntax(err) => write!(f, "JSON syntax error: {}", err),
            ErrorCode::YamlSyntax(err) => write!(f, "YAML syntax error: {}", err),
            ErrorCode::AcquireError(err) => write!(f, "Semaphore err: {}", err),
            ErrorCode::JoinError(err) => write!(f, "Join handle err: {}", err),
        }
    }
}

impl Display for FsTesterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&*self.err, f)?;
        if let Some(sandbox_dir) = &self.sandbox_dir {
            write!(f, " Created dir \"{}\" will be removed.", sandbox_dir,)?;
        }

        Ok(())
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
            "FsTesterError {{{:?}, line: {}, column: {}, message: {} }}",
            self.err.code, self.err.line, self.err.column, self
        )
    }
}

impl std::error::Error for FsTesterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.err.code {
            ErrorCode::Io(err) => Some(err),
            ErrorCode::JsonSyntax(err) => Some(err),
            ErrorCode::YamlSyntax(err) => Some(err),
            ErrorCode::WalkDir(err) => Some(err),
            ErrorCode::AcquireError(err) => Some(err),
            ErrorCode::JoinError(err) => Some(err),
            ErrorCode::EmptyConfig
            | ErrorCode::LinksNotAllowed
            | ErrorCode::ShouldStartFromDirectory => None,
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
                Category::Syntax
                | Category::ConfigFormat
                | Category::NotAllowedSettings
                | Category::Multitasking => IoError::new(ErrorKind::InvalidData, error),
            }
        }
    }
}

impl From<IoError> for FsTesterError {
    fn from(error: IoError) -> Self {
        FsTesterError::io_error(error)
    }
}

impl From<serde_json::Error> for FsTesterError {
    fn from(err: serde_json::Error) -> FsTesterError {
        let line = err.line();
        let column = err.column();
        let code = match err.classify() {
            serde_json::error::Category::Io => ErrorCode::Io(err.into()),
            _ => ErrorCode::JsonSyntax(err),
        };
        fs_tester_error!(code, line, column)
    }
}

impl From<serde_yaml::Error> for FsTesterError {
    fn from(err: serde_yaml::Error) -> Self {
        let location = err.location();
        let line = location.as_ref().map(|loc| loc.line()).unwrap_or(0);
        let column = location.as_ref().map(|loc| loc.column()).unwrap_or(0);
        fs_tester_error!(ErrorCode::YamlSyntax(err), line, column)
    }
}

impl From<walkdir::Error> for FsTesterError {
    fn from(err: walkdir::Error) -> Self {
        fs_tester_error!(ErrorCode::WalkDir(err))
    }
}

impl From<AcquireError> for FsTesterError {
    fn from(err: AcquireError) -> Self {
        fs_tester_error!(ErrorCode::AcquireError(err))
    }
}

impl From<JoinError> for FsTesterError {
    fn from(err: JoinError) -> Self {
        fs_tester_error!(ErrorCode::JoinError(err))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use tokio::sync::Semaphore;
    use walkdir::WalkDir;

    use super::*;

    #[test]
    fn test_empty_config_error() {
        let error = FsTesterError::empty_config();
        assert!(error.is_config_format());
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
    }

    #[test]
    fn test_not_allowed_settings_error() {
        let error = FsTesterError::not_allowed_settings();
        assert!(error.is_not_allowed_settings());
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
    }

    #[test]
    fn test_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error = FsTesterError::io_error(io_error);
        assert!(error.is_io());
        assert_eq!(error.io_error_kind(), Some(std::io::ErrorKind::NotFound));
    }

    #[test]
    fn test_io_error_kind_return_none() {
        let error = FsTesterError::empty_config();
        assert!(error.io_error_kind().is_none());
    }

    #[test]
    fn test_is_empty_config() {
        let error = FsTesterError::empty_config();
        assert!(error.is_empty_config());
    }

    #[test]
    fn test_should_start_from_directory_error() {
        let error = FsTesterError::should_start_from_directory();
        assert!(error.is_config_format());
        assert_eq!(error.line(), 0);
        assert_eq!(error.column(), 0);
    }

    #[test]
    fn test_json_syntax_error() {
        let invalid_json = "{ invalid: json }";
        let json_error = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();

        let error = FsTesterError::from(json_error);

        assert!(error.is_syntax());
    }

    #[test]
    fn test_json_io_error() {
        use std::io;

        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let json_error = serde_json::Error::io(io_error);

        let error = FsTesterError::from(json_error);

        assert!(error.is_io());
    }

    #[test]
    fn test_json_data_error() {
        let json_data = r#"{ "number": "not_a_number" }"#;
        let json_error = serde_json::from_str::<i32>(json_data).unwrap_err();

        let error = FsTesterError::from(json_error);

        assert!(error.is_syntax());
    }

    #[test]
    fn test_yaml_syntax_error() {
        // Attempt to parsing the invalid YAML
        let invalid_yaml = "invalid: yaml: [";
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>(invalid_yaml).unwrap_err();

        // Convert error into FsTesterError
        let error = FsTesterError::from(yaml_error);

        // Verify then error classifying correctly
        assert!(error.is_syntax());
    }

    #[test]
    fn test_display_fmt_for_empty_config() {
        let error = FsTesterError::empty_config();

        assert_eq!(
            format!("{}", error),
            "The configuration should not be empty."
        );
    }

    #[test]
    fn test_display_fmt_for_should_start_from_directory() {
        let error = FsTesterError::should_start_from_directory();

        assert_eq!(
            format!("{}", error),
            "The configuration should start from the containing directory."
        );
    }

    #[test]
    fn test_display_fmt_for_links_not_allowed() {
        let error = FsTesterError::not_allowed_settings();

        assert_eq!(
            format!("{}", error),
            r#"
                    The use of links has been disabled!
                    !!! Be warned that the contents of linked files may be corrupted !!!
                    If you want to enable the use of links, you can do so at your own risk
                    by setting the LINKS_ALLOWED environment variable to "Y".
                    "#
        );
    }

    #[test]
    fn test_display_fmt_for_walkdir_error() {
        let walkdir_error = WalkDir::new("./blahblah")
            .into_iter()
            .next()
            .expect("should return first entry?")
            .expect_err("should be error on non-existent directory");

        let error = FsTesterError::walkdir_error(walkdir_error);

        assert_eq!(format!("{}", error), "Walkdir error: IO error for operation on ./blahblah: No such file or directory (os error 2)");
    }

    #[test]
    fn test_display_fmt_for_io_error() {
        let io_error = std::fs::File::open("blah.blah")
            .expect_err("should return error for attempt to open of non-existent file");
        let error = FsTesterError::io_error(io_error);

        assert_eq!(
            format!("{}", error),
            "IO error: No such file or directory (os error 2)"
        );
    }

    #[test]
    fn test_display_fmt_for_json_syntax_error() {
        let json_data = r#"{ "number": "not_a_number" }"#;
        let json_error = serde_json::from_str::<i32>(json_data).unwrap_err();

        let error = FsTesterError::from(json_error);

        assert_eq!(format!("{}", error), "JSON syntax error: invalid type: map, expected i32 at line 1 column 0 at line 1 column 0")
    }

    #[test]
    fn test_display_fmt_for_yaml_syntax_error() {
        let invalid_yaml = "invalid: yaml: [";
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>(invalid_yaml).unwrap_err();

        let error = FsTesterError::from(yaml_error);

        assert_eq!(format!("{}", error), "YAML syntax error: mapping values are not allowed in this context at line 1 column 14 at line 1 column 14");
    }

    #[test]
    fn test_display_fmt_with_sandbox_dir() {
        let mut error = FsTesterError::empty_config();
        error.set_sandbox_dir(Some("sandbox_dir".to_string()));

        assert!(format!("{}", error).contains("Created dir \"sandbox_dir\" will be removed."));
    }

    #[test]
    fn test_debug_fmt_implementation() {
        let invalid_yaml = "invalid: yaml: [";
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>(invalid_yaml).unwrap_err();

        let error = FsTesterError::from(yaml_error);
        assert_eq!(
            format!("{:?}", error),
            "FsTesterError {YamlSyntax(Error { kind: SCANNER, problem: \"mapping values are not allowed in this context\", problem_mark: Mark { line: 1, column: 14 } }), line: 1, column: 14, message: YAML syntax error: mapping values are not allowed in this context at line 1 column 14 at line 1 column 14 }",
        );
    }

    #[test]
    fn test_error_source_for_io_error() {
        let io_error = std::fs::File::open("blah.blah")
            .expect_err("should return error for attempt to open of non-existent file");
        let error = FsTesterError::io_error(io_error);

        assert_eq!(
            format!("{:?}", error.source()),
            "Some(Os { code: 2, kind: NotFound, message: \"No such file or directory\" })"
        );
    }

    #[test]
    fn test_error_source_for_json_syntax_error() {
        let json_data = r#"{ "number": "not_a_number" }"#;
        let json_error = serde_json::from_str::<i32>(json_data).unwrap_err();

        let error = FsTesterError::from(json_error);

        assert_eq!(
            format!("{:?}", error.source()),
            "Some(Error(\"invalid type: map, expected i32\", line: 1, column: 0))",
        );
    }

    #[test]
    fn test_error_source_for_yaml_syntax_error() {
        let invalid_yaml = "invalid: yaml: [";
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>(invalid_yaml).unwrap_err();

        let error = FsTesterError::from(yaml_error);

        assert_eq!(
            format!("{:?}", error.source()),
            "Some(Error { kind: SCANNER, problem: \"mapping values are not allowed in this context\", problem_mark: Mark { line: 1, column: 14 } })",
        );
    }

    #[test]
    fn test_error_source_for_walkdir_error() {
        let walkdir_error = WalkDir::new("./blahblah")
            .into_iter()
            .next()
            .expect("should return first entry?")
            .expect_err("should be error on non-existent directory");

        let error = FsTesterError::walkdir_error(walkdir_error);

        assert_eq!(
            format!("{:?}", error.source()),
            "Some(Error { depth: 0, inner: Io { path: Some(\"./blahblah\"), err: Os { code: 2, kind: NotFound, message: \"No such file or directory\" } } })",
        );
    }

    #[test]
    fn test_error_source_for_empty_config_error() {
        let error = FsTesterError::empty_config();

        assert_eq!(format!("{:?}", error.source()), "None");
    }

    #[test]
    fn test_error_source_for_links_not_allowed_error() {
        let error = FsTesterError::not_allowed_settings();

        assert_eq!(format!("{:?}", error.source()), "None");
    }

    #[test]
    fn test_error_source_for_should_start_from_directory_error() {
        let error = FsTesterError::should_start_from_directory();

        assert_eq!(format!("{:?}", error.source()), "None");
    }

    #[test]
    fn test_error_from_for_walkdir_error() {
        let walkdir_error = WalkDir::new("./blahblah")
            .into_iter()
            .next()
            .expect("should return first entry?")
            .expect_err("should be error on non-existent directory");

        let error = FsTesterError::from(walkdir_error);

        assert!(error.is_io());
    }

    #[test]
    fn test_from_fs_tester_error_for_io_error() {
        let io_error = std::fs::File::open("blah.blah").expect_err("should be error");
        let error = FsTesterError::io_error(io_error);
        let converted_io_error = std::io::Error::from(error);

        assert!(converted_io_error.kind() == ErrorKind::NotFound);
    }

    #[test]
    fn test_from_fs_tester_error_for_io_error_invalid_data() {
        let error = FsTesterError::empty_config();
        let converted_io_error = std::io::Error::from(error);

        assert!(converted_io_error.kind() == ErrorKind::InvalidData);
    }

    #[test]
    fn test_from_io_error_into_fs_tester_error() {
        let io_error = std::fs::File::open("blah.blah").expect_err("should be error");
        let error = FsTesterError::from(io_error);

        assert!(error.is_io());
    }

    #[tokio::test]
    async fn test_display_fmt_for_acquire_error() {
        let semaphore = Semaphore::const_new(2);
        semaphore.close();
        let acquire_error = semaphore
            .acquire()
            .await
            .expect_err("closed semaphore should return return error");
        let error = FsTesterError::from(acquire_error);

        assert_eq!(format!("{}", error), "Semaphore err: semaphore closed");
    }

    #[tokio::test]
    async fn test_display_fmt_for_join_error() {
        let handle = tokio::spawn(async move {
            panic!("test JoinError");
        });

        let join_error = handle
            .await
            .expect_err("panic in handle should produce JoinError");
        let error = FsTesterError::from(join_error);

        assert!(format!("{}", error).contains("Join handle err: task"));
        assert!(format!("{}", error).contains("panicked with message \"test JoinError\""));
    }

    #[tokio::test]
    async fn test_acquire_error() {
        let semaphore = Semaphore::new(2);
        semaphore.close();
        let acquire_error = semaphore
            .acquire()
            .await
            .expect_err("closed semaphore should return error");
        let error = FsTesterError::from(acquire_error);

        assert!(error.is_multitasking());
    }

    #[tokio::test]
    async fn test_join_error() {
        let handle = tokio::spawn(async move {
            panic!("test JoinError");
        });

        let join_error = handle
            .await
            .expect_err("panic in handle should produce JoinError");
        let error = FsTesterError::from(join_error);

        assert!(error.is_multitasking());
    }

    #[tokio::test]
    async fn test_error_source_for_acquire_error() {
        let semaphore = Semaphore::new(2);
        semaphore.close();
        let acquire_error = semaphore
            .acquire()
            .await
            .expect_err("closed semaphore should return error");
        let error = FsTesterError::from(acquire_error);

        assert_eq!(format!("{:?}", error.source()), "Some(AcquireError(()))");
    }

    #[tokio::test]
    async fn test_error_source_for_join_error() {
        let handle = tokio::spawn(async move {
            panic!("test JoinError");
        });

        let join_error = handle
            .await
            .expect_err("panic in handle should produce JoinError");
        let error = FsTesterError::from(join_error);

        assert!(format!("{:?}", error.source()).contains("JoinError::Panic"));
    }
}
