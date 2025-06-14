/*!
 * 智能建议数据结构
 *
 * 定义补全建议的数据结构和分类系统
 */

/// 智能提示建议项
#[derive(Clone, Debug)]
pub struct Suggestion {
    /// 补全文本
    pub text: String,
    /// 描述信息
    pub description: String,
    /// 建议分类
    pub category: SuggestionCategory,
    /// 相关性评分 (0-100)
    pub relevance: u8,
}

/// 建议分类枚举
#[derive(Clone, Debug, PartialEq)]
pub enum SuggestionCategory {
    /// 数据库
    Database,
    /// 表
    Table,
    /// 列/字段
    Column,
    /// SQL关键字
    SqlKeyword,
    /// 函数
    Function,
    /// 命令
    Command,
}

impl Suggestion {
    /// 创建新的建议项
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
            relevance: relevance.min(100), // 确保不超过100
        }
    }

    /// 格式化显示文本（带 emoji 图标）
    pub fn format_display(&self) -> String {
        let icon = self.category.icon();
        format!("{} {} - {}", icon, self.text, self.description)
    }

    /// 创建数据库建议
    pub fn database(name: String, relevance: u8) -> Self {
        Self::new(
            format!("`{}`", name),
            format!("数据库: {}", name),
            SuggestionCategory::Database,
            relevance,
        )
    }

    /// 创建表建议
    pub fn table(name: String, database: &str, relevance: u8) -> Self {
        Self::new(
            format!("`{}`", name),
            format!("表: {} (在 {} 数据库)", name, database),
            SuggestionCategory::Table,
            relevance,
        )
    }

    /// 创建列建议
    pub fn column(name: String, table: &str, relevance: u8) -> Self {
        Self::new(
            format!("`{}`", name),
            format!("列: {} (来自表 {})", name, table),
            SuggestionCategory::Column,
            relevance,
        )
    }

    /// 创建SQL关键字建议
    pub fn sql_keyword(keyword: String, description: String, relevance: u8) -> Self {
        Self::new(
            keyword,
            description,
            SuggestionCategory::SqlKeyword,
            relevance,
        )
    }

    /// 创建函数建议
    pub fn function(name: String, description: String, relevance: u8) -> Self {
        Self::new(name, description, SuggestionCategory::Function, relevance)
    }

    /// 创建命令建议
    pub fn command(command: String, description: String, relevance: u8) -> Self {
        Self::new(command, description, SuggestionCategory::Command, relevance)
    }
}

impl SuggestionCategory {
    /// 获取分类对应的 emoji 图标
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
