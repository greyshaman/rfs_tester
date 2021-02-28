use std::fs::{self, DirBuilder, File};
use std::io::{self, Write, Read};
use std::result as std_result;
use rand::Rng;
use serde::{Serialize, Deserialize};
use std::path::Path;

use crate::rfs::error::FsTesterError;

/// Customized result type to handle config parse error
type Result<T> = std_result::Result<T, Box<dyn std::error::Error>>;

/// Struct for directory record in configuration
/// for example:
/// 
/// ## yaml:
/// 
/// ```yaml
/// ---
///   - directory:
///       name: test
///       content:
///         - file:
///             name: test.txt
///             content: empty
///         - link:
///             name: test_link
///             target: test.txt
/// ```
/// 
/// ## json:
/// 
/// ```json
/// {
///   "name": "test_dir",
///   "content": [
///     "file": {
///       "name": "test.txt",
///       "content": "empty"
///     },
///     "link": {
///       "name": "test_link",
///       "target": "test.txt"
///     }
///   ]
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DirectoryConf {
  pub name: String,
  pub content: Vec<ConfigEntry>,
}

/// Struct for file record in configuration.
/// File can be configured as empty, with bytes array or with reference to real file whitch body will be used as content
/// for test file
/// 
/// ## Empty file
/// 
/// ### yaml:
/// 
/// ```yaml
/// - file:
///     name: test.txt
///     content: empty
/// ```
/// 
/// ### json:
/// 
/// ```json
/// "file": {
///   "name": "test.txt",
///   "content": "empty"
/// }
/// ```
/// ## Inline
/// File content can be configured as **inline**. When you use inline you should add bytes array in configuration.
/// This configuration case usefull only for small test files
/// 
/// Example of configuration for file with name **test.txt** and with "test" in content
/// 
/// ### yaml
/// 
/// ```yaml
/// - file:
///     name: test.txt
///     content:
///       inline:
///         - 106
///         - 101
///         - 115
///         - 116
/// ```
/// 
/// ### json
/// 
/// ```json
/// "file": {
///   "name": "test.txt",
///   "content": {
///     "inline": [116, 101, 115, 116]
///   }
/// }
/// ```
/// 
/// ## Original file
/// In case we need bigger file then we can spicify by **inline**,
/// we can set path to original file in file system and its contens will be copied to test file
/// 
/// For example we have music.mp3 file and want to have test file with same content
/// 
/// ### yaml
/// 
/// ```yaml
/// - file:
///     name: test.mp3
///     content:
///       original_file: "./music.mp3"
/// ```
/// 
/// ### json
/// ```json
/// "file" : {
///   "name": "test.mp3",
///   "content": {
///     "original_file": "./music.mp3"
///   }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FileConf {
  pub name: String,
  pub content: FileContent,
}

/// File content can be present by three ways
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all="snake_case")]
pub enum FileContent {
  /// Inline - by vector of bytes
  /// 
  /// ```yaml
  /// - file:
  ///     name: test.txt
  ///     content
  ///       inline: 
  ///         - 116            
  ///         - 101            
  ///         - 115            
  ///         - 116
  /// ```
  Inline(Vec<u8>),
  /// Get from real file by its path
  /// 
  /// ```yaml
  /// - file:
  ///     name: test.txt
  ///     content:
  ///       original_file: "test.txt"
  /// ```
  OriginalFile(String),
  /// or simply Empty
  /// 
  /// ```yaml
  /// - file: 
  ///     name: test.txt
  ///     content: empty
  Empty,
}

/// Struct for configuration link 
/// 
/// link can be refering to other test file
/// 
/// ### yaml
/// 
/// ```yaml
/// - link:
///     name: test_link
///     target: test.txt
/// ```
/// 
/// ### json
/// ```json
/// "link": {
///   "name": "test_link",
///   "target": "test.txt"
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct LinkConf {
  pub name: String,
  pub target: String,
}

/// FS Config entry enum
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all="snake_case")]
pub enum ConfigEntry {
  Directory(DirectoryConf),
  File(FileConf),
  Link(LinkConf),
}

/// File System config structure to contains directories, files and links
/// to execute tests with fs io operations
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Config(pub Vec<ConfigEntry>);

/// File System Tester used to create some configured structure in directory with 
/// files and links to them. FsTester can start custom test closure and remove fs
/// structure after testing complete or fail.
pub struct FsTester {
  pub config: Config,
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

  fn create_link(link_name: &str, target_name: &str) -> std_result::Result<String, io::Error> {
    fs::hard_link(target_name, link_name)?; // TODO: try to use platform based softlink
  
    Ok(String::from(link_name))
  }

  fn delete_test_set(dirname: &str) -> std_result::Result<(), io::Error> {
    fs::remove_dir_all(dirname)?;
    Ok(())
  }

  fn build_directory(
    directory_conf: &DirectoryConf,
    parent_path: &str,
    level: i32
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
                                            //requrement propbably better way existing
      let result = match entry {
        ConfigEntry::Directory(conf) =>
          Self::build_directory(conf, &dir_path, level + 1),
        ConfigEntry::File(conf) => {
          let file_name: String = format!("{}/{}", &dir_path, conf.name);
          let content: &[u8] = match &conf.content {
            FileContent::Inline(data) => data,
            FileContent::OriginalFile(file_path) => {
              let mut original_file = File::open(file_path)?;
              original_file.read_to_end(&mut buffer)?;
              &buffer
            }
            FileContent::Empty => &[]
          };
          Self::create_file(&file_name, &content)
        }
        ConfigEntry::Link(conf) => {
          let link_name = format!("{}/{}", &dir_path, conf.name);
          Self::create_link(&link_name, &conf.target)
        },
      };
      if let Err(e) = result {
        panic!("{}", e);
      }
    }

    Ok(dir_path)
  }

  /// Config parser
  /// config can be string in yaml or json format:
  /// 
  /// ## Example for yaml
  /// 
  /// ```rust
  /// # use rfs_tester::{FsTester, FsTesterError};
  /// # use rfs_tester::rfs::fs_tester::{ Config, ConfigEntry, DirectoryConf, FileConf, LinkConf, FileContent };
  /// let simple_conf_str = "---
  ///   - directory:
  ///       name: test
  ///       content:
  ///         - file:
  ///             name: test.txt
  ///             content:
  ///               inline:            
  ///                 - 116            
  ///                 - 101            
  ///                 - 115            
  ///                 - 116            
  /// ";
  /// # let test_conf = Config(vec!(ConfigEntry::Directory(
  /// #   DirectoryConf {
  /// #     name: String::from("test"),
  /// #     content: vec!(
  /// #       ConfigEntry::File(
  /// #         FileConf {
  /// #           name: String::from("test.txt"),
  /// #           content: 
  /// #             FileContent::Inline(
  /// #               String::from("test").into_bytes(),
  /// #             )
  /// #         }
  /// #       )
  /// #     ),
  /// #   }
  /// # )));
  /// #   
  /// # assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
  /// ```
  /// 
  /// ## Example for json
  /// 
  /// ```rust
  /// # use rfs_tester::{FsTester, FsTesterError};
  /// # use rfs_tester::rfs::fs_tester::{ Config, ConfigEntry, DirectoryConf, FileConf, LinkConf, FileContent };
  /// let simple_conf_str = 
  ///   "[{\"directory\":{\"name\":\"test\",\"content\":[{\"file\":{\"name\":\"test.txt\",\"content\":{\"inline\":[116,101,115,116]}}}]}}]";
  /// # let test_conf = Config(vec!(ConfigEntry::Directory(
  /// #   DirectoryConf {
  /// #     name: String::from("test"),
  /// #     content: vec!(
  /// #       ConfigEntry::File(
  /// #         FileConf {
  /// #           name: String::from("test.txt"),
  /// #           content: 
  /// #             FileContent::Inline(
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
  pub fn parse_config(config_str: &str) -> Result<Config> {
    // detect format parse and return config instance
    match config_str.chars().next() {
      Some('{') => serde_json::from_str(config_str).or_else(|error| Err(error.into())),
      Some(_) => serde_yaml::from_str(config_str).or_else(|error| Err(error.into())),
      None => Err(FsTesterError::EmptyConfig.into()), 
    }
  }
  
  /// create the test directory, files and link set
  /// config_str - configuration of test directory in yaml or json format
  /// start_point - directory name where will create testing directory, it should 
  ///               presents in FS.
  pub fn new(config_str: &str, start_point: &str) -> FsTester {
    let config: Config = match Self::parse_config(config_str) {
      Ok(conf) => conf,
      Err(error) => panic!("{}", error),
    };
    let base_dir = if start_point.len() == 0 {
      String::from(".")
    } else {
      if Path::new(start_point).is_dir() {
        String::from(start_point)
      } else {
        // return Err(FsTesterError::BaseDirNotFound.into());
        panic!("Base directory not found!");
      }
    };

    // check if config started from single directory
    let zero_level_config_ref: Option<&ConfigEntry> = config.0.iter().next();
    let directory_conf = match zero_level_config_ref {
      Some(entry) => match entry {
        ConfigEntry::File(_) | ConfigEntry::Link(_) => 
          // return Err(FsTesterError::ShouldFromDirectory.into()),
          panic!("Config should start from containing directory"),
        ConfigEntry::Directory(conf) => conf
      }
      // None => return Err(FsTesterError::EmptyConfig.into()),
      None => panic!("Config should not be empty"),
    };

    let base_dir = Self::build_directory(&directory_conf, &base_dir, 0).unwrap();

    FsTester { config, base_dir }
  }

  /// prepare testing fs structure, start test_proc function and remove directory after
  /// testing complete.
  pub fn perform_fs_test<F>(&self, test_proc: F) 
    where F: Fn(&str) -> io::Result<()>,
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
  use super::*;

  const YAML_DIR_WITH_EMPTY_FILE: &str = "---
  - directory:
      name: test
      content:
        - file:
            name: test.txt
            content:
              empty:
  ";
  const YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML: &str = "---
  - directory:
      name: test
      content:
        - file:
            name: test_from_cargo.toml
            content:
              original_file: Cargo.toml
  ";
  
  #[test]
  #[should_panic(expected = "Config should not be empty")]
  fn constructor_should_throw_error_when_empty_config() {
      FsTester::new("", ".");
  }

  #[test]
  #[should_panic(expected = "Base directory not found!")]
  fn constructor_should_panic_when_base_dir_not_found() {
      FsTester::new(YAML_DIR_WITH_EMPTY_FILE, "unexisting_directory");
  }

  #[test]
  #[should_panic(expected = "Config should start from containing directory")]
  fn constructor_should_panic_when_conf_starts_from_file() {
    let config_started_from_file = "---
    - file:
        name: test.txt
        content:
          empty:
    ";
    
    FsTester::new(config_started_from_file, ".");
  }

  #[test]
  #[should_panic(expected = "Config should start from containing directory")]
  fn constructor_should_panic_when_conf_starts_from_link() {
    let config_started_from_file = "---
    - link:
        name: test_link.txt
        target: test.txt
    ";
    
    FsTester::new(config_started_from_file, ".");
  }

  #[test]
  fn parser_should_accept_json_correct_simple_config() {
      assert_eq!(
        FsTester::parse_config("[{\"directory\":{\"name\": '.',\"content\": []}}]").unwrap(),
        Config(vec!(ConfigEntry::Directory(
          DirectoryConf { name: String::from("."), content: Vec::new() }
        ))),
      );
  }

  #[test]
  fn serialization_for_simple_json_config() {
      let conf: Config = Config(vec!(ConfigEntry::Directory(
        DirectoryConf { name: String::from("."), content: Vec::new() }
      )));

      assert_eq!(
        String::from("[{\"directory\":{\"name\":\".\",\"content\":[]}}]"),
        serde_json::to_string(&conf).unwrap(),
      );
  }

  #[test]
  fn parser_should_accept_yaml_correct_simple_config() {
      assert_eq!(
        Config(vec!(ConfigEntry::Directory(
          DirectoryConf { name: String::from("."), content: Vec::new() }
        ))),
        FsTester::parse_config(
          "---\n- directory:\n    name: \".\"\n    content: []\n"
        ).unwrap(),
      );
  }

  #[test]
  fn parser_should_accept_yaml_config_with_directory_and_file_by_inline() {
    let simple_conf_str = "---
    - directory:
        name: test
        content:
        - file:
            name: test.txt
            content:
              inline:            
              - 116            
              - 101            
              - 115            
              - 116            
    ";
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::Inline(
                  String::from("test").into_bytes(),
                )
            }
          )
        ),
      }
    )));

    assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
  }

  #[test]
  fn parser_should_accept_yaml_config_with_directory_and_file_by_original_path() {
    let simple_conf_str = "---
    - directory:
        name: test
        content:
        - file:
            name: test.txt
            content:
              original_file: sample_test.txt            
    ";
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::OriginalFile(
                  String::from("sample_test.txt"),
                )
            }
          )
        ),
      }
    )));

    assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
  }

  #[test]
  fn parser_should_accept_yaml_config_with_directory_and_file_by_empty() {
    let simple_conf_str = "---
    - directory:
        name: test
        content:
        - file:
            name: test.txt
            content:
              empty:            
    ";
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::Empty
            }
          )
        ),
      }
    )));

    assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
  }

  #[test]
  fn parser_should_accept_json_config_with_directory_and_file() {
    let simple_conf_str = 
      "[{\"directory\":{\"name\":\"test\",\"content\":[{\"file\":{\"name\":\"test.txt\",\"content\":{\"inline\":[116,101,115,116]}}}]}}]";
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::Inline(
                  String::from("test").into_bytes(),
                )
            }
          )
        ),
      }
    )));

    assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
  }

  #[test]
  fn parser_should_accept_yaml_config_with_directory_and_file_and_link() {
    let simple_conf_str = "---
    - directory:
        name: test
        content:
        - file:
            name: test.txt
            content:
              inline:            
              - 116            
              - 101            
              - 115            
              - 116            
        - link:
            name: test_link.txt
            target: test.txt
    ";
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::Inline(
                  String::from("test").into_bytes(),
                )
            }
          ),
          ConfigEntry::Link(
            LinkConf {
              name: String::from("test_link.txt"),
              target: String::from("test.txt"),
            }
          )
        ),
      }
    )));

    assert_eq!(test_conf, FsTester::parse_config(simple_conf_str).unwrap());
  }

  #[test]
  fn serialization_for_simple_yaml_config() {
    let conf: Config = Config(vec!(ConfigEntry::Directory(
      DirectoryConf { name: String::from("."), content: Vec::new() }
    )));

    assert_eq!(
      String::from("---\n- directory:\n    name: \".\"\n    content: []\n"),
      serde_yaml::to_string(&conf).unwrap(),
    );
  }

  #[test]
  fn start_simple_successfull_test_should_be_success() {
    use std::fs;

    let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".");
    tester.perform_fs_test(|dirname| {
      let inner_file_name = format!("{}/{}", dirname, "test_from_cargo.toml");
      let metadata = fs::metadata(inner_file_name)?;
      
      assert!(metadata.len() > 0);
      Ok(())
    });
  }

  #[test]
  #[should_panic]
  fn start_simple_failed_test_should_be_success() {
    use std::fs;

    let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".");
    tester.perform_fs_test(|dirname| {
      let inner_file_name = format!("{}/{}", dirname, "test_from_cargo.toml");
      let metadata = fs::metadata(inner_file_name)?;
      
      assert!(metadata.len() == 0);
      Ok(())
    });
  }

  // //////////////////////////////////////////////////////////////////////////////////
  // This test need to explore the yaml format of config string.
  // To see serialized string from some config 
  // you need write the config object in test_conf and change assert_ne! to assert_eq!.
  // Serialized result will be show in error message. This is not pretty, but very fast.
  // //////////////////////////////////////////////////////////////////////////////////
  #[test]
  #[ignore]
  fn yaml_config_serialization_explorer() {
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::OriginalFile(String::from("Cargo.toml"))
            }
          )
        ),
      }
    )));

    assert_eq!(String::new(), serde_yaml::to_string(&test_conf).unwrap());
  }

  // This test need to explore the json format of config string.
  // To see serialized string from some config 
  // you need write the config object in test_conf and change assert_ne! to assert_eq!.
  // Serialized result will be show in error message. This is not pretty, but very fast.
  #[test]
  #[ignore]
  fn json_config_serialization_explorer() {
    let test_conf = Config(vec!(ConfigEntry::Directory(
      DirectoryConf {
        name: String::from("test"),
        content: vec!(
          ConfigEntry::File(
            FileConf {
              name: String::from("test.txt"),
              content: 
                FileContent::OriginalFile(String::from("Cargo.toml"))
            }
          )
        ),
      }
    )));

    assert_eq!(String::new(), serde_json::to_string(&test_conf).unwrap());
  }
}
