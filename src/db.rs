use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_count: usize,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .context(format!("Failed to open database at {}", db_path))?;
        Ok(Self { conn })
    }

    pub fn execute_query(&self, sql: &str) -> Result<QueryResult> {
        let mut stmt = self.conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let columns: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt
            .query_map([], |row| {
                let mut values = Vec::new();
                for i in 0..column_count {
                    let value: Result<String, _> = row.get(i);
                    values.push(value.unwrap_or_else(|_| "NULL".to_string()));
                }
                Ok(values)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let row_count = rows.len();

        Ok(QueryResult {
            columns,
            rows,
            row_count,
        })
    }

    pub fn get_schema_info(&self) -> Result<String> {
        let mut schema = String::new();

        schema.push_str("DATABASE SCHEMA:\n\n");

        let tables_sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        let mut stmt = self.conn.prepare(tables_sql)?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for table in tables {
            schema.push_str(&format!("Table: {}\n", table));

            let pragma_sql = format!("PRAGMA table_info({})", table);
            let mut pragma_stmt = self.conn.prepare(&pragma_sql)?;
            let columns: Vec<(String, String)> = pragma_stmt
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    let col_type: String = row.get(2)?;
                    Ok((name, col_type))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            for (name, col_type) in columns {
                schema.push_str(&format!("  - {} ({})\n", name, col_type));
            }
            schema.push('\n');
        }

        Ok(schema)
    }

    pub fn get_schema_with_relationships(&self) -> Result<String> {
        let mut schema = self.get_schema_info()?;

        schema.push_str("FOREIGN KEY RELATIONSHIPS:\n\n");

        let tables_sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        let mut stmt = self.conn.prepare(tables_sql)?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for table in tables {
            let fk_sql = format!("PRAGMA foreign_key_list({})", table);
            let mut fk_stmt = self.conn.prepare(&fk_sql)?;

            let fks: Vec<(String, String)> = fk_stmt
                .query_map([], |row| {
                    let from_col: String = row.get(3)?;
                    let to_table: String = row.get(2)?;
                    Ok((from_col, to_table))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            for (from_col, to_table) in fks {
                schema.push_str(&format!("{}.{} -> {}\n", table, from_col, to_table));
            }
        }

        Ok(schema)
    }

    pub fn get_schema_with_sample_data(&self) -> Result<String> {
        let mut schema = self.get_schema_with_relationships()?;

        schema.push_str("\n\nSAMPLE DATA (first 3 rows from each table):\n\n");

        let tables_sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        let mut stmt = self.conn.prepare(tables_sql)?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for table in tables {
            schema.push_str(&format!("Sample from {}:\n", table));

            let sample_sql = format!("SELECT * FROM {} LIMIT 3", table);
            match self.execute_query(&sample_sql) {
                Ok(result) => {
                    schema.push_str(&format!("  Columns: {}\n", result.columns.join(", ")));
                    for (i, row) in result.rows.iter().enumerate() {
                        let truncated_values: Vec<String> = row
                            .iter()
                            .map(|v| {
                                if v.len() > 50 {
                                    format!("{}...", &v[..50])
                                } else {
                                    v.clone()
                                }
                            })
                            .collect();
                        schema.push_str(&format!("  Row {}: {}\n", i + 1, truncated_values.join(" | ")));
                    }
                }
                Err(e) => {
                    schema.push_str(&format!("  Error fetching sample: {}\n", e));
                }
            }
            schema.push('\n');
        }

        Ok(schema)
    }

    pub fn get_statistics(&self) -> Result<HashMap<String, usize>> {
        let mut stats = HashMap::new();

        let count_sql = "SELECT COUNT(*) FROM products";
        let products: usize = self.conn.query_row(count_sql, [], |row| row.get(0))?;
        stats.insert("products".to_string(), products);

        let count_sql = "SELECT COUNT(*) FROM variants";
        let variants: usize = self.conn.query_row(count_sql, [], |row| row.get(0))?;
        stats.insert("variants".to_string(), variants);

        let count_sql = "SELECT COUNT(*) FROM tags";
        let tags: usize = self.conn.query_row(count_sql, [], |row| row.get(0))?;
        stats.insert("tags".to_string(), tags);

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_connection() {
        let db = Database::new("flies.db").expect("Failed to connect to database");
        let stats = db.get_statistics().expect("Failed to get statistics");
        assert!(stats.get("products").unwrap_or(&0) > &0);
    }
}
