//! Query DSL and Search Logic for CodeMate.

use crate::Language;
use chrono::{DateTime, Utc};

/// A parsed search query with semantic text and metadata filters.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// The core semantic or lexical query string
    pub raw_query: String,
    /// Filter by author name or email
    pub author: Option<String>,
    /// Filter by programming language
    pub lang: Option<Language>,
    /// Filter for results after this date
    pub after: Option<DateTime<Utc>>,
    /// Filter for results before this date
    pub before: Option<DateTime<Utc>>,
    /// Filter by file path pattern
    pub file_pattern: Option<String>,
    /// Maximum number of results
    pub limit: usize,
}

impl SearchQuery {
    /// Create a new search query from a raw input string.
    /// 
    /// Example: "storage author:Stanley lang:rust"
    pub fn parse(input: &str) -> Self {
        let mut query = SearchQuery::default();
        query.limit = 10; // Default limit

        let mut semantic_parts = Vec::new();
        let tokens = input.split_whitespace();

        for token in tokens {
            if let Some((key, value)) = token.split_once(':') {
                match key.to_lowercase().as_str() {
                    "author" => query.author = Some(value.to_string()),
                    "lang" | "language" => query.lang = Some(Language::from_str(value)),
                    "after" => {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
                            query.after = Some(dt.with_timezone(&Utc));
                        }
                    }
                    "before" => {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
                            query.before = Some(dt.with_timezone(&Utc));
                        }
                    }
                    "file" | "path" => query.file_pattern = Some(value.to_string()),
                    "limit" => {
                        if let Ok(l) = value.parse::<usize>() {
                            query.limit = l;
                        }
                    }
                    _ => semantic_parts.push(token), // Treat unknown prefix as part of query
                }
            } else {
                semantic_parts.push(token);
            }
        }

        query.raw_query = semantic_parts.join(" ");
        query
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let q = SearchQuery::parse("indexing engine");
        assert_eq!(q.raw_query, "indexing engine");
        assert_eq!(q.author, None);
    }

    #[test]
    fn test_parse_with_filters() {
        let q = SearchQuery::parse("storage author:Stanley lang:rust limit:5");
        assert_eq!(q.raw_query, "storage");
        assert_eq!(q.author, Some("Stanley".to_string()));
        assert_eq!(q.lang, Some(Language::Rust));
        assert_eq!(q.limit, 5);
    }

    #[test]
    fn test_parse_with_unsupported_filter() {
        let q = SearchQuery::parse("parser unknown:value");
        assert_eq!(q.raw_query, "parser unknown:value");
    }
}
