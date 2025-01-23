use rand::Rng;
use std::fs::{self, hard_link, DirBuilder, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::result as std_result;

use crate::rfs::fs_tester_error::{FsTesterError, Result};

use super::config::config_entry::ConfigEntry;
use super::config::configuration::Configuration;
use super::config::directory_conf::DirectoryConf;
use super::file_content::FileContent;

/// File System Tester is used to create a configured structure in a directory
/// with files and links to them. It can start a custom test process
/// and remove the file system structure after the testing is complete or fails.
///
/// # Example of use in tests
///
/// ```rust
/// use rfs_tester::{FsTester, FileContent, FsTesterError};
/// use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};
///
/// #[test]
/// fn test_file_creation() -> Result<(), FsTesterError> {
///     let config_str = r#"---
///     - !directory
///         name: test
///         content:
///           - !file
///               name: test.txt
///               content:
///                 !inline_text "Hello, world!"
///     "#;
///
///     let tester = FsTester::new(config_str, ".")?;
///
///     tester.perform_fs_test(|dirname| {
///         let file_path = format!("{}/test.txt", dirname);
///         let content = std::fs::read_to_string(file_path)?;
///         assert_eq!(content, "Hello, world!");
///         Ok(())
///     });
///     Ok(())
/// }
/// ```
pub struct FsTester {
    pub config: Configuration,
    pub base_dir: String,
}

impl FsTester {
    fn get_random_code() -> u64 {
        rand::thread_rng().gen::<u64>()
    }

    fn create_dir(dirname: &str) -> std_result::Result<(), io::Error> {
        let dir_builder = DirBuilder::new();
        dir_builder.create(dirname)?;

        Ok(())
    }

    fn create_file(file_name: &str, content: &[u8]) -> std_result::Result<String, io::Error> {
        let mut file = File::create(&file_name)?;
        file.write_all(content)?;

        Ok(String::from(file_name))
    }

    /// WARNING!!! Use links with caution, as making changes to the content using a link may modify the original file.
    /// TODO: Limit the use of links and only allow them if the intent is explicitly specified via an environment variable.
    fn create_link(link_name: &str, target_name: &str) -> std_result::Result<String, io::Error> {
        hard_link(target_name, link_name)?;

        Ok(String::from(link_name))
    }

    fn delete_test_set(dirname: &str) -> std_result::Result<(), io::Error> {
        fs::remove_dir_all(dirname)?;
        Ok(())
    }

    fn build_directory(
        directory_conf: &DirectoryConf,
        parent_path: &str,
        level: i32,
    ) -> std_result::Result<String, io::Error> {
        let dir_path = if level == 0 {
            let uniq_code = Self::get_random_code();
            format!("{}/{}_{}", parent_path, directory_conf.name, uniq_code)
        } else {
            format!("{}/{}", parent_path, directory_conf.name)
        };
        Self::create_dir(&dir_path)?;

        for entry in directory_conf.content.iter() {
            let mut buffer: Vec<u8> = Vec::new(); // placed here to satisfy lifetime
                                                  // requirement probably better way existing
            let result = match entry {
                ConfigEntry::Directory(conf) => Self::build_directory(conf, &dir_path, level + 1),
                ConfigEntry::File(conf) => {
                    let file_name: String = format!("{}/{}", &dir_path, conf.name);
                    let content: &[u8] = match &conf.content {
                        FileContent::InlineBytes(data) => data,
                        FileContent::InlineText(text) => text.as_bytes(),
                        FileContent::OriginalFile(file_path) => {
                            let mut original_file = File::open(file_path)?;
                            original_file.read_to_end(&mut buffer)?;
                            &buffer
                        }
                        FileContent::Empty => &[],
                    };
                    Self::create_file(&file_name, &content)
                }
                ConfigEntry::Link(conf) => {
                    let link_name = format!("{}/{}", &dir_path, conf.name);
                    Self::create_link(&link_name, &conf.target)
                }
            };
            if let Err(e) = result {
                panic!("{}", e);
            }
        }

        Ok(dir_path)
    }

    /// The configuration parser
    /// The configuration can be in the form of a string in YAML or JSON format:
    ///
    /// # YAML Example
    ///
    /// ```rust
    /// # use rfs_tester::{FsTester, FsTesterError, FileContent};
    /// # use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf, LinkConf};
    /// let simple_conf_str = "---
    ///   - !directory
    ///       name: test
    ///       content:
    ///         - !file
    ///             name: test.txt
    ///             content:
    ///               !inline_bytes
    ///                 - 116
    ///                 - 101
    ///                 - 115
    ///                 - 116
    /// ";
    /// let test_conf = Configuration(vec!(ConfigEntry::Directory(
    /// #   DirectoryConf {
    /// #     name: String::from("test"),
    /// #     content: vec!(
    /// #       ConfigEntry::File(
    /// #         FileConf {
    /// #           name: String::from("test.txt"),
    /// #           content:
    /// #             FileContent::InlineBytes(
    /// #               String::from("test").into_bytes(),
    /// #             )
    /// #         }
    /// #       )
    /// #     ),
    /// #   }
    /// # )));
    /// # assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    /// ```
    ///
    /// ## JSON Example
    ///
    /// ```rust
    /// use rfs_tester::{FsTester, FsTesterError, FileContent};
    /// use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf, LinkConf};
    /// let simple_conf_str =
    ///   "[{\"directory\":{\"name\":\"test\",\"content\":[{\"file\":{\"name\":\"test.txt\",\"content\":{\"inline_bytes\":[116,101,115,116]}}}]}}]";
    /// # let test_conf = Configuration(vec!(ConfigEntry::Directory(
    /// #   DirectoryConf {
    /// #     name: String::from("test"),
    /// #     content: vec!(
    /// #       ConfigEntry::File(
    /// #         FileConf {
    /// #           name: String::from("test.txt"),
    /// #           content:
    /// #             FileContent::InlineBytes(
    /// #               String::from("test").into_bytes(),
    /// #             )
    /// #         }
    /// #       )
    /// #     ),
    /// #   }
    /// # )));
    /// #
    /// # assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    ///
    /// ```
    pub fn parse_config(config_str: &str) -> Result<Configuration> {
        // detect format parse and return config instance
        match config_str.chars().next() {
            Some('{') | Some('[') => {
                serde_json::from_str(config_str).or_else(|error| Err(error.into()))
            }
            Some(_) => serde_yaml::from_str(config_str).or_else(|error| Err(error.into())),
            None => Err(FsTesterError::empty_config()),
        }
    }

    /// Creates a test directory, files, and links.
    /// config_str - The configuration of the test directory is provided in the string in YAML or JSON format
    /// start_point - The directory name where the testing directory will be created should be specified.
    ///               It should be present in the file system.
    pub fn new(config_str: &str, start_point: &str) -> Result<FsTester> {
        let config: Configuration = Self::parse_config(config_str)?;

        let base_dir = if start_point.len() == 0 {
            String::from(".")
        } else {
            if Path::new(start_point).is_dir() {
                String::from(start_point)
            } else {
                return Err(FsTesterError::should_start_from_directory());
            }
        };

        // Checks if the configuration starts from a single directory.
        let zero_level_config_ref: Option<&ConfigEntry> = config.0.iter().next();
        let directory_conf = match zero_level_config_ref {
            Some(entry) => match entry {
                ConfigEntry::File(_) | ConfigEntry::Link(_) => {
                    return Err(FsTesterError::should_start_from_directory());
                }
                ConfigEntry::Directory(conf) => conf,
            },
            None => return Err(FsTesterError::empty_config()),
        };

        let base_dir = Self::build_directory(&directory_conf, &base_dir, 0).unwrap();

        Ok(FsTester { config, base_dir })
    }

    /// The test_proc function starts. The test unit is defined as a closure parameter
    /// of the perform_fs_test function. The dirname closure parameter represents
    /// the name of the temporary test directory that is generated and contains the fs unit set.
    /// We don't know the full name until the testing starts, because it has a random number at the end.
    /// FsTester will know this after the instance has been built.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::fs;
    /// # use rfs_tester::FsTester;
    /// const YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML: &str = "---
    /// - !directory
    ///     name: test
    ///     content:
    ///       - !file
    ///           name: test_from_cargo.toml
    ///           content:
    ///             !original_file Cargo.toml
    ///  ";
    ///
    /// let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".").expect("Incorrect config");
    /// tester.perform_fs_test(|dirname| {
    /// //                      ^^^^^^^ name with appended random at the end of name
    ///   let inner_file_name = format!("{}/{}", dirname, "test_from_cargo.toml");
    ///   let metadata = fs::metadata(inner_file_name)?;
    ///
    ///   assert!(metadata.len() > 0);
    ///   Ok(())
    /// });
    ///
    /// ```
    pub fn perform_fs_test<F>(&self, test_proc: F)
    where
        F: Fn(&str) -> io::Result<()>,
    {
        let dirname: &str = &self.base_dir;

        if let Err(e) = test_proc(dirname) {
            panic!("inner test has error: {}", e)
        } else {
            ()
        }
    }
}

impl Drop for FsTester {
    fn drop(&mut self) {
        if let Err(_) = Self::delete_test_set(&self.base_dir) {
            // TODO: handle delete directory but cannot figure out how and what to do right now. Sorry.
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rfs::config::{file_conf::FileConf, link_conf::LinkConf};
    use crate::rfs::fs_tester_error::Result;

    use super::*;

    const YAML_DIR_WITH_EMPTY_FILE: &str = r#"---
  - !directory
      name: test
      content:
        - !file
            name: test.txt
            content:
              !empty
  "#;
    const YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML: &str = "
  - !directory
      name: test
      content:
        - !file
            name: test_from_cargo.toml
            content:
              !original_file Cargo.toml
  ";

    const YAML_DOUBLE_ROOT_DIRS: &str = "
  - !directory
      name: test
      content:
        - !file
            name: test_from_cargo.toml
            content:
              !original_file Cargo.toml
  - !directory
      name: bad_dir
      content:
        -!file
            name: test.txt
            content:
              !inline_text test
  ";

    #[test]
    fn constructor_should_throw_error_when_empty_config() {
        let res = FsTester::new("", ".");
        assert!(res.is_err());
    }

    #[test]
    fn constructor_should_return_error_when_base_dir_not_found() -> Result<()> {
        let res = FsTester::new(YAML_DIR_WITH_EMPTY_FILE, "unexisting_directory");
        assert!(res.is_err());
        if let Err(error) = res {
            if error.is_should_start_from_directory() {
                Ok(())
            } else {
                Err(error)
            }
        } else {
            panic!("Error expected but constructor returned Ok");
        }
    }

    #[test]
    fn constructor_should_return_error_when_conf_starts_from_file() -> Result<()> {
        let config_started_from_file = "
    - !file
        name: test.txt
        content:
          !empty
    ";

        let res = FsTester::new(config_started_from_file, ".");
        assert!(res.is_err());
        if let Err(error) = res {
            if error.is_should_start_from_directory() {
                Ok(())
            } else {
                Err(error)
            }
        } else {
            panic!("Error expected but constructor returned Ok");
        }
    }

    #[test]
    // #[should_panic(expected = "The configuration should start from the containing directory.")]
    fn constructor_should_return_error_when_conf_starts_from_link() -> Result<()> {
        let config_started_from_file = "
    - !link
        name: test_link.txt
        target: test.txt
    ";

        let res = FsTester::new(config_started_from_file, ".");
        assert!(res.is_err());

        if let Err(error) = res {
            if error.is_should_start_from_directory() {
                Ok(())
            } else {
                Err(error)
            }
        } else {
            panic!("Error expected but constructor returned Ok");
        }
    }

    #[test]
    fn parser_should_accept_json_correct_simple_config() {
        assert_eq!(
            FsTester::parse_config("[{\"directory\":{\"name\": \".\",\"content\": []}}]").unwrap(),
            Configuration(vec!(ConfigEntry::Directory(DirectoryConf {
                name: String::from("."),
                content: Vec::new()
            }))),
        );
    }

    #[test]
    fn serialization_for_simple_json_config() {
        let conf: Configuration = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("."),
            content: Vec::new(),
        })]);

        assert_eq!(
            String::from("[{\"directory\":{\"name\":\".\",\"content\":[]}}]"),
            serde_json::to_string(&conf).unwrap(),
        );
    }

    #[test]
    fn parser_should_accept_yaml_correct_simple_config() {
        assert_eq!(
            Configuration(vec!(ConfigEntry::Directory(DirectoryConf {
                name: String::from("."),
                content: Vec::new()
            }))),
            FsTester::parse_config("---\n- !directory\n    name: \".\"\n    content: []\n")
                .unwrap(),
        );
    }

    #[test]
    fn parser_should_accept_yaml_config_with_directory_and_file_by_inline_bytes() {
        let simple_conf_str = "
    - !directory
        name: test
        content:
        - !file
            name: test.txt
            content:
              !inline_bytes
              - 116
              - 101
              - 115
              - 116
    ";
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::InlineBytes(String::from("test").into_bytes()),
            })],
        })]);

        assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    }

    #[test]
    fn parser_should_accept_yaml_config_with_directory_and_file_by_inline_text() {
        let simple_conf_str = "
    - !directory
        name: test
        content:
        - !file
            name: test.txt
            content:
              !inline_text test
    ";
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::InlineText(String::from("test")),
            })],
        })]);

        assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    }

    #[test]
    fn parser_should_accept_yaml_config_with_directory_and_file_by_original_path() {
        let simple_conf_str = "
    - !directory
        name: test
        content:
        - !file
            name: test.txt
            content:
              !original_file sample_test.txt
    ";
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::OriginalFile(String::from("sample_test.txt")),
            })],
        })]);

        assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    }

    #[test]
    fn parser_should_accept_yaml_config_with_directory_and_file_by_empty() {
        let simple_conf_str = "
    - !directory
        name: test
        content:
        - !file
            name: test.txt
            content:
              !empty
    ";
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::Empty,
            })],
        })]);

        assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    }

    #[test]
    fn parser_should_accept_json_config_with_directory_and_file() {
        let simple_conf_str =
      "[{\"directory\":{\"name\":\"test\",\"content\":[{\"file\":{\"name\":\"test.txt\",\"content\":{\"inline_bytes\":[116,101,115,116]}}}]}}]";
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::InlineBytes(String::from("test").into_bytes()),
            })],
        })]);

        assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
    }

    #[test]
    fn parser_should_accept_yaml_config_with_directory_and_file_and_link() {
        let simple_conf_str = "
    - !directory
        name: test
        content:
        - !file
            name: test.txt
            content:
              !inline_bytes
              - 116
              - 101
              - 115
              - 116
        - !link
            name: test_link.txt
            target: test.txt
    ";
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![
                ConfigEntry::File(FileConf {
                    name: String::from("test.txt"),
                    content: FileContent::InlineBytes(String::from("test").into_bytes()),
                }),
                ConfigEntry::Link(LinkConf {
                    name: String::from("test_link.txt"),
                    target: String::from("test.txt"),
                }),
            ],
        })]);

        let parsed_config = FsTester::parse_config(simple_conf_str).unwrap();

        assert_eq!(test_conf, parsed_config);
    }

    #[test]
    fn serialization_for_simple_yaml_config() {
        let conf: Configuration = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("."),
            content: Vec::new(),
        })]);

        assert_eq!(
            String::from("- !directory\n  name: .\n  content: []\n"),
            serde_yaml::to_string(&conf).unwrap(),
        );
    }

    #[test]
    fn start_simple_successful_test_should_be_success() -> Result<()> {
        use std::fs;

        let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".")?;
        tester.perform_fs_test(|dirname| {
            let inner_file_name = format!("{}/{}", dirname, "test_from_cargo.toml");
            let metadata = fs::metadata(inner_file_name)?;

            assert!(metadata.len() > 0);
            Ok(())
        });
        Ok(())
    }

    #[test]
    #[should_panic]
    fn start_simple_failed_test_should_be_success() {
        use std::fs;

        let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".")
            .expect("Configuration parsing fail");
        tester.perform_fs_test(|dirname| {
            let inner_file_name = format!("{}/{}", dirname, "test_from_cargo.toml");
            let metadata = fs::metadata(inner_file_name)?;

            assert!(metadata.len() == 0);
            Ok(())
        });
    }

    #[test]
    fn yaml_config_serialization_explorer() {
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::OriginalFile(String::from("Cargo.toml")),
            })],
        })]);

        let config = serde_yaml::to_string(&test_conf).unwrap();
        assert!(config.contains("test.txt"));
        assert!(config.contains("Cargo.toml"));
    }

    // This test needs to explore the JSON format of the config string.
    // To see the serialized string from a config,
    // you need to write the config object into test_conf and change assert_ne! to assert_eq! in the code.
    // The serialized result will be shown in the error message.
    // While this is not very pretty, it is very fast.
    #[test]
    fn json_config_serialization_explorer() {
        let test_conf = Configuration(vec![ConfigEntry::Directory(DirectoryConf {
            name: String::from("test"),
            content: vec![ConfigEntry::File(FileConf {
                name: String::from("test.txt"),
                content: FileContent::OriginalFile(String::from("Cargo.toml")),
            })],
        })]);

        let config = serde_json::to_string(&test_conf).unwrap();
        assert!(config.contains("test.txt"));
        assert!(config.contains("Cargo.toml"));
    }
}
