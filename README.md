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

## Design and Software Architecture

### Core Design Philosophy

Our design focuses on memory efficiency and performance by:
1. Storing only non-default cell values
2. Using efficient data structures to represent dependencies
3. Employing topological sorting for formula re-evaluation
4. Optimizing memory allocations with boxed collections

### Architecture

The application is built around these core components:

- **Spreadsheet Structure**: Central data container with optimized storage
- **Cell Management**: Handles individual cell operations and storage
- **Formula Evaluation**: Processes formulas and dependencies
- **Graph Management**: Tracks dependencies between cells
- **Command Processing**: Interprets and executes user commands
- **Visualization**: Displays the spreadsheet and relationships

### Approaches for Encapsulation

We implement encapsulation through Rust's module system:
1. The `Spreadsheet` struct exposes only necessary methods publicly
2. Internal implementation details are hidden from the public API
3. Each module has a clear responsibility and hides its implementation

## Primary Data Structures

### Spreadsheet

The core `Spreadsheet` struct uses several optimized data structures:

```rust
pub struct Spreadsheet {
    pub grid: Vec<CellValue>,                                // Vector of CellValues (contiguous in memory)
    pub children: HashMap<i32, Box<HashSet<i32>>>,           // Map from cell key to boxed HashSet of children
    pub range_children: Vec<RangeChild>,                     // Vector of range-based child relationships
    pub cell_meta: HashMap<i32, CellMeta>,                   // Map from cell key to metadata
    // ...other fields...
}
```

Key design choices:

1. **On-demand allocation**: The `children` field uses a HashMap that only allocates HashSets for cells with dependencies, saving memory
2. **Boxed HashSets**: Each children HashSet is boxed to allow for different sizes without affecting locality
3. **Range-based optimizations**: The `range_children` structure optimizes range-based formulas by storing ranges instead of individual cells
4. **Sparse metadata**: The `cell_meta` HashMap only stores metadata for cells with formulas, not empty cells

### Range Child

For efficient range-based formula handling:

```rust
pub struct RangeChild {
    pub start_key: i32,       // Range start cell key
    pub end_key: i32,         // Range end cell key
    pub child_key: i32,       // Child cell key
}
```

This structure reduces memory usage by not creating individual dependencies for each cell in a range.

### CellMeta

Stores formula information while keeping the grid simple:

```rust
pub struct CellMeta {
    pub formula: i16,
    pub parent1: i32,
    pub parent2: i32,
}
```

## Interfaces Between Software Modules

The application has clean interfaces between its major components:

### Spreadsheet ⟷ Evaluator

- Evaluator uses Spreadsheet's public API to read and modify cells
- Spreadsheet provides methods like `get_cell`, `get_mut_cell`, and `get_cell_meta`

### Evaluator ⟷ Graph

- Evaluator calls Graph functions to handle adding and removing cell dependencies
- Graph updates the dependency structures in the Spreadsheet

### Graph ⟷ Reevaluate Topological Sort

- The Graph module provides functions for cycle detection
- The Reevaluate module performs topological sorting to determine evaluation order

## Extensions

### Implemented Extensions

1. **Memory Optimization**
   - Boxed HashSets for variable-sized collections
   - HashMap-based sparse storage for cell children
   - Range-based dependency tracking

2. **Efficient Formula Evaluation**
   - Topological sorting for efficient re-evaluation
   - Range-based function optimizations

3. **Visualization**
   - Cell dependency visualization
   - Textual representation of relationships

### Extensions That Couldn't Be Implemented

We initially planned to implement thread-based parallel formula evaluation, but we faced challenges with Rust's ownership model when sharing the spreadsheet across threads. The borrowing rules made it difficult to update cells concurrently while maintaining proper dependencies.

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
- `visual A1` - Show dependencies for cell A1
- `enable_output`, `disable_output` - Toggle spreadsheet display
- `q` - Quit the application

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
