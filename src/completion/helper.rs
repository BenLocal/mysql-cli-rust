/*!
 * MySQL completion helper
 *
 * Main interface integrating all completion functionality, implementing various rustyline traits
 */

use super::engine::SmartSuggestionEngine;
use super::metadata::DatabaseMetadata;
use anyhow::Result;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::Context;
use std::sync::{Arc, Mutex};

/// MySQL Completer
pub struct MySQLCompleter {
    sql_keywords: Vec<String>,
    suggestion_engine: SmartSuggestionEngine,
}

impl MySQLCompleter {
    /// Create completer with shared metadata
    pub fn with_metadata(metadata: Arc<Mutex<DatabaseMetadata>>) -> Self {
        let sql_keywords = Self::init_sql_keywords();
        let suggestion_engine = SmartSuggestionEngine::new(metadata.clone(), sql_keywords.clone());

        Self {
            sql_keywords,
            suggestion_engine,
        }
    }

    /// Initialize SQL keywords list
    fn init_sql_keywords() -> Vec<String> {
        let keywords = [
            // Basic SQL keywords
            "SELECT",
            "FROM",
            "WHERE",
            "INSERT",
            "UPDATE",
            "DELETE",
            "CREATE",
            "DROP",
            "ALTER",
            "TABLE",
            "DATABASE",
            "INDEX",
            "VIEW",
            "TRIGGER",
            "PROCEDURE",
            "FUNCTION",
            // 数据类型
            "INT",
            "INTEGER",
            "BIGINT",
            "SMALLINT",
            "TINYINT",
            "DECIMAL",
            "NUMERIC",
            "FLOAT",
            "DOUBLE",
            "VARCHAR",
            "CHAR",
            "TEXT",
            "LONGTEXT",
            "MEDIUMTEXT",
            "TINYTEXT",
            "DATE",
            "TIME",
            "DATETIME",
            "TIMESTAMP",
            "YEAR",
            "BINARY",
            "VARBINARY",
            "BLOB",
            "LONGBLOB",
            "MEDIUMBLOB",
            "TINYBLOB",
            "JSON",
            "GEOMETRY",
            // 约束和修饰符
            "PRIMARY",
            "KEY",
            "FOREIGN",
            "REFERENCES",
            "UNIQUE",
            "NOT",
            "NULL",
            "DEFAULT",
            "AUTO_INCREMENT",
            "UNSIGNED",
            "ZEROFILL",
            // 查询相关
            "DISTINCT",
            "ALL",
            "AS",
            "JOIN",
            "INNER",
            "LEFT",
            "RIGHT",
            "FULL",
            "OUTER",
            "CROSS",
            "ON",
            "USING",
            "UNION",
            "INTERSECT",
            "EXCEPT",
            "ORDER",
            "BY",
            "GROUP",
            "HAVING",
            "LIMIT",
            "OFFSET",
            "INTO",
            "VALUES",
            "SET",
            // Conditions and operators
            "AND",
            "OR",
            "NOT",
            "IN",
            "EXISTS",
            "BETWEEN",
            "LIKE",
            "REGEXP",
            "RLIKE",
            "IS",
            "ISNULL",
            "CASE",
            "WHEN",
            "THEN",
            "ELSE",
            "END",
            // Aggregate functions
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "GROUP_CONCAT",
            // String functions
            "CONCAT",
            "SUBSTRING",
            "LENGTH",
            "CHAR_LENGTH",
            "UPPER",
            "LOWER",
            "TRIM",
            "LTRIM",
            "RTRIM",
            "REPLACE",
            "REVERSE",
            // 数学函数
            "ABS",
            "CEIL",
            "CEILING",
            "FLOOR",
            "ROUND",
            "MOD",
            "POW",
            "POWER",
            "SQRT",
            "RAND",
            "SIGN",
            "PI",
            "DEGREES",
            "RADIANS",
            "SIN",
            "COS",
            "TAN",
            // 日期时间函数
            "NOW",
            "CURDATE",
            "CURTIME",
            "YEAR",
            "MONTH",
            "DAY",
            "HOUR",
            "MINUTE",
            "SECOND",
            "DAYOFWEEK",
            "DAYOFYEAR",
            "WEEKDAY",
            "DATE_ADD",
            "DATE_SUB",
            "DATEDIFF",
            "DATE_FORMAT",
            "STR_TO_DATE",
            // 控制流函数
            "IF",
            "IFNULL",
            "NULLIF",
            "COALESCE",
            // 管理命令
            "SHOW",
            "DESCRIBE",
            "DESC",
            "EXPLAIN",
            "USE",
            "GRANT",
            "REVOKE",
            "FLUSH",
            "RESET",
            "START",
            "STOP",
            "RESTART",
            // Transaction control
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "SAVEPOINT",
            "RELEASE",
            "TRANSACTION",
            "READ",
            "WRITE",
            "ONLY",
            // Others
            "LOCK",
            "UNLOCK",
            "TABLES",
            "ENGINE",
            "CHARSET",
            "COLLATE",
            "TEMPORARY",
            "CASCADE",
            "RESTRICT",
        ];

        keywords.iter().map(|s| s.to_string()).collect()
    }

    /// Get current word start position
    fn get_word_start(&self, line: &str, pos: usize) -> usize {
        line[..pos]
            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',' || c == '.' || c == ';')
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    /// Update current database for better context-aware suggestions
    pub fn set_current_database(&self, database: Option<String>) {
        self.suggestion_engine.set_current_database(database);
    }
}

impl Completer for MySQLCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let start = self.get_word_start(line, pos);
        let word = &line[start..pos];

        // Use smart suggestion engine to get suggestions
        let suggestions = self.suggestion_engine.get_suggestions(line, word);

        let mut completions = Vec::new();

        // Convert smart suggestions to rustyline Pair format
        for suggestion in suggestions {
            // Extract clean text for replacement (remove backticks)
            let clean_text = suggestion.text.trim_matches('`').to_string();

            completions.push(Pair {
                display: format!(
                    "{} {} - {}",
                    suggestion.category.icon(),
                    clean_text,
                    suggestion.description
                ),
                replacement: clean_text,
            });
        }

        // If no smart suggestions, check if we're in a specific context where we shouldn't show SQL keywords
        if completions.is_empty() {
            let line_upper = line.to_uppercase();
            let should_show_keywords = !line_upper.ends_with("FROM ")
                && !line_upper.ends_with("JOIN ")
                && !line_upper.ends_with("USE ");

            if should_show_keywords {
                let word_lower = word.to_lowercase();
                for keyword in &self.sql_keywords {
                    if keyword.to_lowercase().starts_with(&word_lower) {
                        completions.push(Pair {
                            display: format!("🔵 {} - SQL keyword", keyword),
                            replacement: keyword.clone(),
                        });
                    }
                }
            }
        }

        // Limit result count based on context
        let line_upper = line.to_uppercase();
        let limit = if line_upper.contains("USE ") {
            20 // Show more databases for USE command
        } else if line_upper.ends_with("FROM ") || line_upper.ends_with("JOIN ") {
            15 // Show more tables for FROM/JOIN
        } else {
            10 // Default limit
        };

        completions.truncate(limit);

        Ok((start, completions))
    }
}

/// MySQL Helper (integrating all functionality)
pub struct MySQLHelper {
    completer: MySQLCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
}

impl MySQLHelper {
    /// Create MySQL helper with shared metadata
    pub fn with_metadata(metadata: Arc<Mutex<DatabaseMetadata>>) -> Self {
        Self {
            completer: MySQLCompleter::with_metadata(metadata),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter::new(),
        }
    }

    /// Update current database for better context-aware suggestions
    pub fn set_current_database(&self, database: Option<String>) {
        self.completer.set_current_database(database);
    }
}

impl Completer for MySQLHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for MySQLHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        // First try history hints
        if let Some(history_hint) = self.hinter.hint(line, pos, ctx) {
            return Some(history_hint);
        }

        // Get current word being typed
        let start = self.completer.get_word_start(line, pos);
        let word = &line[start..pos];

        // Use smart suggestion engine to get suggestions
        let suggestions = self.completer.suggestion_engine.get_suggestions(line, word);

        if !suggestions.is_empty() {
            // Show most relevant suggestion as inline hint
            let top_suggestion = &suggestions[0];

            // If current word is not empty and suggestion starts with current word, show only completion part
            if !word.is_empty()
                && top_suggestion
                    .text
                    .to_lowercase()
                    .starts_with(&word.to_lowercase())
            {
                // Extract the actual completion text (remove backticks if present)
                let clean_text = top_suggestion.text.trim_matches('`');
                if clean_text.to_lowercase().starts_with(&word.to_lowercase()) {
                    let completion = &clean_text[word.len()..];
                    // Only show the completion part, not the description
                    if !completion.is_empty() {
                        return Some(completion.to_string());
                    }
                }
            }

            return None;
        }

        // Fallback to basic context hints
        let line_upper = line.to_uppercase();

        if line_upper == "USE" || line_upper.ends_with("USE ") {
            Some("💡 Enter database name (press Tab to see all options)".to_string())
        } else if line_upper.ends_with("FROM ") || line_upper.ends_with("JOIN ") {
            Some("💡 Enter table name (press Tab to see all options)".to_string())
        } else if line_upper == "SELECT" {
            Some("💡 Enter column name or * (press Tab for suggestions)".to_string())
        } else if line.trim().is_empty() {
            Some(
                "💡 Enter SQL command (e.g: SELECT, USE, SHOW) or press Tab for options"
                    .to_string(),
            )
        } else {
            None
        }
    }
}

impl Highlighter for MySQLHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        // Simple syntax highlighting: highlight SQL keywords
        let mut highlighted = line.to_string();

        // Add color to SQL keywords (displayed as bold in terminal)
        for keyword in &self.completer.sql_keywords {
            if keyword.chars().all(|c| c.is_uppercase()) {
                let pattern = format!(r"\b{}\b", regex::escape(keyword));
                if let Ok(re) = regex::Regex::new(&pattern) {
                    highlighted = re
                        .replace_all(&highlighted, format!("\x1b[1m{}\x1b[0m", keyword))
                        .to_string();
                }
            }
        }

        std::borrow::Cow::Owned(highlighted)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> std::borrow::Cow<'b, str> {
        std::borrow::Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for MySQLHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> Result<validate::ValidationResult, ReadlineError> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

impl rustyline::Helper for MySQLHelper {}
