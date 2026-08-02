// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Serial JSON-line source (Arduino Nano 33 BLE path).
//!
//! Feature: `serial`.
//!
//! Reads one JSON object per line at the configured baud rate (default 115200).

use std::{
    io::{
        BufRead,
        BufReader,
    },
    time::{
        Duration,
        SystemTime,
    },
};

use serialport::SerialPort;

use crate::{
    error::{
        EmbedError,
        Result,
    },
    protocol::{
        enrich,
        parse_json_line,
    },
    source::StreamSource,
    types::{
        SourceKind,
        StreamSample,
    },
};

/// Configuration for [`SerialSource`].
#[derive(Debug, Clone)]
pub struct SerialConfig {
    /// Device path (e.g. `/dev/ttyACM0`, `COM3`).
    pub port: String,
    /// Baud rate (Arduino sketch uses 115200).
    pub baud: u32,
    /// Read timeout for the underlying port.
    pub timeout: Duration,
    /// Subject id attached to every sample (`sid`).
    pub sid: String,
    /// Skip non-JSON noise lines (boot banners) instead of erroring.
    pub skip_bad_lines: bool,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyACM0".into(),
            baud: 115_200,
            timeout: Duration::from_millis(1000),
            sid: "S001".into(),
            skip_bad_lines: true,
        }
    }
}

/// Blocking serial reader that yields enriched [`StreamSample`]s.
pub struct SerialSource {
    reader: BufReader<Box<dyn SerialPort>>,
    line_buf: String,
    cfg: SerialConfig,
}

impl SerialSource {
    /// Open the serial port with the given config.
    pub fn open(cfg: SerialConfig) -> Result<Self> {
        if cfg.port.trim().is_empty() {
            return Err(EmbedError::InvalidParameter("serial port is empty".into()));
        }
        let port = serialport::new(&cfg.port, cfg.baud)
            .timeout(cfg.timeout)
            .open()
            .map_err(|e| EmbedError::Serial(format!("{}: {e}", cfg.port)))?;

        Ok(Self {
            reader: BufReader::new(port),
            line_buf: String::with_capacity(256),
            cfg,
        })
    }

    /// Open with defaults, overriding port and `sid`.
    pub fn open_port(port: impl Into<String>, sid: impl Into<String>) -> Result<Self> {
        Self::open(SerialConfig {
            port: port.into(),
            sid: sid.into(),
            ..Default::default()
        })
    }
}

impl StreamSource for SerialSource {
    fn next_sample(&mut self) -> Result<Option<StreamSample>> {
        loop {
            self.line_buf.clear();
            match self.reader.read_line(&mut self.line_buf) {
                Ok(0) => return Ok(None), // EOF
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // No full line yet; keep waiting.
                    continue;
                }
                Err(e) => return Err(EmbedError::Io(e)),
            }

            match parse_json_line(&self.line_buf) {
                Ok(None) => continue,
                Ok(Some(raw)) => {
                    let mut s = enrich(raw, self.cfg.sid.clone(), SourceKind::Arduino);
                    s.host_ts = Some(SystemTime::now());
                    // If device didn't set source, force Arduino.
                    if s.source == SourceKind::Unknown {
                        s.source = SourceKind::Arduino;
                    }
                    return Ok(Some(s));
                }
                Err(e) => {
                    if self.cfg.skip_bad_lines {
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}
