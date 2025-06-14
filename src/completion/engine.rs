/*!
 * 智能建议引擎
 *
 * 核心补全逻辑，负责：
 * - 分析输入上下文
 * - 生成相关建议
 * - 按相关性排序
 */

use super::metadata::DatabaseMetadata;
use super::suggestion::Suggestion;
use std::sync::{Arc, Mutex};

/// 输入上下文分析结果
#[derive(Debug, PartialEq)]
pub enum InputContext {
    /// USE 命令
    UseCommand,
    /// FROM 子句
    FromClause,
    /// SELECT 子句
    SelectClause,
    /// WHERE 子句
    WhereClause,
    /// 一般情况
    General,
}

/// 智能建议引擎
pub struct SmartSuggestionEngine {
    metadata: Arc<Mutex<DatabaseMetadata>>,
    sql_keywords: Vec<String>,
}

impl SmartSuggestionEngine {
    /// 创建新的建议引擎
    pub fn new(metadata: Arc<Mutex<DatabaseMetadata>>, sql_keywords: Vec<String>) -> Self {
        Self {
            metadata,
            sql_keywords,
        }
    }

    /// 获取智能建议列表
    pub fn get_suggestions(&self, line: &str, word: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        let line_upper = line.to_uppercase();
        let word_lower = word.to_lowercase();

        // 分析当前输入上下文
        let context = self.analyze_context(&line_upper);

        // 根据上下文生成相应的建议
        match context {
            InputContext::UseCommand => {
                suggestions.extend(self.get_database_suggestions(&word_lower));
            }
            InputContext::FromClause => {
                suggestions.extend(self.get_table_suggestions(&word_lower));
            }
            InputContext::SelectClause => {
                suggestions.extend(self.get_column_suggestions(&word_lower));
                suggestions.extend(self.get_function_suggestions(&word_lower));
            }
            InputContext::WhereClause => {
                suggestions.extend(self.get_column_suggestions(&word_lower));
                suggestions.extend(self.get_condition_suggestions(&word_lower));
            }
            InputContext::General => {
                suggestions.extend(self.get_sql_keyword_suggestions(&word_lower));
                if word.is_empty() {
                    suggestions.extend(self.get_common_command_suggestions());
                }
            }
        }

        // 按相关性排序并限制数量
        suggestions.sort_by(|a, b| b.relevance.cmp(&a.relevance));
        suggestions.truncate(10);

        suggestions
    }

    /// 分析输入上下文
    fn analyze_context(&self, line: &str) -> InputContext {
        let words: Vec<&str> = line.split_whitespace().collect();

        // USE 命令检测
        if !words.is_empty() && words[0] == "USE" {
            return InputContext::UseCommand;
        }

        // FROM/JOIN 子句检测（更精确的检测）
        let line_trimmed = line.trim();
        if line_trimmed.ends_with("FROM") || line_trimmed.ends_with("FROM ") ||
           line_trimmed.ends_with("JOIN") || line_trimmed.ends_with("JOIN ") ||
           line.contains(" FROM ") || line.contains(" JOIN ") {
            return InputContext::FromClause;
        }

        // SELECT 子句检测（没有 FROM 的情况）
        if line.contains("SELECT") && !line.contains("FROM") {
            return InputContext::SelectClause;
        }

        // WHERE/HAVING 子句检测
        if line.contains("WHERE ") || line.contains("HAVING ") {
            return InputContext::WhereClause;
        }

        InputContext::General
    }

    /// 计算匹配相关性
    fn calculate_relevance(&self, item: &str, word: &str, base_score: u8) -> u8 {
        if word.is_empty() {
            return base_score;
        }

        let item_lower = item.to_lowercase();
        let word_lower = word.to_lowercase();

        if item_lower == word_lower {
            100 // 完全匹配
        } else if item_lower.starts_with(&word_lower) {
            (base_score + 15).min(95) // 前缀匹配
        } else if item_lower.contains(&word_lower) {
            (base_score + 5).min(85) // 包含匹配
        } else {
            base_score.saturating_sub(10) // 降低不匹配项的分数
        }
    }

    /// 获取数据库建议
    fn get_database_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let metadata = self.metadata.lock().unwrap();
        let mut suggestions = Vec::new();

        for db in metadata.get_databases() {
            let relevance = self.calculate_relevance(db, word, 90);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::database(db.clone(), relevance));
            }
        }

        suggestions
    }

    /// 获取表建议
    fn get_table_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let metadata = self.metadata.lock().unwrap();
        let mut suggestions = Vec::new();

        for (db, table) in metadata.get_all_tables() {
            let relevance = self.calculate_relevance(table, word, 85);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::table(table.clone(), db, relevance));
            }
        }

        suggestions
    }

    /// 获取列建议
    fn get_column_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let metadata = self.metadata.lock().unwrap();
        let mut suggestions = Vec::new();

        for (table, column) in metadata.get_all_columns() {
            let relevance = self.calculate_relevance(column, word, 80);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::column(column.clone(), table, relevance));
            }
        }

        suggestions
    }

    /// 获取函数建议
    fn get_function_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let functions = [
            ("COUNT", "计算行数"),
            ("SUM", "求和"),
            ("AVG", "平均值"),
            ("MAX", "最大值"),
            ("MIN", "最小值"),
            ("NOW", "当前时间"),
            ("CONCAT", "字符串连接"),
            ("UPPER", "转大写"),
            ("LOWER", "转小写"),
            ("SUBSTRING", "字符串截取"),
            ("LENGTH", "字符串长度"),
            ("TRIM", "去除空格"),
            ("DATE", "日期函数"),
            ("YEAR", "获取年份"),
            ("MONTH", "获取月份"),
            ("DAY", "获取日期"),
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

    /// 获取条件关键字建议
    fn get_condition_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let conditions = [
            ("AND", "逻辑与"),
            ("OR", "逻辑或"),
            ("NOT", "逻辑非"),
            ("IN", "包含于列表"),
            ("LIKE", "模式匹配"),
            ("BETWEEN", "区间范围"),
            ("IS NULL", "为空值"),
            ("IS NOT NULL", "非空值"),
            ("EXISTS", "存在子查询"),
            ("REGEXP", "正则表达式匹配"),
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

    /// 获取SQL关键字建议
    fn get_sql_keyword_suggestions(&self, word: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();
        for keyword in &self.sql_keywords {
            let relevance = self.calculate_relevance(keyword, word, 65);
            if relevance > 50 || word.is_empty() {
                suggestions.push(Suggestion::sql_keyword(
                    keyword.clone(),
                    format!("SQL 关键字: {}", keyword),
                    relevance,
                ));
            }
        }

        suggestions
    }

    /// 获取常用命令建议
    fn get_common_command_suggestions(&self) -> Vec<Suggestion> {
        vec![
            Suggestion::command(
                "SELECT * FROM".to_string(),
                "查询表中所有数据".to_string(),
                95,
            ),
            Suggestion::command(
                "SHOW DATABASES".to_string(),
                "显示所有数据库".to_string(),
                90,
            ),
            Suggestion::command(
                "SHOW TABLES".to_string(),
                "显示当前数据库的所有表".to_string(),
                85,
            ),
            Suggestion::command("USE".to_string(), "切换到指定数据库".to_string(), 80),
            Suggestion::command("DESCRIBE".to_string(), "查看表结构".to_string(), 75),
            Suggestion::command("INSERT INTO".to_string(), "插入数据".to_string(), 70),
            Suggestion::command("UPDATE".to_string(), "更新数据".to_string(), 65),
            Suggestion::command("DELETE FROM".to_string(), "删除数据".to_string(), 60),
        ]
    }
}
