/*!
 * MySQL 补全助手
 *
 * 整合了所有补全功能的主要接口，实现 rustyline 的各种 trait
 */

use super::engine::SmartSuggestionEngine;
use super::metadata::DatabaseMetadata;
use anyhow::Result;
use mysql;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::Context;
use std::sync::{Arc, Mutex};

/// MySQL 补全器
pub struct MySQLCompleter {
    sql_keywords: Vec<String>,
    metadata: Arc<Mutex<DatabaseMetadata>>,
    suggestion_engine: SmartSuggestionEngine,
}

impl MySQLCompleter {
    /// 创建新的补全器
    pub fn new() -> Self {
        let sql_keywords = Self::init_sql_keywords();
        let metadata = Arc::new(Mutex::new(DatabaseMetadata::new()));
        let suggestion_engine = SmartSuggestionEngine::new(metadata.clone(), sql_keywords.clone());

        Self {
            sql_keywords,
            metadata,
            suggestion_engine,
        }
    }

    /// 使用共享元数据创建补全器
    pub fn with_metadata(metadata: Arc<Mutex<DatabaseMetadata>>) -> Self {
        let sql_keywords = Self::init_sql_keywords();
        let suggestion_engine = SmartSuggestionEngine::new(metadata.clone(), sql_keywords.clone());

        Self {
            sql_keywords,
            metadata,
            suggestion_engine,
        }
    }

    /// 初始化SQL关键字列表
    fn init_sql_keywords() -> Vec<String> {
        let keywords = [
            // 基本SQL关键字
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
            // 条件和操作符
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
            // 聚合函数
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "GROUP_CONCAT",
            // 字符串函数
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
            // 事务控制
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "SAVEPOINT",
            "RELEASE",
            "TRANSACTION",
            "READ",
            "WRITE",
            "ONLY",
            // 其他
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

    /// 更新数据库元数据
    pub fn update_metadata(&self, conn: &mut mysql::Conn) -> Result<()> {
        let mut metadata = self.metadata.lock().unwrap();
        metadata.update_from_connection(conn)
    }

    /// 获取当前单词的起始位置
    fn get_word_start(&self, line: &str, pos: usize) -> usize {
        line[..pos]
            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',' || c == '.' || c == ';')
            .map(|i| i + 1)
            .unwrap_or(0)
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

        // 使用智能提示引擎获取建议
        let suggestions = self.suggestion_engine.get_suggestions(line, word);

        let mut completions = Vec::new();

        // 将智能建议转换为 rustyline 的 Pair 格式
        for suggestion in suggestions {
            completions.push(Pair {
                display: suggestion.format_display(),
                replacement: suggestion.text,
            });
        }

        // 如果没有智能建议，回退到传统的关键字补全
        if completions.is_empty() {
            let word_lower = word.to_lowercase();
            for keyword in &self.sql_keywords {
                if keyword.to_lowercase().starts_with(&word_lower) {
                    completions.push(Pair {
                        display: format!("🔵 {} - SQL关键字", keyword),
                        replacement: keyword.clone(),
                    });
                }
            }
        }

        // 限制结果数量
        completions.truncate(10);

        Ok((start, completions))
    }
}

/// MySQL 助手（整合所有功能）
pub struct MySQLHelper {
    completer: MySQLCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
}

impl MySQLHelper {
    /// 创建新的MySQL助手
    pub fn new() -> Self {
        Self {
            completer: MySQLCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter::new(),
        }
    }

    /// 使用共享元数据创建MySQL助手
    pub fn with_metadata(metadata: Arc<Mutex<DatabaseMetadata>>) -> Self {
        Self {
            completer: MySQLCompleter::with_metadata(metadata),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter::new(),
        }
    }

    /// 更新数据库元数据
    pub fn update_metadata(&self, conn: &mut mysql::Conn) -> Result<()> {
        self.completer.update_metadata(conn)
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
        // 首先尝试历史提示
        if let Some(history_hint) = self.hinter.hint(line, pos, ctx) {
            return Some(history_hint);
        }

        // 获取当前正在输入的单词
        let start = self.completer.get_word_start(line, pos);
        let word = &line[start..pos];

        // 使用智能提示引擎获取建议
        let suggestions = self.completer.suggestion_engine.get_suggestions(line, word);

        if !suggestions.is_empty() {
            // 显示最相关的建议作为内联提示
            let top_suggestion = &suggestions[0];

            // 如果当前单词不为空，且建议以当前单词开头，显示补全部分
            if !word.is_empty()
                && top_suggestion
                    .text
                    .to_lowercase()
                    .starts_with(&word.to_lowercase())
            {
                let completion = &top_suggestion.text[word.len()..];
                return Some(format!("{} - {}", completion, top_suggestion.description));
            }

            // 否则显示完整的建议
            return Some(format!("💡 {}", top_suggestion.format_display()));
        }

        // 回退到基本的上下文提示
        let line_upper = line.to_uppercase();

        if line_upper == "USE" || line_upper.ends_with("USE ") {
            Some("💡 输入数据库名称 (按 Tab 查看所有选项)".to_string())
        } else if line_upper.ends_with("FROM ") || line_upper.ends_with("JOIN ") {
            Some("💡 输入表名 (按 Tab 查看所有选项)".to_string())
        } else if line_upper == "SELECT" {
            Some("💡 输入列名或 * (按 Tab 查看建议)".to_string())
        } else if line.trim().is_empty() {
            Some("💡 输入 SQL 命令 (如: SELECT, USE, SHOW) 或按 Tab 查看选项".to_string())
        } else {
            None
        }
    }
}

impl Highlighter for MySQLHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        // 简单的语法高亮：高亮SQL关键字
        let mut highlighted = line.to_string();

        // 为SQL关键字添加颜色（在终端中显示为粗体）
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
