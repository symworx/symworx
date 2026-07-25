// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Small pure helpers shared across LoadSym views.

pub fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
