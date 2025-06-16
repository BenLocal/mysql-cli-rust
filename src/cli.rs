use crate::commands::QueryExecutor;
use crate::completion::{metadata::DatabaseMetadata, MySQLHelper};
use crate::database::Connection;
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::{history::DefaultHistory, CompletionType, Config, Editor};
use std::sync::{Arc, Mutex};

pub struct Cli {
    connection: Connection,
    query_executor: QueryExecutor,
    editor: Editor<MySQLHelper, DefaultHistory>,
    current_database: Option<String>,
    metadata: Arc<Mutex<DatabaseMetadata>>,
}

impl Cli {
    pub fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: Option<&str>,
    ) -> Result<Self> {
        let mut connection = Connection::new(host, port, user, password, database)?;
        let query_executor = QueryExecutor::new();
        let current_database = database.map(|d| d.to_string());

        println!("Welcome to the MySQL monitor. Commands end with ; or \\g.");
        println!("Your MySQL connection id is {}", connection.connection_id());
        println!("Server version: {}", connection.server_version());
        println!();
        println!(
            "Type 'help;' or '\\h' for help. Type '\\c' to clear the current input statement."
        );
        println!();

        // 配置 rustyline 编辑器
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .auto_add_history(true)
            .edit_mode(rustyline::EditMode::Emacs)
            .build();

        let mut editor = Editor::with_config(config)?; // 创建共享的数据库元数据
        let metadata = Arc::new(Mutex::new(DatabaseMetadata::new()));

        // 设置 MySQL 补全助手
        let helper = MySQLHelper::with_metadata(metadata.clone());

        // 更新数据库元数据
        if let Ok(mut meta) = metadata.lock() {
            let _ = meta.update_from_connection(connection.get_conn_mut());
        }

        editor.set_helper(Some(helper));

        // Set initial current database in completion engine if available
        if let Some(helper) = editor.helper() {
            helper.set_current_database(current_database.clone());
        }

        Ok(Self {
            connection,
            query_executor,
            editor,
            current_database,
            metadata,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let prompt = self.get_prompt();

            let readline = self.editor.readline(&prompt);
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // 添加到历史记录
                    self.editor.add_history_entry(line)?;

                    // Handle special commands
                    if line.starts_with('\\') {
                        if let Err(e) = self.handle_special_command(line) {
                            println!("Error: {}", e);
                        }
                        continue;
                    }

                    // Handle SQL queries
                    if line.ends_with(';') || line.ends_with("\\g") {
                        let query = line.trim_end_matches(';').trim_end_matches("\\g").trim();
                        if let Err(e) = self.execute_query(query) {
                            println!("ERROR: {}", e);
                        }
                    } else {
                        // For simplicity, require explicit semicolons
                        println!("Please end your SQL statement with ';' or '\\g'");
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("Bye");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        Ok(())
    }

    fn get_prompt(&self) -> String {
        match &self.current_database {
            Some(db) => format!("mysql [{}]> ", db),
            None => "mysql> ".to_string(),
        }
    }

    fn handle_special_command(&mut self, command: &str) -> Result<()> {
        match command {
            "\\q" | "\\quit" | "\\exit" => {
                println!("Bye");
                std::process::exit(0);
            }
            "\\h" | "\\help" => {
                self.show_help();
            }
            "\\c" | "\\clear" => {
                println!("Query cleared.");
            }
            "\\s" | "\\status" => {
                self.show_status()?;
            }
            "\\d" | "\\databases" => {
                self.execute_query("SHOW DATABASES")?;
            }
            "\\t" | "\\tables" => {
                self.execute_query("SHOW TABLES")?;
            }
            _ if command.starts_with("\\u ") => {
                let db_name = command.strip_prefix("\\u ").unwrap().trim();
                self.use_database(db_name)?;
            }
            _ => {
                println!("Unknown command: {}", command);
                println!("Type '\\h' for help.");
            }
        }
        Ok(())
    }

    fn show_help(&self) {
        println!("General SQL help:");
        println!("Note that all text commands must be first on line and end with ';'");
        println!();
        println!("\\c (\\clear)     Clear the current input statement.");
        println!("\\d (\\databases) List databases.");
        println!("\\h (\\help)      Display this help.");
        println!("\\q (\\quit)      Quit mysql.");
        println!("\\s (\\status)    Get status information from the server.");
        println!("\\t (\\tables)    List tables in current database.");
        println!("\\u <db> (\\use)  Use database <db>.");
        println!();
    }

    fn show_status(&self) -> Result<()> {
        println!("--------------");
        println!("Connection id:\t\t{}", self.connection.connection_id());
        println!(
            "Current database:\t{}",
            self.current_database.as_deref().unwrap_or("(none)")
        );
        println!("Server version:\t\t{}", self.connection.server_version());
        println!("--------------");
        Ok(())
    }

    fn use_database(&mut self, db_name: &str) -> Result<()> {
        self.execute_query(&format!("USE {}", db_name))?;
        self.current_database = Some(db_name.to_string());
        
        // Update completion engine with current database
        if let Some(helper) = self.editor.helper() {
            helper.set_current_database(self.current_database.clone());
        }
        
        println!("Database changed");
        Ok(())
    }

    fn execute_query(&mut self, query: &str) -> Result<()> {
        let trimmed_query = query.trim().to_uppercase();

        // Check if this query might change database structure
        let should_refresh_metadata = trimmed_query.starts_with("CREATE")
            || trimmed_query.starts_with("DROP")
            || trimmed_query.starts_with("ALTER")
            || trimmed_query.starts_with("USE");

        let result = self.query_executor.execute(&mut self.connection, query);

        // Refresh metadata if needed and query was successful
        if result.is_ok() && should_refresh_metadata {
            // Update database metadata
            if let Ok(mut meta) = self.metadata.lock() {
                let _ = meta.update_from_connection(self.connection.get_conn_mut());
            }

            // Update current database if USE command was executed
            if trimmed_query.starts_with("USE") {
                if let Some(db_name) = query.split_whitespace().nth(1) {
                    self.current_database = Some(db_name.trim_matches('`').to_string());
                    
                    // Update completion engine with current database
                    if let Some(helper) = self.editor.helper() {
                        helper.set_current_database(self.current_database.clone());
                    }
                    
                    // update db matadata
                    self.update_metadata();
                }
            }
        }

        result
    }

    fn update_metadata(&mut self) {
        if let Ok(mut meta) = self.metadata.lock() {
            let _ = meta.update_from_connection(self.connection.get_conn_mut());
        }
    }
}
