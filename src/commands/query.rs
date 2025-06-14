use crate::database::{Connection, QueryResult};
use anyhow::Result;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use std::time::Instant;

pub struct QueryExecutor;

impl QueryExecutor {
    pub fn new() -> Self {
        QueryExecutor
    }

    pub fn execute(&self, connection: &mut Connection, query: &str) -> Result<()> {
        let start_time = Instant::now();

        // Check if query is empty
        if query.trim().is_empty() {
            return Ok(());
        }

        match connection.execute_query(query) {
            Ok(result) => {
                let duration = start_time.elapsed();

                if result.rows.is_empty() && result.columns.is_empty() {
                    // Non-SELECT query (INSERT, UPDATE, DELETE, etc.)
                    println!(
                        "Query OK, {} rows affected ({:.3} sec)",
                        0,
                        duration.as_secs_f64()
                    );
                } else {
                    // SELECT query with results
                    self.display_results(&result);
                    let row_count = result.rows.len();
                    if row_count == 1 {
                        println!(
                            "{} row in set ({:.3} sec)",
                            row_count,
                            duration.as_secs_f64()
                        );
                    } else {
                        println!(
                            "{} rows in set ({:.3} sec)",
                            row_count,
                            duration.as_secs_f64()
                        );
                    }
                }
            }
            Err(e) => {
                println!("ERROR: {}", e);
            }
        }

        Ok(())
    }

    fn display_results(&self, result: &QueryResult) {
        if result.columns.is_empty() {
            return;
        }

        let mut table = Table::new();
        table.set_content_arrangement(ContentArrangement::Dynamic);

        // Add headers
        let mut header_cells = Vec::new();
        for column in &result.columns {
            header_cells.push(Cell::new(column).add_attribute(Attribute::Bold));
        }
        table.set_header(header_cells);

        // Add rows
        for row in &result.rows {
            let mut cells = Vec::new();
            for value in row {
                cells.push(Cell::new(value));
            }
            table.add_row(cells);
        }

        println!("{}", table);
    }
}
