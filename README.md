# Ferro Spreadsheet

A high-performance spreadsheet implementation in Rust, focusing on memory efficiency and processing speed.

## Overview

Ferro Spreadsheet is a command-line spreadsheet application that supports:
- Basic arithmetic operations
- Cell references and formulas
- Range-based functions (SUM, AVG, MIN, MAX, STDEV)
- Special operations like SLEEP()
- Efficient handling of large spreadsheets
- Dependency tracking and cycle detection



## Code Documentation

Our project includes comprehensive rustdoc documentation that can be accessed in two ways:

### Generating Documentation Locally

You can generate and view the documentation locally by running:

```bash
# Generate documentation
cargo doc --no-deps --open
```

This will build the documentation and open it in your default web browser.

### Documentation Structure

Our rustdoc comments follow these principles:

1. **Module-level documentation**: Each module (.rs file) includes a detailed overview explaining its purpose, main components, and usage patterns
2. **Struct and trait documentation**: All public structs and traits have documentation explaining their purpose and usage
3. **Method documentation**: Public methods include:
   - Brief description of functionality
   - Parameter explanations
   - Return value descriptions
   - Example usage where appropriate
   - Notes on edge cases and error handling

### Example Documentation

Here's an example of our rustdoc style:

```rust
/// Represents a cell value in the spreadsheet.
///
/// Cell values can either be integers or error values.
/// This enum is used throughout the spreadsheet for storing
/// and manipulating cell contents.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// An integer value stored in a cell
    Integer(i32),
    /// Represents an error in the cell (e.g., division by zero)
    Error,
}
```

### Building and Viewing Documentation

The project uses rustdoc for comprehensive code documentation. Here's how to build and access it:

```bash
# Generate documentation without external dependencies
cargo doc --no-deps
```

After generating the documentation, you can access it in several ways depending on your environment:

#### On Windows:
```bash
# Generate and automatically open documentation in browser
cargo doc --no-deps --open
```

#### On WSL (Windows Subsystem for Linux):
If you're using WSL and encounter the "couldn't open docs" error, you can:

1. Generate the docs without the open flag:
   ```bash
   cargo doc --no-deps
   ```

2. Then manually open the HTML file using your Windows browser:
   ```bash
   # Option 1: Using the Windows path
   explorer.exe $(wslpath -w ./target/doc/spreadsheet/index.html)

   # Option 2: Copy the path and open manually
   echo "$(wslpath -w $(pwd))/target/doc/spreadsheet/index.html"
   # Then copy the output path and paste in your browser
   ```

#### On Linux:
```bash
# Make sure you have xdg-open installed
sudo apt install xdg-utils  # For Debian/Ubuntu
# Then run
cargo doc --no-deps --open
```

You can also configure rustdoc to include additional features:

```bash
# Generate documentation with all features enabled
cargo doc --all-features --no-deps --open

# Generate private items documentation (including internal functions)
cargo doc --document-private-items --no-deps --open
```

The documentation is generated in HTML format and stored in the `target/doc` directory. The index page provides an overview of all modules and types.

## Installation and Usage

### Requirements

- Rust and Cargo (install from https://rustup.rs/)

### Building the Project

```bash
# Clone the repository
git clone https://github.com/username/ferro_spreadsheet.git
cd ferro_spreadsheet

# Build the project
cargo build --release
```

### Running the Spreadsheet

```bash
# Run with desired dimensions (rows columns)
cargo run --release -- 999 18278
```

### Commands

- `A1=42` - Set cell A1 to the value 42
- `B1=A1+10` - Set B1 to A1's value plus 10
- `C1=SUM(A1:B5)` - Set C1 to the sum of the range A1:B5
- `w`, `a`, `s`, `d` - Scroll viewport
- `scroll_to A10` - Move viewport to cell A10
- `enable_output`, `disable_output` - Toggle spreadsheet display
- `q` - Quit the application
#### Vim mode/ext1 
- `h`to move left, `j` to move down ,`k`to move up ,`l` to move the cursor right
- `visual A1` - Show dependencies for cell A1
- `i` to enter insert mode
- `esc` to exit insert mode
- `:q`to quit the program 
- `:wq` to save and quit the program
- `:w` to save the program 
- `HLP (cell)`to highlight parent
- `HLC (cell)`to highlight children
- `HLPC (cell)`to highlight parent and children
- `HV (Range) Standard function` AVG,SUM,MAX,STDEV,MIN to get the range value using the function
- Pressing upper arrow goes to previous command
- Pressing down arrow goes to more recent command
#### Extension to normal spreadsheet 
- `history <cell>` to revert back to previous value of the cell
- `lock_cell <cell/range>` to disable editing value of the cell or range of cells
- `last_edit` makes the last edited cell the top left cell 
- `name <cell/range> <name>` to name a cell or range of cells and use the name later 
- `unlock_cell <cell>` to enable editing the value of disabled cell
- `is_locked <cell>` to check if the cell is locked

## Testing Approach

Our testing strategy includes:

1. **Unit Tests** - Testing individual components in isolation
2. **Integration Tests** - Testing interactions between components
3. **Continuous Integration** - Automated testing on GitHub Actions

Current test coverage is over 80%, validating our code's correctness and reliability.

## Design Justification

Our design provides several advantages:

1. **Memory Efficiency**: By using sparse data structures, we avoid allocating memory for empty cells
2. **Performance**: Topological sorting and efficient data structures minimize recalculation time
3. **Maintainability**: Clear separation of concerns makes the code easy to understand and extend
4. **Flexibility**: The design allows for easy addition of new formula types and functions
5. **Safety**: Rust's ownership model helps prevent memory leaks and data races

This design balances memory usage, performance, and code clarity, making it suitable for both small and large spreadsheets.
