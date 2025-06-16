/*!
 * Smart suggestion engine
 *
 * Core completion logic responsible for:
 * - Analyzing input context
 * - Generating relevant suggestions
 * - Sorting by relevance
 */

use super::metadata::DatabaseMetadata;
use super::suggestion::Suggestion;
use sqlparser::ast::{Query, SetExpr, Statement};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use std::sync::{Arc, Mutex};

/// Input context analysis result
#[derive(Debug, PartialEq)]
pub enum InputContext {
    /// USE command
    UseCommand,
    /// FROM clause (table names expected)
    FromClause,
    /// SELECT clause (column names, functions expected)
    SelectClause,
    /// WHERE clause (column names, operators, values expected)
    WhereClause,
    /// INSERT INTO clause (table name expected)
    InsertIntoClause,
    /// UPDATE clause (table name expected)
    UpdateClause,
    /// ORDER BY clause (column names expected)
    OrderByClause,
    /// GROUP BY clause (column names expected)
    GroupByClause,
    /// HAVING clause (column names, functions expected)
    HavingClause,
    /// JOIN ON clause (column names for join conditions)
    JoinOnClause,
    /// General case
    General,
}

/// Smart suggestion engine
pub struct SmartSuggestionEngine {
    metadata: Arc<Mutex<DatabaseMetadata>>,
    sql_keywords: Vec<String>,
    current_database: Arc<Mutex<Option<String>>>,
}

impl SmartSuggestionEngine {
    /// Create new suggestion engine
    pub fn new(metadata: Arc<Mutex<DatabaseMetadata>>, sql_keywords: Vec<String>) -> Self {
        Self {
            metadata,
            sql_keywords,
            current_database: Arc::new(Mutex::new(None)),
        }
    }

    /// Update current database
    pub fn set_current_database(&self, database: Option<String>) {
        if let Ok(mut current_db) = self.current_database.lock() {
            *current_db = database;
        }
    }

    /// Get smart suggestion list
    pub fn get_suggestions(&self, line: &str, word: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let line_upper = line.to_uppercase();
        let word_lower = word.to_lowercase();

        // Analyze current input context
        let context = self.analyze_context(&line_upper);

        // Generate suggestions based on context
        match context {
            InputContext::UseCommand => {
                suggestions.extend(self.get_database_suggestions(&word_lower));
                // If no databases found and word is empty, still provide some indication
                if suggestions.is_empty() && word.is_empty() {
                    // Add a placeholder suggestion to indicate no databases available
                    suggestions.push(Suggestion::command(
                        "-- No databases available --".to_string(),
                        "Connect to a MySQL server with databases".to_string(),
                        50,
                    ));
                }
            }
            InputContext::FromClause => {
                suggestions.extend(self.get_table_suggestions(&word_lower));
                // If no tables found and word is empty, still provide some indication
                if suggestions.is_empty() && word.is_empty() {
                    // Add a placeholder suggestion to indicate no tables available
                    suggestions.push(Suggestion::command(
                        "-- No tables available --".to_string(),
                        "Connect to a database with tables".to_string(),
                        50,
                    ));
                }
            }
            InputContext::SelectClause => {
                // For SELECT without FROM, limit suggestions and prefer SQL functions/keywords
                if !line_upper.contains("FROM") {
                    // If it's just "SELECT" or "SELECT ", provide basic suggestions
                    if line_upper.trim() == "SELECT" || line_upper.trim() == "SELECT " {
                        suggestions.extend(self.get_sql_keyword_suggestions(&word_lower));
                        // Add common SELECT suggestions
                        if word_lower.is_empty() {
                            suggestions.push(Suggestion::command(
                                "*".to_string(),
                                "Select all columns".to_string(),
                                95,
                            ));
                            suggestions.push(Suggestion::command(
                                "COUNT(*)".to_string(),
                                "Count all rows".to_string(),
                                90,
                            ));
                        }
                    } else {
                        // Prioritize SQL functions and common keywords
                        suggestions.extend(self.get_function_suggestions(&word_lower));
                        suggestions.extend(self.get_sql_keyword_suggestions(&word_lower));

                        // Only add limited column suggestions if word is not empty (user is typing something specific)
                        if !word_lower.is_empty() && word_lower.len() >= 2 {
                            let mut limited_columns =
                                self.get_limited_column_suggestions(&word_lower, 10);
                            suggestions.append(&mut limited_columns);
                        }
                    }
                } else {
                    // When FROM clause exists, use full context-aware suggestions
                    suggestions
                        .extend(self.get_column_suggestions_for_query(&line_upper, &word_lower));
                    suggestions.extend(self.get_function_suggestions(&word_lower));
                }
            }
            InputContext::WhereClause | InputContext::HavingClause | InputContext::JoinOnClause => {
                suggestions.extend(self.get_column_suggestions_for_query(&line_upper, &word_lower));
                suggestions.extend(self.get_condition_suggestions(&word_lower));
            }
            InputContext::OrderByClause | InputContext::GroupByClause => {
                suggestions.extend(self.get_column_suggestions_for_query(&line_upper, &word_lower));
            }
            InputContext::InsertIntoClause | InputContext::UpdateClause => {
                suggestions.extend(self.get_table_suggestions(&word_lower));
            }
            InputContext::General => {
                suggestions.extend(self.get_sql_keyword_suggestions(&word_lower));
                if word.is_empty() {
                    suggestions.extend(self.get_common_command_suggestions());
                }
            }
        }

        // Sort by relevance and limit quantity based on context
        suggestions.sort_by(|a, b| b.relevance.cmp(&a.relevance));

        // Use different limits based on context
        let limit = match context {
            InputContext::UseCommand => 20, // Show more databases
            InputContext::FromClause
            | InputContext::InsertIntoClause
            | InputContext::UpdateClause => 15, // Show more tables
            InputContext::SelectClause => 12, // Show more columns/functions
            InputContext::WhereClause
            | InputContext::HavingClause
            | InputContext::JoinOnClause
            | InputContext::OrderByClause
            | InputContext::GroupByClause => 15, // Show more columns for filtering/sorting
            InputContext::General => 10,    // Default limit for other contexts
        };

        suggestions.truncate(limit);

        suggestions
    }

    /// Analyze input context using SQL parser for better accuracy
    fn analyze_context(&self, line: &str) -> InputContext {
        let line_trimmed = line.trim();

        // Handle empty input
        if line_trimmed.is_empty() {
            return InputContext::General;
        }

        // Quick check for specific commands first
        let words: Vec<&str> = line_trimmed.split_whitespace().collect();
        if let Some(first_word) = words.first() {
            match first_word.to_uppercase().as_str() {
                "USE" => return InputContext::UseCommand,
                _ => {}
            }
        }

        // Try to parse the SQL to determine context more accurately
        if let Ok(context) = self.analyze_sql_context(line_trimmed) {
            return context;
        }

        // Fallback to simple text-based analysis for incomplete queries
        self.analyze_context_fallback(line_trimmed)
    }

    /// Analyze SQL context using sqlparser
    fn analyze_sql_context(&self, sql: &str) -> Result<InputContext, Box<dyn std::error::Error>> {
        let dialect = MySqlDialect {};

        // Try to parse as a complete statement first
        match Parser::parse_sql(&dialect, sql) {
            Ok(statements) => {
                if let Some(stmt) = statements.first() {
                    return Ok(self.determine_context_from_statement(stmt));
                }
            }
            Err(_) => {
                // If complete parsing fails, try to analyze incomplete queries
                return self.analyze_incomplete_sql(sql);
            }
        }

        Err("Could not determine context".into())
    }

    /// Determine context from a parsed SQL statement
    fn determine_context_from_statement(&self, stmt: &Statement) -> InputContext {
        match stmt {
            Statement::Query(query) => self.analyze_query_context(query),
            Statement::Insert { .. } => InputContext::InsertIntoClause,
            Statement::Update { .. } => InputContext::UpdateClause,
            Statement::Use { .. } => InputContext::UseCommand,
            _ => InputContext::General,
        }
    }

    /// Analyze query context (SELECT, FROM, WHERE, etc.)
    fn analyze_query_context(&self, query: &Query) -> InputContext {
        if let SetExpr::Select(select) = &*query.body {
            // Check for different clauses in the SELECT statement
            if !select.from.is_empty() {
                if select.selection.is_some() {
                    InputContext::WhereClause
                } else {
                    InputContext::FromClause
                }
            } else {
                InputContext::SelectClause
            }
        } else {
            InputContext::General
        }
    }

    /// Analyze incomplete SQL that couldn't be fully parsed
    fn analyze_incomplete_sql(
        &self,
        sql: &str,
    ) -> Result<InputContext, Box<dyn std::error::Error>> {
        let sql_upper = sql.to_uppercase();

        // Look for keyword patterns to determine context
        if sql_upper.ends_with("WHERE") {
            return Ok(InputContext::WhereClause);
        }

        if sql_upper.ends_with("FROM") {
            return Ok(InputContext::FromClause);
        }

        if sql_upper.ends_with("JOIN") {
            return Ok(InputContext::FromClause);
        }

        if sql_upper.ends_with("ON") {
            return Ok(InputContext::JoinOnClause);
        }

        if sql_upper.ends_with("ORDER BY") {
            return Ok(InputContext::OrderByClause);
        }

        if sql_upper.ends_with("GROUP BY") {
            return Ok(InputContext::GroupByClause);
        }

        if sql_upper.ends_with("HAVING") {
            return Ok(InputContext::HavingClause);
        }

        if sql_upper.contains("WHERE ") {
            return Ok(InputContext::WhereClause);
        }

        if sql_upper.contains("FROM ") {
            return Ok(InputContext::FromClause);
        }

        if sql_upper.contains("JOIN ") {
            return Ok(InputContext::FromClause);
        }

        if sql_upper.contains(" ON ") {
            return Ok(InputContext::JoinOnClause);
        }

        if sql_upper.contains("ORDER BY ") {
            return Ok(InputContext::OrderByClause);
        }

        if sql_upper.contains("GROUP BY ") {
            return Ok(InputContext::GroupByClause);
        }

        if sql_upper.contains("HAVING ") {
            return Ok(InputContext::HavingClause);
        }

        if sql_upper.starts_with("SELECT") {
            return Ok(InputContext::SelectClause);
        }

        if sql_upper.starts_with("INSERT INTO") {
            return Ok(InputContext::InsertIntoClause);
        }

        if sql_upper.starts_with("UPDATE") {
            return Ok(InputContext::UpdateClause);
        }

        Err("Could not determine incomplete SQL context".into())
    }

    /// Fallback context analysis when SQL parsing fails
    fn analyze_context_fallback(&self, line: &str) -> InputContext {
        let words: Vec<&str> = line.split_whitespace().map(|s| s.trim()).collect();
        if words.is_empty() {
            return InputContext::General;
        }

        // USE command detection
        if words[0].to_uppercase() == "USE" {
            return InputContext::UseCommand;
        }

        // Look for keywords in any position
        for &word in &words {
            match word.to_uppercase().as_str() {
                "WHERE" => return InputContext::WhereClause,
                "FROM" | "JOIN" => return InputContext::FromClause,
                "ORDER" if words.len() > 1 && words[1].to_uppercase() == "BY" => {
                    return InputContext::OrderByClause;
                }
                "GROUP" if words.len() > 1 && words[1].to_uppercase() == "BY" => {
                    return InputContext::GroupByClause;
                }
                "HAVING" => return InputContext::HavingClause,
                _ => {}
            }
        }

        // Check first word for statement type
        match words[0].to_uppercase().as_str() {
            "SELECT" => InputContext::SelectClause,
            "INSERT" => InputContext::InsertIntoClause,
            "UPDATE" => InputContext::UpdateClause,
            _ => InputContext::General,
        }
    }

    /// Calculate matching relevance
    fn calculate_relevance(&self, item: &str, word: &str, base_score: u8) -> u8 {
        if word.is_empty() {
            return base_score;
        }

        let item_lower = item.to_lowercase();
        let word_lower = word.to_lowercase();

        if item_lower == word_lower {
            100 // Exact match
        } else if item_lower.starts_with(&word_lower) {
            // Give much higher score for prefix matches to prioritize them
            95 // Prefix match gets very high priority
        } else if item_lower.contains(&word_lower) {
            (base_score + 5).min(85) // Contains match
        } else {
            base_score.saturating_sub(10) // Lower score for non-matching items
        }
    }

    /// Get database suggestions
    fn get_database_suggestions(&self, word: &str) -> Vec<Suggestion> {
        // Try to lock metadata with timeout to avoid hanging
        let metadata = match self.metadata.try_lock() {
            Ok(metadata) => metadata,
            Err(_) => {
                // If metadata is locked (potentially loading), return empty suggestions
                return Vec::new();
            }
        };

        let mut suggestions = Vec::new();

        for db in metadata.get_databases() {
            if word.is_empty() {
                // Show all databases when no input
                let relevance = self.calculate_relevance(db, word, 90);
                suggestions.push(Suggestion::database(db.clone(), relevance));
            } else {
                // When user has typed something, only show databases that start with the input
                let db_lower = db.to_lowercase();
                let word_lower = word.to_lowercase();

                if db_lower.starts_with(&word_lower) {
                    let relevance = self.calculate_relevance(db, word, 90);
                    suggestions.push(Suggestion::database(db.clone(), relevance));
                }
            }
        }

        suggestions
    }

    /// Get table suggestions
    fn get_table_suggestions(&self, word: &str) -> Vec<Suggestion> {
        // Try to lock metadata with timeout to avoid hanging
        let metadata = match self.metadata.try_lock() {
            Ok(metadata) => metadata,
            Err(_) => {
                // If metadata is locked (potentially loading), return empty suggestions
                return Vec::new();
            }
        };

        let current_db = self.current_database.lock().unwrap();
        let mut suggestions = Vec::new();

        // Separate current database tables and other tables
        let mut current_db_tables = Vec::new();
        let mut other_tables = Vec::new();

        for (db, table) in metadata.get_all_tables() {
            if word.is_empty() {
                // When no input, show all tables with current database first
                let relevance = self.calculate_relevance(table, word, 85);
                let suggestion = Suggestion::table(table.clone(), db, relevance);

                if current_db.as_ref() == Some(db) {
                    current_db_tables.push(suggestion);
                } else {
                    other_tables.push(suggestion);
                }
            } else {
                // When user has typed something, only show tables that start with the input
                let table_lower = table.to_lowercase();
                let word_lower = word.to_lowercase();

                if table_lower.starts_with(&word_lower) {
                    let relevance = if current_db.as_ref() == Some(db) {
                        95 // Higher relevance for current database tables
                    } else {
                        self.calculate_relevance(table, word, 85)
                    };
                    suggestions.push(Suggestion::table(table.clone(), db, relevance));
                }
            }
        }

        // When no input, add current database tables first, then others
        if word.is_empty() {
            suggestions.extend(current_db_tables);
            suggestions.extend(other_tables);
        }

        suggestions
    }

    /// Get column suggestions for a specific query context
    fn get_column_suggestions_for_query(&self, query: &str, word: &str) -> Vec<Suggestion> {
        // Try to lock metadata with timeout to avoid hanging
        let metadata = match self.metadata.try_lock() {
            Ok(metadata) => metadata,
            Err(_) => {
                // If metadata is locked (potentially loading), return empty suggestions
                return Vec::new();
            }
        };

        let current_db = self.current_database.lock().unwrap();
        let mut suggestions = Vec::new();

        // Extract table names from the query
        let table_names = self.extract_table_names_from_query(query);

        if table_names.is_empty() {
            // Fallback to limited columns if no tables found
            drop(metadata); // Release lock before calling other method
            return self.get_limited_column_suggestions(word, 20);
        }

        // Get columns from the identified tables
        for table_name in &table_names {
            // First try with current database
            if let Some(current_db_name) = current_db.as_ref() {
                let full_table_key = format!("{}.{}", current_db_name, table_name).to_lowercase();
                if let Some(columns) = metadata.columns.get(&full_table_key) {
                    for column in columns {
                        if word.is_empty()
                            || column.to_lowercase().starts_with(&word.to_lowercase())
                        {
                            let relevance = self.calculate_relevance(column, word, 90);
                            suggestions.push(Suggestion::column(
                                column.clone(),
                                &full_table_key,
                                relevance,
                            ));
                        }
                    }
                }
            }
        }

        suggestions
    }

    /// Extract table names from SQL query
    fn extract_table_names_from_query(&self, query: &str) -> Vec<String> {
        let mut table_names = Vec::new();
        let words: Vec<&str> = query.split_whitespace().collect();

        // Look for FROM and JOIN clauses
        for i in 0..words.len() {
            let word_upper = words[i].to_uppercase();
            if word_upper == "FROM" || word_upper == "JOIN" {
                // The next word should be a table name
                if i + 1 < words.len() {
                    let table_name = words[i + 1]
                        .trim_matches('`')
                        .trim_matches(',')
                        .trim_matches(';');
                    // Skip SQL keywords
                    if !self.is_sql_keyword(table_name) {
                        table_names.push(table_name.to_string());
                    }
                }
            }
        }

        table_names
    }

    /// Check if a word is a SQL keyword
    fn is_sql_keyword(&self, word: &str) -> bool {
        let word_upper = word.to_uppercase();
        self.sql_keywords.contains(&word_upper)
    }

    /// Get function suggestions
    fn get_function_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let functions = [
            ("COUNT", "Count rows"),
            ("SUM", "Sum values"),
            ("AVG", "Average value"),
            ("MAX", "Maximum value"),
            ("MIN", "Minimum value"),
            ("NOW", "Current time"),
            ("CONCAT", "String concatenation"),
            ("UPPER", "Convert to uppercase"),
            ("LOWER", "Convert to lowercase"),
            ("SUBSTRING", "String substring"),
            ("LENGTH", "String length"),
            ("TRIM", "Remove spaces"),
            ("DATE", "Date function"),
            ("YEAR", "Get year"),
            ("MONTH", "Get month"),
            ("DAY", "Get day"),
        ];

        let mut suggestions = Vec::new();
        for (func, desc) in &functions {
            let relevance = self.calculate_relevance(func, word, 75);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::function(
                    func.to_string(),
                    desc.to_string(),
                    relevance,
                ));
            }
        }

        suggestions
    }

    /// Get condition keyword suggestions
    fn get_condition_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let conditions = [
            ("AND", "Logical AND"),
            ("OR", "Logical OR"),
            ("NOT", "Logical NOT"),
            ("IN", "Contains in list"),
            ("LIKE", "Pattern matching"),
            ("BETWEEN", "Range condition"),
            ("IS NULL", "Is null value"),
            ("IS NOT NULL", "Is not null value"),
            ("EXISTS", "Exists subquery"),
            ("REGEXP", "Regular expression match"),
        ];

        let mut suggestions = Vec::new();
        for (cond, desc) in &conditions {
            let relevance = self.calculate_relevance(cond, word, 70);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::sql_keyword(
                    cond.to_string(),
                    desc.to_string(),
                    relevance,
                ));
            }
        }

        suggestions
    }

    /// Get SQL keyword suggestions
    fn get_sql_keyword_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for keyword in &self.sql_keywords {
            let relevance = self.calculate_relevance(keyword, word, 65);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::sql_keyword(
                    keyword.clone(),
                    format!("SQL keyword: {}", keyword),
                    relevance,
                ));
            }
        }

        suggestions
    }

    /// Get common command suggestions
    fn get_common_command_suggestions(&self) -> Vec<Suggestion> {
        vec![
            Suggestion::command(
                "SELECT * FROM".to_string(),
                "Query all data from table".to_string(),
                95,
            ),
            Suggestion::command(
                "SHOW DATABASES".to_string(),
                "Show all databases".to_string(),
                90,
            ),
            Suggestion::command(
                "SHOW TABLES".to_string(),
                "Show all tables in current database".to_string(),
                85,
            ),
            Suggestion::command(
                "USE".to_string(),
                "Switch to specified database".to_string(),
                80,
            ),
            Suggestion::command(
                "DESCRIBE".to_string(),
                "View table structure".to_string(),
                75,
            ),
            Suggestion::command("INSERT INTO".to_string(), "Insert data".to_string(), 70),
            Suggestion::command("UPDATE".to_string(), "Update data".to_string(), 65),
            Suggestion::command("DELETE FROM".to_string(), "Delete data".to_string(), 60),
        ]
    }

    /// Get limited column suggestions (to prevent hanging with many columns)
    fn get_limited_column_suggestions(&self, word: &str, limit: usize) -> Vec<Suggestion> {
        // Try to lock metadata with timeout to avoid hanging
        let metadata = match self.metadata.try_lock() {
            Ok(metadata) => metadata,
            Err(_) => {
                // If metadata is locked (potentially loading), return empty suggestions
                return Vec::new();
            }
        };

        let mut suggestions = Vec::new();
        let mut count = 0;

        // Only suggest columns that match the typed word to reduce noise
        for (table, column) in metadata.get_all_columns() {
            if count >= limit {
                break;
            }

            let relevance = self.calculate_relevance(column, word, 80);
            // Only include columns with good relevance (starts with or contains typed text)
            if relevance > 70 {
                suggestions.push(Suggestion::column(column.clone(), table, relevance));
                count += 1;
            }
        }

        suggestions
    }
}

#[cfg(test)]
#[path = "./engine_tests.rs"]
mod engine_tests;
