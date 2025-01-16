# File System Tester is a package that helps you start file system testing.

=================================================================

## Overview

This library provides a simple utility for testing file system operations.
When you are testing something, you need a sandbox that can be wiped out after the testing is finished.
This package allows you to configure a temporary directory and its internal structure, perform tests, and remove it when all the work is done.

It can be configured to generate a customized directory structure. The main idea behind this package is to write test cases for a program that needs to work with the file system, manipulating directories, files, and links to them. The tests require a template directory with several files at various levels of the file system structure.

The random generator is used to add uniqueness to the temporary directory name, which contains other test file system units.

## Configuration

The test directory structure can be configured using the YAML or JSON format.

### Yaml configuration

```yaml
---
  - directory:
      name: test
      content:
        - file:
            name: test.txt
            content:
              inline_bytes:
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

Directory configuration can specify the name and content of:

- name - string representing the directory name
- content - a list of internal file system elements (directories, files, links).

Example using the YAML:

```yaml
---
  - directory:
      name: test
      content:
        - file:
            name: test.txt
            content: empty
        - link:
            name: test_link
            target: test.txt
```

or the same using the JSON:

```json
{
  "name": "test_dir",
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

## How to define test?

When we want to test files, directories, and links in the created sandbox, we need to know the exact name of the outer directory. This name will be unique each time FsTester creates it. Fastest provides us with this name as a closure parameter in the perform_fs_test function.

Example:

```rust
#[cfg(test)]
mod tests {
  use std::fs;
  use rfs_tester::{FsTester, FileContent};
  use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};

  #[test]
  fn test_file_creation() {
    const YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML: &str = "---
    - directory:
        name: test
        content:
          - file:
              name: test_from_cargo.toml
              content:
                original_file: Cargo.toml
     ";

    let tester = FsTester::new(YAML_DIR_WITH_TEST_FILE_FROM_CARGO_TOML, ".");
    tester.perform_fs_test(|dirname| {
    //                      ^^^^^^^ name with a random number at the end
      let inner_file_name = format!("{}/{}", dirname, "test_from_cargo.toml");
      let metadata = fs::metadata(inner_file_name)?;

      assert!(metadata.len() > 0);
      Ok(())
    });
  }
}
```

## TODO

- create more test units
