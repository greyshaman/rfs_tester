use futures::future::BoxFuture;
use futures::FutureExt;
use rand::Rng;
use std::env;
use std::{
    io::{self},
    path::{Path, PathBuf},
};
use tokio::fs::{self, hard_link, File};
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

use crate::rfs::fs_tester_error::{FsTesterError, Result};

use super::config::clone_directory_conf::CloneDirectoryConf;
use super::config::config_entry::ConfigEntry;
use super::config::configuration::Configuration;
use super::config::directory_conf::DirectoryConf;
use super::config::file_content::FileContent;
use super::config::{FileConf, LinkConf};

const LINKS_ALLOWED_VAR_NAME: &str = "LINKS_ALLOWED";

struct Permissions {
    links_allowed: bool,
}

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
        rand::rng().random::<u64>()
    }

    fn gen_dir_path(dir_path: &PathBuf, name: &str, level: u32) -> PathBuf {
        if level == 0 {
            let uniq_code = Self::get_random_code();
            dir_path.join(format!("{}_{}", name, uniq_code))
        } else {
            dir_path.join(name)
        }
    }

    async fn create_dir(dirname: &PathBuf) -> Result<String> {
        fs::create_dir_all(dirname).await?;

        Ok(dirname.to_string_lossy().into_owned())
    }

    async fn copy_dir(
        src_path: &PathBuf,
        dst_path: &PathBuf,
        permissions: &Permissions,
    ) -> Result<String> {
        let dst_dir_name = Self::create_dir(dst_path).await?;
        // Reading source dir
        let src_dir_entries_iter = WalkDir::new(src_path).into_iter();
        for entry in src_dir_entries_iter {
            let entry = entry?;
            let entry_metadata = entry.clone().metadata()?;
            let src_entry_path = entry.path();
            let filename = src_entry_path
                .into_iter()
                .last()
                .expect("source dir should not be empty");
            let dst_entry_path = dst_path.join(filename);
            if entry_metadata.is_file() {
                // copy file
                let mut src_file = File::open(src_entry_path).await?;
                let mut dst_file = File::create(dst_entry_path).await?;
                tokio::io::copy(&mut src_file, &mut dst_file).await?;
            } else if entry_metadata.is_dir() {
                // start recursion for child dir
                let src_entry_path = PathBuf::from(src_entry_path);
                Self::copy_dir_boxed(&src_entry_path, &dst_entry_path, permissions).await?;
            }
        }
        Ok(dst_dir_name)
    }

    async fn create_file(conf: &FileConf, dir_path: &PathBuf) -> Result<String> {
        let dst_file_name = dir_path.join(&conf.name);
        let mut dst_file = File::create(&dst_file_name).await?;

        match &conf.content {
            FileContent::InlineBytes(data) => {
                dst_file.write_all(&data).await?;
            }
            FileContent::InlineText(text) => {
                dst_file.write_all(text.as_bytes()).await?;
            }
            FileContent::OriginalFile(file_path) => {
                let mut src_file = File::open(file_path).await?;
                tokio::io::copy(&mut src_file, &mut dst_file).await?;
            }
            FileContent::Empty => {}
        }

        Ok(dst_file_name.to_string_lossy().into_owned())
    }

    /// WARNING!!! Use links with caution, as making changes to the content using a link may modify the original file.
    async fn create_link(
        conf: &LinkConf,
        dir_path: &PathBuf,
        permissions: &Permissions,
    ) -> Result<String> {
        let link_name = dir_path.join(&conf.name);
        let target_name = PathBuf::from(&conf.target);
        if permissions.links_allowed {
            hard_link(target_name, &link_name).await?;

            Ok(link_name.to_string_lossy().into_owned())
        } else {
            Err(FsTesterError::not_allowed_settings())
        }
    }

    async fn clone_directory(
        conf: &CloneDirectoryConf,
        parent_path: &PathBuf,
        level: u32,
        permissions: &Permissions,
    ) -> Result<String> {
        let dst_dir_path = Self::gen_dir_path(parent_path, &conf.name, level);
        let src_dir_path = parent_path.join(&conf.source);

        Self::copy_dir(&src_dir_path, &dst_dir_path, permissions)
            .await
            .map_err(|mut err| {
                if level == 0 {
                    err.set_sandbox_dir(Some(dst_dir_path.to_string_lossy().into_owned()));
                }
                err
            })?;

        Ok(dst_dir_path.to_string_lossy().into_owned())
    }

    async fn build_directory_with_content(
        directory_conf: &DirectoryConf,
        parent_path: &PathBuf,
        level: u32,
        permissions: &Permissions,
    ) -> Result<String> {
        let dst_dir_path = Self::gen_dir_path(parent_path, &directory_conf.name, level);

        Self::create_dir(&dst_dir_path).await?;

        for entry in &directory_conf.content {
            match entry {
                ConfigEntry::Directory(conf) => Self::build_directory_with_content_boxed(
                    conf,
                    &dst_dir_path,
                    level + 1,
                    permissions,
                )
                .await
                .map_err(|mut err| {
                    if level == 0 {
                        err.set_sandbox_dir(Some(String::from(dst_dir_path.to_string_lossy())));
                    }
                    err
                })?,
                ConfigEntry::CloneDirectory(conf) => {
                    Self::clone_directory(conf, &dst_dir_path, level, permissions).await?
                }
                ConfigEntry::File(conf) => Self::create_file(conf, &dst_dir_path).await?,
                ConfigEntry::Link(conf) => {
                    Self::create_link(conf, &dst_dir_path, permissions).await?
                }
            };
        }

        Ok(dst_dir_path.to_string_lossy().into_owned())
    }

    fn build_directory_with_content_boxed<'a>(
        conf: &'a DirectoryConf,
        parent_path: &'a PathBuf,
        level: u32,
        permissions: &'a Permissions,
    ) -> BoxFuture<'a, Result<String>> {
        async move {
            Self::build_directory_with_content(conf, parent_path, level, permissions)
                .await
        }
        .boxed()
    }

    fn copy_dir_boxed<'a>(
        src_dir: &'a PathBuf,
        dst_path: &'a PathBuf,
        permissions: &'a Permissions,
    ) -> BoxFuture<'a, Result<String>> {
        async move { Self::copy_dir(src_dir, dst_path, permissions).await }.boxed()
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
        let links_allowed =
            env::var(LINKS_ALLOWED_VAR_NAME).unwrap_or_else(|_| "N".to_string()) != "N";
        let permissions = Permissions { links_allowed };

        let config: Configuration = Self::parse_config(config_str)?;

        // The directory where the temporary test sandbox will be created.
        let base_dir = if start_point.len() == 0 {
            // If the starting point is not provided as an argument, we will use the current location.
            PathBuf::from(".")
        } else {
            if Path::new(start_point).is_dir() {
                PathBuf::from(start_point)
            } else {
                return Err(FsTesterError::should_start_from_directory());
            }
        };

        // Checks if the configuration starts from a single config entry (Directory or CloneDirectory).
        if config.0.len() != 1 {
            return Err(FsTesterError::should_start_from_directory());
        }
        // After previous check we can take ConfigEntry
        let root_config_entry = config
            .0
            .iter()
            .next()
            .expect("zero level of configuration should have only one entry");
        // And do verification if the configuration entry is Directory or CloneDirectory
        let result = match root_config_entry {
            ConfigEntry::Directory(conf) => {
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(Self::build_directory_with_content(
                    &conf,
                    &PathBuf::from(&base_dir),
                    0,
                    &permissions,
                ))
            }
            ConfigEntry::CloneDirectory(conf) => {
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(Self::clone_directory(
                    &conf,
                    &PathBuf::from(&base_dir),
                    0,
                    &permissions,
                ))
            }
            _ => return Err(FsTesterError::should_start_from_directory()),
            // None => return Err(FsTesterError::empty_config()),
        };

        if let Err(error) = result {
            if let Some(dst_dir_path) = error.sandbox_dir() {
                if std::fs::metadata(&dst_dir_path)?.is_dir() {
                    // Delete a temporary directory if an error occured while filling it in.
                    std::fs::remove_dir_all(&dst_dir_path)?;
                }
            }
            return Err(error);
        }

        Ok(FsTester {
            config,
            base_dir: String::from(base_dir.to_string_lossy()),
        })
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
        if let Err(e) = std::fs::remove_dir_all(&self.base_dir) {
            eprintln!("Failed to delete directory: {}", e);
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

    const YAML_DIR_WITH_LINK: &str = r#"
    - !directory
        name: test
        content:
            - !link
                name: cargo_link
                target: Cargo.toml
    "#;

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
        - !file
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
    fn constructor_should_return_error_when_double_root_dir_in_config() -> Result<()> {
        let res = FsTester::new(YAML_DOUBLE_ROOT_DIRS, ".");
        assert!(res.is_err());
        if let Err(error) = res {
            if error.is_should_start_from_directory() {
                Ok(())
            } else {
                Err(error)
            }
        } else {
            unreachable!("res.is_ok already above handled by assert");
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
        let simple_conf_str = "[{\"directory\":{\"name\":\"test\",\"content\":[{\"file\":{\"name\":\"test.txt\",\"content\":{\"inline_bytes\":[116,101,115,116]}}}]}}]";
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
    fn create_test_dir_with_link_dependent_from_links_allowed_env_var() {
        if env::var("LINKS_ALLOWED") == Ok("Y".to_string()) {
            let tester_result = FsTester::new(YAML_DIR_WITH_LINK, ".");
            assert!(tester_result.is_ok());
        } else {
            let tester_result = FsTester::new(YAML_DIR_WITH_LINK, ".");
            if let Err(error) = tester_result {
                assert!(error.is_not_allowed_settings());
            } else {
                panic!("should return error");
            }
        }
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

    #[test]
    fn many_files_test() -> Result<()> {
        let conf = r#"
        - !directory
            name: base_dir
            content:
                - !file
                    name: test_from_cargo.toml
                    content:
                        !original_file Cargo.toml
                - !directory
                    name: dir_1_1
                    content:
                        - !file
                            name: text_test.txt
                            content:
                                !inline_text "test"
                        - !file
                            name: empty_file.txt
                            content: !empty
                        - !directory
                            name: dir_2_1
                            content:
                                - !file
                                    name: empty_file.txt
                                    content: !empty
                - !directory
                    name: dir_1_2
                    content:
                        - !file
                            name: test_from_cargo_2.toml
                            content:
                                !original_file Cargo.toml
        "#;
        let tester = FsTester::new(conf, ".")?;
        tester.perform_fs_test(|dirname| {
            let inner_file_name = PathBuf::from(dirname).join("test_from_cargo.toml");
            let metadata = std::fs::metadata(inner_file_name)?;

            assert!(metadata.len() > 0);

            let dir_1_1 = std::fs::metadata(PathBuf::from(dirname).join("dir_1_1"))?;
            assert!(dir_1_1.is_dir());

            let dir_1_2 = std::fs::metadata(PathBuf::from(dirname).join("dir_1_2"))?;
            assert!(dir_1_2.is_dir());
            Ok(())
        });
        Ok(())
    }
}
