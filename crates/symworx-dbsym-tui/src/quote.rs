// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Quote a SQL identifier for SQLite and Postgres (`"` doubled).

/// Wrap `name` in double quotes, doubling any quote inside it.
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::quote_ident;

    #[test]
    fn quotes_plain_and_embedded_quotes() {
        assert_eq!(quote_ident("subjects"), "\"subjects\"");
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
    }
}
