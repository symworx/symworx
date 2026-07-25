// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! LoadSym view enums, catalog rows, metrics fields.

/// LoadSym internal views (selector "home" inside the LoadSym workflow)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadSymView {
    #[default]
    List,
    Workout,
    Calendar,
    Optimization,
    /// Catalog library: per-ride LOADsym metrics table.
    Metrics,
}

/// Workout analyzer data streams (FIT/CSV channels).
/// Toggle with keys `1`–`5`; open panels share height equally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WorkoutStream {
    Power = 0,
    HeartRate = 1,
    Speed = 2,
    Cadence = 3,
    Elevation = 4,
}

impl WorkoutStream {
    pub const ALL: [WorkoutStream; 5] = [
        WorkoutStream::Power,
        WorkoutStream::HeartRate,
        WorkoutStream::Speed,
        WorkoutStream::Cadence,
        WorkoutStream::Elevation,
    ];
    pub const COUNT: usize = 5;

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::Power),
            1 => Some(Self::HeartRate),
            2 => Some(Self::Speed),
            3 => Some(Self::Cadence),
            4 => Some(Self::Elevation),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key_digit(self) -> char {
        match self {
            Self::Power => '1',
            Self::HeartRate => '2',
            Self::Speed => '3',
            Self::Cadence => '4',
            Self::Elevation => '5',
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::HeartRate => "hr",
            Self::Speed => "speed",
            Self::Cadence => "cad",
            Self::Elevation => "elev",
        }
    }

    pub fn chart_title(self) -> &'static str {
        match self {
            Self::Power => "Power (W)",
            Self::HeartRate => "Heart rate (bpm)",
            Self::Speed => "Speed (km/h)",
            Self::Cadence => "Cadence (rpm)",
            Self::Elevation => "Elevation (m)",
        }
    }

    /// Whether this channel has usable samples on the activity.
    pub fn present_on(self, act: &symworx_io::ActivityData) -> bool {
        match self {
            Self::Power => act.has_power(),
            Self::HeartRate => act.has_hr(),
            Self::Speed => act.has_speed(),
            Self::Cadence => act.has_cadence(),
            Self::Elevation => act.has_altitude(),
        }
    }

    /// Series values for charting (display units).
    pub fn series(self, act: &symworx_io::ActivityData) -> Vec<f64> {
        match self {
            Self::Power => act.power_series(),
            Self::HeartRate => act.hr_series(),
            Self::Speed => act.speed_kmh_series(),
            Self::Cadence => act.cadence_series(),
            Self::Elevation => act.altitude_series_m(),
        }
    }
}

/// One ride file for calendar daily list.
#[derive(Debug, Clone)]
pub struct CatalogRideRow {
    pub ride_date: String,
    pub source_file: String,
    pub tss: f64,
    pub duration_s: f64,
    pub np_w: Option<f64>,
}

/// One activity row for Metrics / Library table (from catalog).
#[derive(Debug, Clone)]
pub struct ActivityMetricsUiRow {
    pub id: i64,
    pub ride_date: String,
    pub source_file: String,
    pub duration_s: f64,
    pub sport: Option<String>,
    pub avg_power_w: Option<f64>,
    pub max_power_w: Option<f64>,
    pub np_w: Option<f64>,
    pub intensity_factor: Option<f64>,
    pub tss: Option<f64>,
    pub total_work_kj: Option<f64>,
    pub avg_hr_bpm: Option<f64>,
    pub max_hr_bpm: Option<f64>,
    pub ftp_used_w: Option<f64>,
}

/// Numeric fields plottable on Metrics trends / bi-plots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsField {
    Tss,
    Np,
    AvgPower,
    DurationMin,
    AvgHr,
    If,
    WorkKj,
    MaxPower,
}

impl MetricsField {
    pub const ALL: [MetricsField; 8] = [
        MetricsField::Tss,
        MetricsField::Np,
        MetricsField::AvgPower,
        MetricsField::DurationMin,
        MetricsField::AvgHr,
        MetricsField::If,
        MetricsField::WorkKj,
        MetricsField::MaxPower,
    ];

    /// Short label (status / compact UI).
    pub fn label(self) -> &'static str {
        match self {
            Self::Tss => "TSLi",
            Self::Np => "SEPi",
            Self::AvgPower => "avg W",
            Self::DurationMin => "dur min",
            Self::AvgHr => "avg HR",
            Self::If => "SRIi",
            Self::WorkKj => "work kJ",
            Self::MaxPower => "max W",
        }
    }

    /// Axis / chart title: full name with LOADsym acronym when applicable.
    pub fn axis_label(self) -> &'static str {
        match self {
            Self::Tss => "Training stress (TSLi)",
            Self::Np => "Normalized power (SEPi)",
            Self::AvgPower => "Average power (W)",
            Self::DurationMin => "Duration (min)",
            Self::AvgHr => "Average heart rate (bpm)",
            Self::If => "Intensity factor (SRIi)",
            Self::WorkKj => "Total work (kJ)",
            Self::MaxPower => "Max power (W)",
        }
    }

    pub fn short_key(self) -> char {
        match self {
            Self::Tss => '1',
            Self::Np => '2',
            Self::AvgPower => '3',
            Self::DurationMin => '4',
            Self::AvgHr => '5',
            Self::If => '6',
            Self::WorkKj => '7',
            Self::MaxPower => '8',
        }
    }

    pub fn from_digit(d: char) -> Option<Self> {
        match d {
            '1' => Some(Self::Tss),
            '2' => Some(Self::Np),
            '3' => Some(Self::AvgPower),
            '4' => Some(Self::DurationMin),
            '5' => Some(Self::AvgHr),
            '6' => Some(Self::If),
            '7' => Some(Self::WorkKj),
            '8' => Some(Self::MaxPower),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        let all = Self::ALL;
        let i = all.iter().position(|&f| f == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    pub fn value(self, row: &ActivityMetricsUiRow) -> Option<f64> {
        let v = match self {
            Self::Tss => row.tss,
            Self::Np => row.np_w,
            Self::AvgPower => row.avg_power_w,
            Self::DurationMin => Some(row.duration_s / 60.0),
            Self::AvgHr => row.avg_hr_bpm,
            Self::If => row.intensity_factor,
            Self::WorkKj => row.total_work_kj,
            Self::MaxPower => row.max_power_w,
        }?;
        if !v.is_finite() {
            return None;
        }
        // LOADsym metrics are non-negative for plotting; floor noise / bad signs.
        Some(v.max(0.0))
    }
}

/// Chart mode inside Metrics view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricsChartMode {
    #[default]
    Trend,
    Biplot,
}

/// One ISO-ish week aggregate for calendar weekly list.
#[derive(Debug, Clone)]
pub struct WeeklyLoadRow {
    /// Monday of the week (`YYYY-MM-DD`)
    pub week_start: String,
    pub total_tss: f64,
    pub ride_count: i64,
    pub day_count: usize,
    /// Inclusive indices into `daily_loads` covered by this week
    pub day_index_lo: usize,
    pub day_index_hi: usize,
}
