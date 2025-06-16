/*!
 * MySQL CLI Rust - Smart completion module
 *
 * Provides traditional command-line completion experience, supporting:
 * - SQL keyword completion
 * - Database name, table name, field name auto-completion
 * - Context-aware smart suggestions
 * - Inline hints and history
 */

pub mod engine;
pub mod helper;
pub mod metadata;
pub mod suggestion;

// Re-export main interfaces
pub use helper::MySQLHelper;
