// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Built-in attribute presets (composable). Blank install is allowed.

use rusqlite::Connection;

use crate::error::{
    DbSymError,
    Result,
};

/// One seeded attribute.
#[derive(Debug, Clone, Copy)]
pub struct SeedAttr {
    /// Unique attribute name.
    pub name: &'static str,
    /// `numeric`, `text`, or `date`.
    pub kind: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// Exercise / human performance demographics and session tags.
///
/// Body size uses workspace units: height in meters, mass in kilograms.
/// Assay-specific fields (hormones, plate IDs, …) belong in the study schema, not here.
pub const EXERCISE_SCIENCE: &[SeedAttr] = &[
    SeedAttr {
        name: "age",
        kind: "numeric",
        description: "Participant age in years",
    },
    SeedAttr {
        name: "sex",
        kind: "text",
        description: "Biological sex",
    },
    SeedAttr {
        name: "height_m",
        kind: "numeric",
        description: "Height in meters",
    },
    SeedAttr {
        name: "weight_kg",
        kind: "numeric",
        description: "Body mass in kilograms",
    },
    SeedAttr {
        name: "vo2_max",
        kind: "numeric",
        description: "Maximal oxygen uptake (mL/kg/min)",
    },
    SeedAttr {
        name: "condition",
        kind: "text",
        description: "Session condition (e.g. rest, exercise)",
    },
    SeedAttr {
        name: "visit",
        kind: "text",
        description: "Visit or session label",
    },
    SeedAttr {
        name: "session_date",
        kind: "date",
        description: "Session date (YYYY-MM-DD)",
    },
];

const PRESETS: &[(&str, &[SeedAttr])] = &[("exercise_science", EXERCISE_SCIENCE)];

/// Names of shipped presets, stable order.
pub fn available_presets() -> Vec<&'static str> {
    PRESETS.iter().map(|(name, _)| *name).collect()
}

fn attrs_for(name: &str) -> Result<&'static [SeedAttr]> {
    PRESETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, attrs)| *attrs)
        .ok_or_else(|| {
            let known = available_presets().join(", ");
            DbSymError::UnknownPreset(name.to_string(), known)
        })
}

/// Insert attributes for each named preset (`INSERT OR IGNORE` on name).
pub fn apply_presets(conn: &Connection, presets: &[String]) -> Result<usize> {
    let mut inserted = 0usize;
    for name in presets {
        let attrs = attrs_for(name)?;
        for a in attrs {
            let n = conn.execute(
                "INSERT OR IGNORE INTO attributes (name, type, description, preset) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![a.name, a.kind, a.description, name.as_str()],
            )?;
            inserted += n;
        }
    }
    Ok(inserted)
}
