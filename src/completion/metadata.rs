/*!
 * Database metadata management
 *
 * Responsible for caching and managing database metadata information, including:
 * - Database list
 * - Table information
 * - Field information
 * - Cache refresh logic
 */

use anyhow::Result;
use mysql::prelude::*;
use std::collections::HashMap;

/// Database metadata cache
#[derive(Debug)]
pub struct DatabaseMetadata {
    /// Database list
    pub databases: Vec<String>,
    /// Table information: database name -> table name list
    pub tables: HashMap<String, Vec<String>>,
    /// Field information: table name -> field list
    pub columns: HashMap<String, Vec<String>>,
    /// Last update time
    last_update: std::time::Instant,
    /// Whether data has been loaded at least once
    has_loaded: bool,
}

impl DatabaseMetadata {
    /// Create new metadata instance
    pub fn new() -> Self {
        Self {
            databases: Vec::new(),
            tables: HashMap::new(),
            columns: HashMap::new(),
            last_update: std::time::Instant::now(),
            has_loaded: false,
        }
    }

    /// Check if cache needs refresh (5 minute expiry)
    pub fn needs_refresh(&self) -> bool {
        !self.has_loaded || self.last_update.elapsed().as_secs() > 300
    }

    /// Update metadata from database connection
    pub fn update_from_connection(&mut self, conn: &mut mysql::Conn) -> Result<()> {
        if !self.needs_refresh() {
            return Ok(());
        }

        // Get database list
        let databases: Vec<String> = conn.query("SHOW DATABASES")?;
        self.databases = databases.clone();

        // Clear old table and column information
        self.tables.clear();
        self.columns.clear();

        // Get table information for each database
        for db in &databases {
            // Skip detailed table information retrieval for system databases (avoid permission issues)
            if self.is_system_database(db) {
                continue;
            }

            if let Ok(tables) = conn.query::<String, _>(format!("SHOW TABLES FROM `{}`", db)) {
                self.tables
                    .insert(db.clone().to_lowercase(), tables.clone());

                // Get column information for each table
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
                        self.columns.insert(table_key.to_lowercase(), columns);
                    }
                }
            }
        }

        self.last_update = std::time::Instant::now();
        self.has_loaded = true;
        Ok(())
    }

    /// Check if it's a system database
    fn is_system_database(&self, db: &str) -> bool {
        matches!(
            db,
            "information_schema" | "mysql" | "performance_schema" | "sys"
        )
    }

    /// Get all database names
    pub fn get_databases(&self) -> &Vec<String> {
        &self.databases
    }

    /// Get all table names (across databases)
    pub fn get_all_tables(&self) -> Vec<(&String, &String)> {
        let mut tables = Vec::new();
        for (db, table_list) in &self.tables {
            for table in table_list {
                tables.push((db, table));
            }
        }
        tables
    }

    /// Get all column names (across tables)
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
