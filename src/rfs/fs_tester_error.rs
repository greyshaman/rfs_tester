//! FsTesterError is used to detect errors in the construction of Fs Tester.
use std::fmt;

/// Configuration parse error
#[derive(Debug)]
pub enum FsTesterError {
    EmptyConfig,
    BaseDirNotFound,
    ShouldFromDirectory,
    // ParseYaml,
    // Io(io::Error),
    // ParseJson(serde_json::Error),
}

impl fmt::Display for FsTesterError {
    /// fmt implementation for FsTesterError
    /// handle different cases
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FsTesterError::EmptyConfig => write!(f, "The configuration should not be empty."),
            FsTesterError::BaseDirNotFound => write!(f, "Base directory not found!"),
            FsTesterError::ShouldFromDirectory => {
                write!(
                    f,
                    "The configuration should start from the containing directory."
                )
            } // FsTesterError::ParseYaml => write!(f, "Config is not satisfy the yaml format"),
              // FsTesterError::ParseJson(ref e) => e.fmt(f),
              // FsTesterError::Io(ref e) => e.fmt(f),
        }
    }
}

impl std::error::Error for FsTesterError {
    // fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    //   match *self {
    //     FsTesterError::EmptyConfig | FsTesterError::ParseYaml => None,
    //     FsTesterError::ParseJson(ref e) => Some(e),
    //     FsTesterError::Io(ref e) => Some(e),
    //   }
    // }
}

// impl From<serde_json::Error> for FsTesterError {
//   /// from implementation for wrapped Error structs
//   fn from(err: serde_json::Error) -> FsTesterError {
//     use serde_json::error::Category;

//     match err.classify() {
//       Category::Io => FsTesterError::Io(err.into()),
//       Category::Syntax | Category::Data | Category::Eof => FsTesterError::ParseJson(err),
//     }
//   }
// }
