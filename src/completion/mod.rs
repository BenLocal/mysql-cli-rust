/*!
 * MySQL CLI Rust - 智能补全模块
 *
 * 提供传统命令行补全体验，支持：
 * - SQL 关键字补全
 * - 数据库名、表名、字段名的自动补全
 * - 上下文感知的智能建议
 * - 内联提示和历史记录
 */

pub mod engine;
pub mod helper;
pub mod metadata;
pub mod suggestion;

// 重新导出主要接口
pub use helper::MySQLHelper;
