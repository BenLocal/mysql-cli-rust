use super::*;
use std::sync::{Arc, Mutex};

fn create_test_engine() -> SmartSuggestionEngine {
    let mut md = DatabaseMetadata::new();
    md.databases = vec!["test_db".to_string(), "sales".to_string()];
    md.tables.insert(
        "test_db".to_string(),
        vec!["users".to_string(), "orders".to_string()],
    );
    md.columns.insert(
        "test_db.users".to_string(),
        vec!["id".to_string(), "name".to_string(), "email".to_string()],
    );
    md.columns.insert(
        "test_db.orders".to_string(),
        vec![
            "order_id".to_string(),
            "user_id".to_string(),
            "amount".to_string(),
        ],
    );

    let metadata = Arc::new(Mutex::new(md));

    let sql_keywords = vec![
        "SELECT".to_string(),
        "FROM".to_string(),
        "WHERE".to_string(),
        "INSERT".to_string(),
        "UPDATE".to_string(),
        "ORDER".to_string(),
        "BY".to_string(),
        "GROUP".to_string(),
        "HAVING".to_string(),
    ];
    SmartSuggestionEngine::new(metadata, sql_keywords)
}

#[test]
fn test_use_command_context() {
    let engine = create_test_engine();
    assert_eq!(engine.analyze_context("USE"), InputContext::UseCommand);
    assert_eq!(
        engine.analyze_context("USE test_db"),
        InputContext::UseCommand
    );
    assert_eq!(
        engine.analyze_context("use database_name"),
        InputContext::UseCommand
    );
}

#[test]
fn test_select_context() {
    let engine = create_test_engine();
    assert_eq!(engine.analyze_context("SELECT"), InputContext::SelectClause);
    assert_eq!(
        engine.analyze_context("SELECT *"),
        InputContext::SelectClause
    );
    assert_eq!(
        engine.analyze_context("SELECT name, age"),
        InputContext::SelectClause
    );
}

#[test]
fn test_from_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("SELECT * FROM"),
        InputContext::FromClause
    );
    assert_eq!(
        engine.analyze_context("SELECT * FROM users JOIN"),
        InputContext::FromClause
    );
    assert_eq!(engine.analyze_context("FROM"), InputContext::FromClause);
}

#[test]
fn test_where_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("SELECT * FROM users WHERE"),
        InputContext::WhereClause
    );
    assert_eq!(engine.analyze_context("WHERE"), InputContext::WhereClause);
    assert_eq!(
        engine.analyze_context("SELECT name FROM employees WHERE age > 30 AND"),
        InputContext::WhereClause
    );
}

#[test]
fn test_order_by_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("SELECT * FROM users ORDER BY"),
        InputContext::OrderByClause
    );
    assert_eq!(
        engine.analyze_context("ORDER BY"),
        InputContext::OrderByClause
    );
}

#[test]
fn test_group_by_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("SELECT COUNT(*) FROM orders GROUP BY"),
        InputContext::GroupByClause
    );
    assert_eq!(
        engine.analyze_context("GROUP BY"),
        InputContext::GroupByClause
    );
}

#[test]
fn test_having_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("SELECT COUNT(*) FROM orders GROUP BY status HAVING"),
        InputContext::HavingClause
    );
    assert_eq!(engine.analyze_context("HAVING"), InputContext::HavingClause);
}

#[test]
fn test_insert_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("INSERT INTO"),
        InputContext::InsertIntoClause
    );
    assert_eq!(
        engine.analyze_context("INSERT"),
        InputContext::InsertIntoClause
    );
}

#[test]
fn test_update_context() {
    let engine = create_test_engine();
    assert_eq!(engine.analyze_context("UPDATE"), InputContext::UpdateClause);
    assert_eq!(
        engine.analyze_context("UPDATE users SET"),
        InputContext::UpdateClause
    );
}

#[test]
fn test_join_on_context() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("SELECT * FROM users u JOIN orders o ON"),
        InputContext::JoinOnClause
    );
    assert_eq!(engine.analyze_context("ON"), InputContext::JoinOnClause);
}

#[test]
fn test_general_context() {
    let engine = create_test_engine();
    assert_eq!(engine.analyze_context(""), InputContext::General);
    assert_eq!(engine.analyze_context("SHOW TABLES"), InputContext::General);
    assert_eq!(
        engine.analyze_context("DESCRIBE users"),
        InputContext::General
    );
}

#[test]
fn test_complex_queries() {
    let engine = create_test_engine();

    // Complex SELECT with subquery
    assert_eq!(
        engine.analyze_context(
            "SELECT u.name FROM users u WHERE u.id IN (SELECT user_id FROM orders WHERE"
        ),
        InputContext::WhereClause
    );

    // Multi-table JOIN
    assert_eq!(
        engine.analyze_context(
            "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id WHERE"
        ),
        InputContext::WhereClause
    );
}

#[test]
fn test_select_column_suggestions() {
    let engine = create_test_engine();
    assert_eq!(
        engine.analyze_context("select * from worker_jobs where"),
        InputContext::WhereClause
    );
    engine.set_current_database(Some("test_db".to_string()));

    let suggestions = engine.get_column_suggestions_for_query("select * from orders where", "");

    // display all columns from the orders table
    assert_eq!(suggestions.len(), 6);
}
