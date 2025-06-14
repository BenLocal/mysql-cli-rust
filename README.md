# MySQL CLI Rust

A production-ready MySQL command-line interface written in Rust with intelligent Tab completion.

## Features

- **Smart Tab Completion**: Complete SQL keywords, database names, table names, and column names
- **Context-Aware Suggestions**: Intelligent completion based on SQL context
- **History and Hints**: Command history with inline hints
- **Traditional CLI Experience**: Clean command-line interface without GUI elements

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Connect to MySQL server
./target/release/mysql-cli-rust -u username -p -D database_name

# Connect with explicit password
./target/release/mysql-cli-rust --user username --password your_password --database test

# Connect to remote host
./target/release/mysql-cli-rust --host 192.168.1.100 --port 3306 -u username -p
```

## Tab Completion Examples

- `SEL<Tab>` → `SELECT`
- `SHOW DATAB<Tab>` → `SHOW DATABASES`
- `SELECT * FROM <Tab>` → Shows available table names
- `USE <Tab>` → Shows available database names
- `SELECT column_name FROM table_name WHERE <Tab>` → Shows column names

## Special Commands

- `\h` or `\help` - Show help
- `\q` or `\quit` - Exit the program
- `\d` or `\databases` - Show databases
- `\t` or `\tables` - Show tables
- `\u database_name` - Use database
- `\s` or `\status` - Show connection status
- `\c` or `\clear` - Clear current input

## Architecture

- **Completion Engine**: Context-aware SQL completion using rustyline
- **Database Metadata**: Automatic loading of database schema for intelligent suggestions
- **Command Processing**: Efficient SQL query execution with result formatting
- **Connection Management**: Robust MySQL connection handling

Built with Rust for performance, reliability, and memory safety.
