# Rfs_tester

<center><img src="https://github.com/greyshaman/rfs_tester/raw/refs/heads/master/images/sketch.webp" width="50%" alt="Rfs_tester Logo"></center>

[![Crates.io](https://img.shields.io/crates/v/rfs_tester)](https://crates.io/crates/rfs_tester)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://crates.io/crates/rfs_tester)
[![Coverage](https://github.com/greyshaman/rfs_tester/raw/refs/heads/master/images/flat.svg)](https://crates.io/crates/rfs_tester)

A Rust library for testing file system operations with temporary directories.

## Features

- Create temporary directories and files for testing.
- Automatically clean up after tests.
- Flexible configuration using YAML or JSON.
- Create copy of specified directories.
- Working in parallel to speed up the creation of sandboxes for testing.

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dev-dependencies]
rfs_tester = "1.1.2"
```

or

```toml
[dependencies]
rfs_tester = "1.1.2"
```

## Overview

This library provides a simple utility for testing file system operations.
When you are testing something, you need a sandbox that can be wiped out after the testing is finished.
This package allows you to configure a temporary directory and its internal structure, perform tests, and remove it when all the work is done. To speed up the process of creating a temporary directory, the contents are loaded asynchronously.

It can be configured to generate a customized directory structure. The main idea behind this package is to write test cases for a program that needs to work with the file system, manipulating directories, files, and links to them. The tests require a template directory with several files at various levels of the file system structure.

The random generator is used to add uniqueness to the temporary directory name, which contains other test file system units.

## Configuration

The test directory structure can be configured using the YAML or JSON format.

> __WARNING!!!__ Use links with caution, as making changes to the content using a link may modify the original file.
>
> By default, links are disabled to prevent users from accidentally damaging files. In order to enable link support, users must set the "Y" value of the LINKS_ALLOWED environment variable prior to running link tests. If this variable has not been set and a link is found in the configuration for any test, users will be notified with an error message and brief instructions. This way, you can enable link support, but do so at your own risk.
>
> Example:
>
> ```bash
> LINKS_ALLOWED=Y cargo test
> ```
>

### Yaml configuration

```yaml
---
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
```

It will create a directory named **test_726537253725**, and a file named **test.txt** within that directory, which will contain the text "test". The number in the directory name may vary, as it is a random number.

### Json configuration

The same directory structure can be configured using JSON format:

```json
[
  {
    "directory":
      {
        "name": "test",
        "content":
          [
            {
              "file":
                {
                  "name": "test.txt",
                  "content":
                    {
                      "inline_bytes": [116,101,115,116]
                    }
                }
            }
          ]
      }
  }
]
```

### Directory configuration

The directory structure can contain many nested directories. However, it is important to note that the first level of the configuration should begin with a single directory. This directory will serve as a sandbox container, with a name that includes a randomly generated number. Other inner components, such as directories, files, and links, should not change their original names and can continue to be used for testing purposes in the configuration.

A new feature has been added that allows you to create a copy of a specified directory. This cloned directory can then be used as the root of a sandbox.

Directory configuration can specify the name and content of:

- name - string representing the directory name
- content - a list of internal file system elements (directories, files, links).

Example using the YAML:

```yaml
---
  - !directory
      name: test
      content:
        - !file
            name: test.txt
            content: empty
        - !link
            name: test_link
            target: test.txt
```

or the same using the JSON:

```json
{
  "name": "test",
  "content": [
    {
      "file": {
        "name": "test.txt",
        "content": "empty"
      }
    },
    {
      "link": {
        "name": "test_link",
        "target": "test.txt"
      }
    }
  ]
}
```

### Configuration example of cloning directory

```ymal
- !clone_directory
    name: test_yaml_config_with_clone_directory
    source: src
```

## How to Define a Test?

When we want to test files, directories, and links in the created sandbox, we need to know the exact name of the outer directory. This name will be unique each time `FsTester` creates it. `FsTester` provides us with this name as a closure parameter in the `perform_fs_test` function.

Example:

```rust
#[cfg(test)]
mod tests {
  use std::fs;
  use rfs_tester::{FsTester, FileContent};
  use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};

  #[test]
  fn test_file_creation() {
    const YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML: &str = "
    - !directory
        name: test
        content:
          - !file
              name: test_from_cargo.toml
              content:
                !original_file Cargo.toml
     ";

    let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".").expect("Incorrect configuration");
    tester.perform_fs_test(|dirname| {
    //                      ^^^^^^^ name with a random number at the end
      let inner_file_name = std::path::PathBuf::from(dirname).join("test_from_cargo.toml");
      let metadata = fs::metadata(inner_file_name)?;

      assert!(metadata.len() > 0);
      Ok(())
    });
  }
}
```

## Examples

### Basic Usage with macro rfs_test_macro from [rfs_test_macro](https://crates.io/crates/rfs_test_macro) crate

You can significantly simplify the unit test code by moving the FsTester configuration out of the body of the unit test and into the descriptive part of the macro that declares the test.

Instead of this:

```rust
use std::path::PathBuf;

#[test]
fn test_file_creation() {
  const YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML: &str = r#"
  - !directory
      name: test
      content:
        - !file
            name: test.txt
            content:
              !inline_text "Hello, world!"
  "#;

  let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".").expect("Incorrect configuration");
  tester.perform_fs_test(|dirname| {
    let file_path = PathBuf::from(dirname).join("test.txt");
    let content = std::fs::read_to_string(file_path)?;

    assert_eq!(content, "Hello, world!");
    Ok(())
  });
}
```

You can use the `rfc_test` macro to write more readable and clear unit tests:

```rust
use rfs_test_macro::rfs_test;
use std::path::PathBuf;

#[rfs_test(
    config = r#"
    - !directory
        name: test
        content:
          - !file
              name: test.txt
              content:
                !inline_text "Hello, world!"
    "#,
    start_point = "."
)]
fn file_creation_test(dirname: &str) -> std::io::Result<()> {
    let file_path = PathBuf::from(dirname).join("test.txt");
    let content = std::fs::read_to_string(file_path)?;

    assert_eq!(content, "Hello, world!");
    Ok(())
}
```

Or you can even set the configuration as a constant:

```rust
use std::path::PathBuf;

use rfs_test_macro::rfs_test;

const CONFIG: &str = r#"
    - !directory
        name: test
        content:
          - !file
              name: test.txt
              content:
                !inline_text "Hello, world!"
    "#;

#[rfs_test(
    config = CONFIG,
    start_point = "."
)]
fn file_creation_test_macro_with_conf_in_const(dirname: &str) -> std::io::Result<()> {
    let file_path = PathBuf::from(dirname).join("test.txt");
    let content = std::fs::read_to_string(file_path)?;

    assert_eq!(content, "Hello, world!");
    Ok(())
}
```

Add dependency in Cargo.toml before use `rfs_test` macro:

```toml
[dependencies]
rfs_test_macro = "1.1.1"
```

or

```toml
[dev-dependencies]
rfs_test_macro = "1.1.1"
```

### Using JSON Configuration

```rust
use rfs_tester::{FsTester, FileContent};
use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};

#[test]
fn test_file_creation() {
    const JSON_CONFIG: &str = r#"
    [
      {
        "directory": {
          "name": "test",
          "content": [
            {
              "file": {
                "name": "test.txt",
                "content": {
                  "inline_bytes": [116, 101, 115, 116]
                }
              }
            }
          ]
        }
      }
    ]
    "#;

    let tester = FsTester::new(JSON_CONFIG, ".").expect("Incorrect configuration");;
    tester.perform_fs_test(|dirname| {
        let file_path = std::path::PathBuf::from(dirname).join("test.txt");
        let content = std::fs::read_to_string(file_path)?;
        assert_eq!(content, "test");
        Ok(())
    });
}
```

## Contributing

Contributions are welcome! If you'd like to contribute, please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Make your changes and ensure all tests pass.
4. Submit a pull request with a detailed description of your changes.

## Licenses

This project is licensed under the MIT or Apache-2.0 License.
