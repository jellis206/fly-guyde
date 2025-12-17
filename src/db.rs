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

        let db = Self { conn };
        db.create_history_table()?;

        Ok(db)
    }

    fn create_history_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS query_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    pub fn load_query_history(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT query FROM query_history ORDER BY timestamp ASC"
        )?;

        let queries = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(queries)
    }

    pub fn save_query_to_history(&self, query: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO query_history (query) VALUES (?1)",
            [query],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn clear_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM query_history", [])?;
        Ok(())
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
                    // Use get_ref to handle different SQLite types properly
                    let value = match row.get_ref(i) {
                        Ok(val_ref) => {
                            use rusqlite::types::ValueRef;
                            match val_ref {
                                ValueRef::Null => "NULL".to_string(),
                                ValueRef::Integer(i) => i.to_string(),
                                ValueRef::Real(f) => format!("{:.2}", f),
                                ValueRef::Text(s) => {
                                    String::from_utf8_lossy(s).to_string()
                                }
                                ValueRef::Blob(b) => {
                                    format!("<BLOB {} bytes>", b.len())
                                }
                            }
                        }
                        Err(_) => "NULL".to_string(),
                    };
                    values.push(value);
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

    pub fn fuzzy_search_products(&self, recommendation: &str, patterns: &[String]) -> Result<Vec<(String, String, f32)>> {
        use crate::fuzzy::FuzzyMatcher;

        let mut all_results = Vec::new();

        for pattern in patterns {
            // Search in products table (title and description)
            let sql = "
                SELECT DISTINCT p.title, p.vendor, p.price_min, p.description
                FROM products p
                WHERE p.title LIKE ?1 OR p.description LIKE ?1
                LIMIT 20
            ";

            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map([pattern], |row| {
                let title: String = row.get(0)?;
                let vendor: String = row.get(1)?;
                let price: f64 = row.get(2)?;
                let description: String = row.get(3).unwrap_or_default();

                Ok((title, vendor, price, description))
            })?;

            for row_result in rows {
                if let Ok((title, vendor, price, _description)) = row_result {
                    let score = FuzzyMatcher::calculate_match_score(
                        recommendation,
                        &title,
                        pattern,
                    );
                    let display = format!("{} | {} | ${:.2}", title, vendor, price);
                    all_results.push((title, display, score));
                }
            }

            // Search in tags
            let tag_sql = "
                SELECT DISTINCT p.title, p.vendor, p.price_min
                FROM products p
                JOIN product_tags pt ON p.id = pt.product_id
                JOIN tags t ON pt.tag_id = t.id
                WHERE t.name LIKE ?1
                LIMIT 20
            ";

            let mut tag_stmt = self.conn.prepare(tag_sql)?;
            let tag_rows = tag_stmt.query_map([pattern], |row| {
                let title: String = row.get(0)?;
                let vendor: String = row.get(1)?;
                let price: f64 = row.get(2)?;

                Ok((title, vendor, price))
            })?;

            for row_result in tag_rows {
                if let Ok((title, vendor, price)) = row_result {
                    // Check if we already have this product
                    if all_results.iter().any(|(t, _, _)| t == &title) {
                        continue;
                    }

                    let score = FuzzyMatcher::calculate_match_score(
                        recommendation,
                        &title,
                        pattern,
                    ) * 0.9; // Tag matches get slightly lower score

                    let display = format!("{} | {} | ${:.2}", title, vendor, price);
                    all_results.push((title, display, score));
                }
            }
        }

        Ok(all_results)
    }

    pub fn merge_and_score_results(&self, all_results: Vec<(String, String, f32)>) -> Result<QueryResult> {
        // Deduplicate by title, keeping highest score
        let mut best_scores: HashMap<String, (String, f32)> = HashMap::new();

        for (title, display, score) in all_results {
            best_scores
                .entry(title.clone())
                .and_modify(|(_, existing_score)| {
                    if score > *existing_score {
                        *existing_score = score;
                    }
                })
                .or_insert((display, score));
        }

        // Convert to QueryResult format
        let mut results: Vec<(String, f32)> = best_scores
            .into_iter()
            .map(|(_, (display, score))| (display, score))
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(50);

        // Format as QueryResult
        let columns = vec!["Product | Vendor | Price".to_string(), "Relevance".to_string()];
        let rows: Vec<Vec<String>> = results
            .iter()
            .map(|(display, score)| {
                vec![
                    display.clone(),
                    format!("{:.0}%", score * 100.0),
                ]
            })
            .collect();

        let row_count = rows.len();

        Ok(QueryResult {
            columns,
            rows,
            row_count,
        })
    }

    #[allow(dead_code)]
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
