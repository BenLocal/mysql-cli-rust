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
use std::sync::{Arc, Mutex};

/// Input context analysis result
#[derive(Debug, PartialEq)]
pub enum InputContext {
    /// USE command
    UseCommand,
    /// FROM clause
    FromClause,
    /// SELECT clause
    SelectClause,
    /// WHERE clause
    WhereClause,
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
            InputContext::WhereClause => {
                suggestions.extend(self.get_column_suggestions_for_query(&line_upper, &word_lower));
                suggestions.extend(self.get_condition_suggestions(&word_lower));
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
            InputContext::UseCommand => 20,   // Show more databases
            InputContext::FromClause => 15,   // Show more tables
            InputContext::SelectClause => 12, // Show more columns/functions
            _ => 10,                          // Default limit for other contexts
        };

        suggestions.truncate(limit);

        suggestions
    }

    /// Analyze input context
    fn analyze_context(&self, line: &str) -> InputContext {
        let words: Vec<&str> = line.split_whitespace().collect();
        let line_trimmed = line.trim();

        // USE command detection (more comprehensive)
        if !words.is_empty() && words[0] == "USE" {
            return InputContext::UseCommand;
        }
        // Also check for "USE " pattern at the end
        if line_trimmed.ends_with("USE") || line_trimmed.ends_with("USE ") || line.contains("USE ")
        {
            return InputContext::UseCommand;
        }

        // FROM/JOIN clause detection (more precise detection)
        if line_trimmed.ends_with("FROM")
            || line_trimmed.ends_with("FROM ")
            || line_trimmed.ends_with("JOIN")
            || line_trimmed.ends_with("JOIN ")
            || line.contains(" FROM ")
            || line.contains(" JOIN ")
        {
            return InputContext::FromClause;
        }

        // SELECT clause detection (without FROM)
        if line.contains("SELECT") && !line.contains("FROM") {
            return InputContext::SelectClause;
        }

        // WHERE/HAVING clause detection
        if line.contains("WHERE ") || line.contains("HAVING ") {
            return InputContext::WhereClause;
        }

        InputContext::General
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

    /// Get column suggestions
    #[allow(dead_code)]
    fn get_column_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let metadata = self.metadata.lock().unwrap();
        let mut suggestions = Vec::new();
        let mut count = 0;
        const MAX_COLUMNS: usize = 50; // Limit to prevent hanging

        for (table, column) in metadata.get_all_columns() {
            if count >= MAX_COLUMNS {
                break;
            }

            let relevance = self.calculate_relevance(column, word, 80);
            // Show all columns when word is empty, or when relevance is high enough
            if word.is_empty() || relevance > 50 {
                suggestions.push(Suggestion::column(column.clone(), table, relevance));
                count += 1;
            }
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
                let full_table_key = format!("{}.{}", current_db_name, table_name);
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

            // Also try all other databases
            for (table_key, columns) in &metadata.columns {
                if table_key.ends_with(&format!(".{}", table_name)) {
                    for column in columns {
                        if word.is_empty()
                            || column.to_lowercase().starts_with(&word.to_lowercase())
                        {
                            let relevance = if current_db.as_ref()
                                == Some(&table_key.split('.').next().unwrap().to_string())
                            {
                                self.calculate_relevance(column, word, 90)
                            } else {
                                self.calculate_relevance(column, word, 75)
                            };
                            suggestions.push(Suggestion::column(
                                column.clone(),
                                table_key,
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
            if words[i] == "FROM" || words[i] == "JOIN" {
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
