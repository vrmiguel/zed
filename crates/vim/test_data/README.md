# Vim Test Data Files

## Purpose

These JSON files contain recorded sequences of Vim operations and expected editor states used in Zed's Vim mode tests. They allow testing the Vim implementation without requiring a real Neovim instance.

## File Format

Each test file consists of a series of JSON objects, one per line, that represent:

- **Put**: Sets up an initial editor state with cursor position marked by `ˇ`
- **Key**: Simulates pressing a key
- **Get**: Expected editor state after keys are pressed, including the mode
- **ReadRegister**: Reads the content of a named register
- **Exec**: Executes a Vim command
- **SetOption**: Sets a Vim option

Example:
```json
{"Put":{"state":"The quick\nbrownˇ fox\njumps over\nthe lazy"}}
{"Key":"d"}
{"Key":"shift-g"}
{"Get":{"state":"The qˇuick","mode":"Normal"}}
```

## How Tests Work

1. When tests run with the "neovim" feature enabled:
   - A real Neovim instance is started
   - The test operations are performed against it
   - The results are recorded to these JSON files

2. When tests run without the "neovim" feature:
   - These files are read and used to validate that Zed's Vim implementation 
     produces the same results as Neovim did

## Test Naming Convention

- `test_[operation]_[target].json`: Tests for operations (like delete, change, yank) with targets (like end of document, start of document)
- Example: `test_delete_gg.json` tests the "delete to start of document" operation

## Regenerating Test Files

To regenerate or create new test files, run the tests with the "neovim" feature enabled:

```
cargo test --package vim --features neovim
```

This will create or update the test JSON files in this directory.