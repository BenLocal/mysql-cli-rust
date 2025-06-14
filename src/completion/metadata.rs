/*!
 * 数据库元数据管理
 *
 * 负责缓存和管理数据库的元数据信息，包括：
 * - 数据库列表
 * - 表名信息
 * - 字段信息
 * - 缓存刷新逻辑
 */

use anyhow::Result;
use mysql::prelude::*;
use std::collections::HashMap;

/// 数据库元数据缓存
#[derive(Debug)]
pub struct DatabaseMetadata {
    /// 数据库列表
    pub databases: Vec<String>,
    /// 表信息：数据库名 -> 表名列表
    pub tables: HashMap<String, Vec<String>>,
    /// 字段信息：表名 -> 字段列表
    pub columns: HashMap<String, Vec<String>>,
    /// 最后更新时间
    last_update: std::time::Instant,
}

impl DatabaseMetadata {
    /// 创建新的元数据实例
    pub fn new() -> Self {
        Self {
            databases: Vec::new(),
            tables: HashMap::new(),
            columns: HashMap::new(),
            last_update: std::time::Instant::now(),
        }
    }

    /// 检查是否需要刷新缓存（5分钟过期）
    pub fn needs_refresh(&self) -> bool {
        self.last_update.elapsed().as_secs() > 300
    }

    /// 从数据库连接更新元数据
    pub fn update_from_connection(&mut self, conn: &mut mysql::Conn) -> Result<()> {
        if !self.needs_refresh() {
            return Ok(());
        }

        // 获取数据库列表
        let databases: Vec<String> = conn.query("SHOW DATABASES")?;
        self.databases = databases.clone();

        // 清空旧的表和列信息
        self.tables.clear();
        self.columns.clear();

        // 获取每个数据库的表信息
        for db in &databases {
            // 跳过系统数据库的详细表信息获取（避免权限问题）
            if self.is_system_database(db) {
                continue;
            }

            if let Ok(tables) = conn.query::<String, _>(format!("SHOW TABLES FROM `{}`", db)) {
                self.tables.insert(db.clone(), tables.clone());

                // 获取每个表的列信息
                for table in &tables {
                    let query = format!("SHOW COLUMNS FROM `{}`.`{}`", db, table);
                    if let Ok(rows) = conn.query::<mysql::Row, _>(query) {
                        let mut columns = Vec::new();
                        for row in rows {
                            if let Some(field_name) = row.get::<String, _>(0) {
                                columns.push(field_name);
                            }
                        }
                        let table_key = format!("{}.{}", db, table);
                        self.columns.insert(table_key, columns);
                    }
                }
            }
        }

        self.last_update = std::time::Instant::now();
        Ok(())
    }

    /// 检查是否为系统数据库
    fn is_system_database(&self, db: &str) -> bool {
        matches!(
            db,
            "information_schema" | "mysql" | "performance_schema" | "sys"
        )
    }

    /// 获取所有数据库名
    pub fn get_databases(&self) -> &Vec<String> {
        &self.databases
    }

    /// 获取所有表名（跨数据库）
    pub fn get_all_tables(&self) -> Vec<(&String, &String)> {
        let mut tables = Vec::new();
        for (db, table_list) in &self.tables {
            for table in table_list {
                tables.push((db, table));
            }
        }
        tables
    }

    /// 获取所有列名（跨表）
    pub fn get_all_columns(&self) -> Vec<(&String, &String)> {
        let mut columns = Vec::new();
        for (table, column_list) in &self.columns {
            for column in column_list {
                columns.push((table, column));
            }
        }
        columns
    }
}

impl Default for DatabaseMetadata {
    fn default() -> Self {
        Self::new()
    }
}
