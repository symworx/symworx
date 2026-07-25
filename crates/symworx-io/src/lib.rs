// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! # symworx-io
//!
//! I/O for biosignals and activity data (CSV, Parquet, IBI, FIT, optional email / Polar).
//! Domain crates and the TUI load/save through this crate (see workspace I/O rule).

#![warn(missing_docs)]

// Modules
/// CSV module and related utilities.
pub mod csv;

/// GBD module and related utilities.
pub mod gbd;

/// IBI module and related utilities.
pub mod ibi;

#[cfg(feature = "parquet")]
/// Parquet module and related utilities.
pub mod parquet;

/// Exercise / activity file support (FIT + other formats; sport-agnostic).
/// Enabled via `fit` feature (and future `gpx` etc).
pub mod activity;

/// Tabular numeric CSV for StatsSym and general analysis.
pub mod table;

/// Email input support (IMAP fetching of .fit files, e.g. from SRM PC8 emails).
/// Enabled via the `email` feature.
#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "email")]
pub use email::{
    ImapConfig,
    SYMLOAD_IMAP_HOST_ENV,
    SYMLOAD_IMAP_MAILBOX_ENV,
    SYMLOAD_IMAP_PORT_ENV,
    fetch_fit_attachments,
    fetch_fit_attachments_with_config,
    fetch_srm_fit_attachments,
};

/// Polar AccessLink (OAuth + exercise FIT download). Enabled via the `polar` feature.
#[cfg(feature = "polar")]
pub mod polar;

#[cfg(feature = "polar")]
pub use polar::{
    DEFAULT_MEMBER_ID,
    DEFAULT_REDIRECT_URI,
    POLAR_ACCESS_TOKEN_ENV,
    POLAR_CLIENT_ID_ENV,
    POLAR_CLIENT_SECRET_ENV,
    POLAR_MEMBER_ID_ENV,
    POLAR_REDIRECT_URI_ENV,
    POLAR_USER_ID_ENV,
    PolarCredentials,
    PolarExerciseSummary,
    PolarFetchReport,
    PolarTokenFile,
    default_polar_raw_dir,
    default_token_path,
    download_exercise_fit,
    external_id_from_polar_filename,
    fetch_exercise_fits,
    list_exercises,
    load_token_file,
    polar_fit_filename,
    register_user,
    run_oauth_flow,
    save_token_file,
};

/// Personal archive path helpers (`~/velofit`) and activity file discovery.
pub mod paths;

/// Additional traits used in io.
pub mod traits;

// Re-exports
pub use activity::{
    ActivityData,
    load_activity,
    load_activity_power_series,
};
pub use csv::{
    CsvReader,
    CsvWriter,
};
pub use gbd::{
    GbdReader,
    GbdTable,
};
pub use ibi::{
    IbiRecord,
    read_ibi,
};
#[cfg(feature = "parquet")]
pub use parquet::ParquetReader;
pub use paths::{
    ActivityFileEntry,
    VELOFIT_HOME_ENV,
    default_activity_search_dirs,
    default_velofit_db,
    default_velofit_inbox,
    default_velofit_polar,
    default_velofit_raw,
    default_velofit_root,
    discover_activity_files,
    find_newest_activity_path,
};
use symworx_error::SymError;
pub use table::{
    TableData,
    load_numeric_table,
    write_columns_csv,
    write_numeric_table,
};
use traits::SymReader;

/// Parent load function.
///
/// Auto-detect the file format (csv, parquet) and read in the file.
pub fn load_any(path: &str) -> Result<Vec<Vec<f64>>, SymError> {
    if path.ends_with(".csv") {
        CsvReader::read(path)
    } else if path.ends_with(".parquet") {
        #[cfg(feature = "parquet")]
        return ParquetReader::read(path);
        #[cfg(not(feature = "parquet"))]
        return Err(SymError::UnsupportedFormat(path.into()));
    } else {
        Err(SymError::UnsupportedFormat(path.into()))
    }
}

/// symworx-io version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
