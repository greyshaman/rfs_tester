# File System Tester - package to help start file system test units

=================================================================

## Overview

This library provides simple util for testing file system operations.
When you test something you need some sandbox which should wipe out after testing finish.
This package allow you configure temporary directory and its inner structure, perform tests and
remove it when all work will done.

It can be configured to create cutomized directory structure.
The main idea to create this package is write test units for progaram which
should work whith file system, manipulate with directories, files and links to them.
Io tests need template directory with some files on different depth of fs structure.

The random generator is using for add uniqueness for temporary directory name which
contains other tested file system units.

## Configuration

The test directory structure can be configured by yaml or json format.

### Yaml configuration

```yaml
---
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
```

It will produce directory with name **test_726537253725** and file named **test.txt** in this directory with content "test".
Number in directory name can be differe because this is random number.

### Json configuration

The same directory structure can be configured by json format
#### Examples

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
                      "inline": [116,101,115,116]
                    }
                }
            }
          ]
      }
  }
]
```

#### Directory configuration

Directory structure can contains many nested directories. **Important** The first level of configuration should start from
single directory. This containing directory will be sand box container with name with appended random number in its name.
Other inner units: directories, files and links will not change their names and can be used in tests as it is in configuration

Directory configuration can specify name and content:

- name -  string represents directory name
- content - list(array) of inner file system units (directories, files, links)

##### Examples

yaml: 

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

or the same in json:

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

Project in progress.... will be continue.

## TODO

+rework it using macro to allow srart testing
+create more test units
