use anyhow::Result;
use mysql::prelude::*;
use mysql::{Conn, OptsBuilder, Value};

pub struct Connection {
    conn: Conn,
    connection_id: u32,
    server_version: String,
}

impl Connection {
    pub fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: Option<&str>,
    ) -> Result<Self> {
        let mut opts_builder = OptsBuilder::new()
            .ip_or_hostname(Some(host))
            .tcp_port(port)
            .user(Some(user))
            .pass(Some(password));

        if let Some(db) = database {
            opts_builder = opts_builder.db_name(Some(db));
        }

        let mut conn = Conn::new(opts_builder)?;

        // Get connection info
        let connection_id: u32 = conn.query_first("SELECT CONNECTION_ID()")?.unwrap_or(0);
        let server_version: String = conn.query_first("SELECT VERSION()")?.unwrap_or_default();

        Ok(Self {
            conn,
            connection_id,
            server_version,
        })
    }

    pub fn connection_id(&self) -> u32 {
        self.connection_id
    }

    pub fn server_version(&self) -> &str {
        &self.server_version
    }

    pub fn execute_query(&mut self, query: &str) -> Result<QueryResult> {
        let result = self.conn.query_iter(query)?;

        let mut rows = Vec::new();

        // Get column information
        let columns: Vec<String> = result
            .columns()
            .as_ref()
            .iter()
            .map(|col| col.name_str().to_string())
            .collect();

        // Collect all rows
        for row in result {
            let row = row?;
            let mut row_values = Vec::new();

            for i in 0..row.len() {
                let value = match row.get_opt::<Value, usize>(i) {
                    Some(Ok(value)) => format_value(&value),
                    Some(Err(_)) => "ERROR".to_string(),
                    None => "NULL".to_string(),
                };
                row_values.push(value);
            }
            rows.push(row_values);
        }

        Ok(QueryResult { columns, rows })
    }

    pub fn get_conn_mut(&mut self) -> &mut Conn {
        &mut self.conn
    }
}

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

fn format_value(value: &Value) -> String {
    match value {
        Value::NULL => "NULL".to_string(),
        Value::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Value::Int(i) => i.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Date(year, month, day, hour, minute, second, micro) => {
            if *hour == 0 && *minute == 0 && *second == 0 && *micro == 0 {
                format!("{:04}-{:02}-{:02}", year, month, day)
            } else {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    year, month, day, hour, minute, second
                )
            }
        }
        Value::Time(neg, _days, hours, minutes, seconds, _micro) => {
            let sign = if *neg { "-" } else { "" };
            format!("{}{:02}:{:02}:{:02}", sign, hours, minutes, seconds)
        }
    }
}
