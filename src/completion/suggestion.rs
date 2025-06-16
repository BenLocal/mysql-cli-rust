/*!
 * Smart suggestion data structures
 *
 * Defines data structures and classification system for completion suggestions
 */

/// Smart suggestion item
#[derive(Clone, Debug)]
pub struct Suggestion {
    /// Completion text
    pub text: String,
    /// Description text
    pub description: String,
    /// Suggestion category
    pub category: SuggestionCategory,
    /// Relevance score (0-100)
    pub relevance: u8,
}

/// Suggestion category enum
#[derive(Clone, Debug, PartialEq)]
pub enum SuggestionCategory {
    /// Database
    Database,
    /// Table
    Table,
    /// Column/Field
    Column,
    /// SQL Keyword
    SqlKeyword,
    /// Function
    Function,
    /// Command
    Command,
}

impl Suggestion {
    /// Create a new suggestion item
    pub fn new(
        text: String,
        description: String,
        category: SuggestionCategory,
        relevance: u8,
    ) -> Self {
        Self {
            text,
            description,
            category,
            relevance: relevance.min(100), // Ensure not exceeding 100
        }
    }

    /// Create database suggestion
    pub fn database(name: String, relevance: u8) -> Self {
        Self::new(
            format!("`{}`", name),
            format!("Database: {}", name),
            SuggestionCategory::Database,
            relevance,
        )
    }

    /// Create table suggestion
    pub fn table(name: String, database: &str, relevance: u8) -> Self {
        Self::new(
            format!("`{}`", name),
            format!("Table: {} (in {} database)", name, database),
            SuggestionCategory::Table,
            relevance,
        )
    }

    /// Create column suggestion
    pub fn column(name: String, table: &str, relevance: u8) -> Self {
        Self::new(
            format!("`{}`", name),
            format!("Column: {} (from table {})", name, table),
            SuggestionCategory::Column,
            relevance,
        )
    }

    /// Create SQL keyword suggestion
    pub fn sql_keyword(keyword: String, description: String, relevance: u8) -> Self {
        Self::new(
            keyword,
            description,
            SuggestionCategory::SqlKeyword,
            relevance,
        )
    }

    /// Create function suggestion
    pub fn function(name: String, description: String, relevance: u8) -> Self {
        Self::new(name, description, SuggestionCategory::Function, relevance)
    }

    /// Create command suggestion
    pub fn command(command: String, description: String, relevance: u8) -> Self {
        Self::new(command, description, SuggestionCategory::Command, relevance)
    }
}

impl SuggestionCategory {
    /// Get emoji icon for category
    pub fn icon(&self) -> &'static str {
        match self {
            SuggestionCategory::Database => "🗄️",
            SuggestionCategory::Table => "📊",
            SuggestionCategory::Column => "📋",
            SuggestionCategory::SqlKeyword => "🔵",
            SuggestionCategory::Function => "⚡",
            SuggestionCategory::Command => "⚙️",
        }
    }
}
