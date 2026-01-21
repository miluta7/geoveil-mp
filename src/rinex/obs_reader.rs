//! RINEX observation file parser.
//!
//! Supports RINEX versions 2.xx, 3.xx, and 4.xx observation files.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use flate2::read::GzDecoder;

use super::types::*;
use crate::utils::{Ecef, Epoch, Error, Result};

/// RINEX observation file reader
pub struct RinexObsReader {
    /// Allow reading gzip compressed files
    allow_compressed: bool,
}

impl Default for RinexObsReader {
    fn default() -> Self {
        Self::new()
    }
}

impl RinexObsReader {
    /// Create a new reader
    pub fn new() -> Self {
        Self {
            allow_compressed: true,
        }
    }

    /// Read RINEX observation file
    pub fn read<P: AsRef<Path>>(&self, path: P) -> Result<ObservationData> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(Error::FileNotFound(path.to_path_buf()));
        }

        let file = File::open(path)?;
        
        // Check for compression
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        if extension == "gz" && self.allow_compressed {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            self.parse_reader(reader)
        } else {
            let reader = BufReader::new(file);
            self.parse_reader(reader)
        }
    }

    /// Parse from any reader
    fn parse_reader<R: BufRead>(&self, reader: R) -> Result<ObservationData> {
        let mut lines = reader.lines().enumerate();
        
        // Parse header
        let header = self.parse_header(&mut lines)?;
        
        // Parse epochs based on version
        let epochs = if header.version.is_v2() {
            self.parse_epochs_v2(&mut lines, &header)?
        } else {
            self.parse_epochs_v3v4(&mut lines, &header)?
        };

        Ok(ObservationData { header, epochs })
    }

    /// Parse RINEX header
    fn parse_header<I>(&self, lines: &mut I) -> Result<Header>
    where
        I: Iterator<Item = (usize, std::io::Result<String>)>,
    {
        let mut header = Header::default();
        header.file_type = 'O';

        for (_line_num, line_result) in lines {
            let line = line_result.map_err(|e| Error::Io(e))?;
            // Strip Windows CR if present
            let line = line.trim_end_matches('\r');
            
            // Get label (columns 61-80)
            let label = if line.len() >= 60 {
                line[60..].trim()
            } else {
                continue;
            };

            match label {
                "RINEX VERSION / TYPE" => {
                    if line.len() >= 20 {
                        let version_str = line[0..9].trim();
                        header.version = RinexVersion::parse(version_str)
                            .ok_or_else(|| Error::UnsupportedRinexVersion(version_str.to_string()))?;
                        
                        if line.len() >= 20 {
                            header.file_type = line.chars().nth(20).unwrap_or('O');
                        }
                        
                        if line.len() >= 40 {
                            header.satellite_system = GnssSystem::from_char(
                                line.chars().nth(40).unwrap_or('M')
                            );
                        }
                    }
                }
                "PGM / RUN BY / DATE" => {
                    header.program = line[0..20.min(line.len())].trim().to_string();
                    if line.len() >= 40 {
                        header.run_by = line[20..40.min(line.len())].trim().to_string();
                    }
                    if line.len() >= 60 {
                        header.date = line[40..60.min(line.len())].trim().to_string();
                    }
                }
                "MARKER NAME" => {
                    header.marker_name = line[0..60.min(line.len())].trim().to_string();
                }
                "MARKER NUMBER" => {
                    header.marker_number = line[0..20.min(line.len())].trim().to_string();
                }
                "MARKER TYPE" => {
                    header.marker_type = Some(line[0..20.min(line.len())].trim().to_string());
                }
                "OBSERVER / AGENCY" => {
                    header.observer = line[0..20.min(line.len())].trim().to_string();
                    if line.len() >= 60 {
                        header.agency = line[20..60.min(line.len())].trim().to_string();
                    }
                }
                "REC # / TYPE / VERS" => {
                    header.receiver_number = line[0..20.min(line.len())].trim().to_string();
                    if line.len() >= 40 {
                        header.receiver_type = line[20..40.min(line.len())].trim().to_string();
                    }
                    if line.len() >= 60 {
                        header.receiver_version = line[40..60.min(line.len())].trim().to_string();
                    }
                }
                "ANT # / TYPE" => {
                    header.antenna_number = line[0..20.min(line.len())].trim().to_string();
                    if line.len() >= 40 {
                        header.antenna_type = line[20..40.min(line.len())].trim().to_string();
                    }
                }
                "APPROX POSITION XYZ" => {
                    let parts: Vec<f64> = line[0..60.min(line.len())]
                        .split_whitespace()
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if parts.len() >= 3 {
                        header.approx_position = Some(Ecef::new(parts[0], parts[1], parts[2]));
                    }
                }
                "ANTENNA: DELTA H/E/N" => {
                    let parts: Vec<f64> = line[0..60.min(line.len())]
                        .split_whitespace()
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if parts.len() >= 3 {
                        header.antenna_delta = Some([parts[0], parts[1], parts[2]]);
                    }
                }
                "SYS / # / OBS TYPES" => {
                    // RINEX 3/4 observation types
                    self.parse_obs_types_v3(&line, &mut header)?;
                }
                "# / TYPES OF OBSERV" => {
                    // RINEX 2 observation types
                    self.parse_obs_types_v2(&line, &mut header)?;
                }
                "TIME OF FIRST OBS" => {
                    if let Ok(epoch) = self.parse_header_epoch(&line[0..43.min(line.len())]) {
                        header.time_first_obs = Some(epoch);
                    }
                    if line.len() >= 51 {
                        header.time_system = line[48..51.min(line.len())].trim().to_string();
                    }
                }
                "TIME OF LAST OBS" => {
                    if let Ok(epoch) = self.parse_header_epoch(&line[0..43.min(line.len())]) {
                        header.time_last_obs = Some(epoch);
                    }
                }
                "INTERVAL" => {
                    if let Ok(interval) = line[0..10.min(line.len())].trim().parse::<f64>() {
                        header.interval = Some(interval);
                    }
                }
                "LEAP SECONDS" => {
                    if let Ok(leap) = line[0..6.min(line.len())].trim().parse::<i32>() {
                        header.leap_seconds = Some(leap);
                    }
                }
                "# OF SATELLITES" => {
                    if let Ok(num) = line[0..6.min(line.len())].trim().parse::<u32>() {
                        header.num_satellites = Some(num);
                    }
                }
                "GLONASS SLOT / FRQ #" => {
                    self.parse_glonass_slot_frq(&line, &mut header);
                }
                "GLONASS COD/PHS/BIS" => {
                    self.parse_glonass_cod_phs_bis(&line, &mut header);
                }
                "COMMENT" => {
                    header.comments.push(line[0..60.min(line.len())].trim().to_string());
                }
                "END OF HEADER" => {
                    break;
                }
                _ => {}
            }
        }

        Ok(header)
    }

    /// Parse RINEX 3/4 observation types
    fn parse_obs_types_v3(&self, line: &str, header: &mut Header) -> Result<()> {
        if line.len() < 7 {
            return Ok(());
        }

        let system_char = line.chars().next().unwrap_or(' ');
        let system = match GnssSystem::from_char(system_char) {
            Some(s) => s,
            None => return Ok(()),
        };

        // Parse observation count
        let _num_obs: usize = line[3..6].trim().parse().unwrap_or(0);

        // Parse observation codes (starting at column 7, 4 chars each)
        let obs_str = &line[7..line.len().min(60)];
        let codes: Vec<SignalCode> = obs_str
            .as_bytes()
            .chunks(4)
            .filter_map(|chunk| {
                let s = std::str::from_utf8(chunk).ok()?.trim();
                if s.len() >= 3 {
                    SignalCode::parse(s)
                } else {
                    None
                }
            })
            .collect();

        // Append or create entry for this system
        header.obs_types
            .entry(system)
            .or_insert_with(Vec::new)
            .extend(codes);

        Ok(())
    }

    /// Parse RINEX 2 observation types
    fn parse_obs_types_v2(&self, line: &str, header: &mut Header) -> Result<()> {
        // First line has count, continuation lines don't
        let start = if line[0..6].trim().parse::<usize>().is_ok() { 6 } else { 0 };
        
        let obs_str = &line[start..line.len().min(60)];
        for chunk in obs_str.as_bytes().chunks(6) {
            if let Ok(s) = std::str::from_utf8(chunk) {
                let obs = s.trim();
                if !obs.is_empty() {
                    header.obs_types_v2.push(obs.to_string());
                }
            }
        }

        Ok(())
    }

    /// Parse epoch from header (TIME OF FIRST/LAST OBS)
    fn parse_header_epoch(&self, s: &str) -> Result<Epoch> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() < 6 {
            return Err(Error::InvalidEpoch(s.to_string()));
        }

        Ok(Epoch {
            year: parts[0].parse()?,
            month: parts[1].parse()?,
            day: parts[2].parse()?,
            hour: parts[3].parse()?,
            minute: parts[4].parse()?,
            second: parts[5].parse()?,
        })
    }

    /// Parse GLONASS slot/frequency mapping
    fn parse_glonass_slot_frq(&self, line: &str, header: &mut Header) {
        // Format: num slots, then slot/frq pairs
        let content = &line[0..60.min(line.len())];
        let parts: Vec<&str> = content.split_whitespace().collect();
        
        let mut i = 1; // Skip count
        while i + 1 < parts.len() {
            if let (Ok(slot), Ok(frq)) = (
                parts[i].trim_start_matches('R').parse::<u32>(),
                parts[i + 1].parse::<i8>(),
            ) {
                header.glonass_slot_frq.insert(slot, frq);
            }
            i += 2;
        }
    }

    /// Parse GLONASS code/phase bias
    fn parse_glonass_cod_phs_bis(&self, line: &str, header: &mut Header) {
        let mut biases = header.glonass_cod_phs_bis.take().unwrap_or_default();
        
        // Parse bias values (format: code bias pairs)
        let content = &line[0..60.min(line.len())];
        let parts: Vec<&str> = content.split_whitespace().collect();
        
        let mut i = 0;
        while i + 1 < parts.len() {
            if let Ok(bias) = parts[i + 1].parse::<f64>() {
                biases.insert(parts[i].to_string(), bias);
            }
            i += 2;
        }
        
        if !biases.is_empty() {
            header.glonass_cod_phs_bis = Some(biases);
        }
    }

    /// Parse epochs for RINEX 3/4
    fn parse_epochs_v3v4<I>(&self, lines: &mut I, header: &Header) -> Result<Vec<EpochObservations>>
    where
        I: Iterator<Item = (usize, std::io::Result<String>)>,
    {
        let mut epochs = Vec::new();

        while let Some((line_num, line_result)) = lines.next() {
            let line = line_result.map_err(|e| Error::Io(e))?;
            // Strip Windows CR
            let line = line.trim_end_matches('\r');
            
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Epoch record starts with '>'
            if !line.starts_with('>') {
                continue;
            }

            // Parse epoch header
            let epoch_data = self.parse_epoch_header_v3(line, line_num)?;
            let mut epoch_obs = EpochObservations::new(epoch_data.0);
            epoch_obs.flag = epoch_data.1;
            epoch_obs.clock_offset = epoch_data.3;
            let num_sats = epoch_data.2;

            // Parse satellite observations
            for _ in 0..num_sats {
                if let Some((_, sat_line_result)) = lines.next() {
                    let sat_line = sat_line_result.map_err(|e| Error::Io(e))?;
                    // Strip Windows CR
                    let sat_line = sat_line.trim_end_matches('\r');
                    if let Some((sat, obs)) = self.parse_satellite_obs_v3(sat_line, header)? {
                        epoch_obs.satellites.insert(sat, obs);
                    }
                }
            }

            epochs.push(epoch_obs);
        }

        Ok(epochs)
    }

    /// Parse RINEX 3/4 epoch header line
    fn parse_epoch_header_v3(&self, line: &str, line_num: usize) -> Result<(Epoch, u8, usize, Option<f64>)> {
        if line.len() < 32 {
            return Err(Error::rinex_parse(line_num, "Epoch line too short"));
        }

        // Format: > YYYY MM DD HH MM SS.SSSSSSS  FLAG  NUM_SATS  [CLOCK_OFFSET]
        // Helper to safely parse with default
        fn parse_or<T: std::str::FromStr>(s: &str, default: T) -> T {
            s.trim().parse().unwrap_or(default)
        }
        
        // Safe substring extraction
        fn safe_substr(s: &str, start: usize, end: usize) -> &str {
            if start >= s.len() {
                ""
            } else if end > s.len() {
                &s[start..]
            } else {
                &s[start..end]
            }
        }
        
        let year: i32 = parse_or(safe_substr(line, 2, 6), 2000);
        let month: u32 = parse_or(safe_substr(line, 7, 9), 1);
        let day: u32 = parse_or(safe_substr(line, 10, 12), 1);
        let hour: u32 = parse_or(safe_substr(line, 13, 15), 0);
        let minute: u32 = parse_or(safe_substr(line, 16, 18), 0);
        let second: f64 = parse_or(safe_substr(line, 19, 30), 0.0);
        
        let flag: u8 = parse_or(safe_substr(line, 31, 32), 0);
        let num_sats: usize = parse_or(safe_substr(line, 32, 35), 0);
        
        let clock_offset = if line.len() >= 56 {
            safe_substr(line, 41, 56).trim().parse().ok()
        } else {
            None
        };

        let epoch = Epoch::new(year, month, day, hour, minute, second);
        
        Ok((epoch, flag, num_sats, clock_offset))
    }

    /// Parse satellite observation line (RINEX 3/4)
    fn parse_satellite_obs_v3(
        &self,
        line: &str,
        header: &Header,
    ) -> Result<Option<(Satellite, SatelliteObservations)>> {
        if line.len() < 3 {
            return Ok(None);
        }

        // Get satellite ID
        let sat_id = &line[0..3];
        let satellite = match Satellite::parse(sat_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Get observation types for this system
        let obs_types = match header.obs_types.get(&satellite.system) {
            Some(types) => types,
            None => return Ok(None),
        };

        let mut observations = HashMap::new();
        let obs_data = &line[3..];

        // Each observation is 16 characters: 14 for value, 1 for LLI, 1 for SSI
        for (i, obs_type) in obs_types.iter().enumerate() {
            let start = i * 16;
            let end = start + 16;
            
            if start >= obs_data.len() {
                break;
            }

            let obs_str = if end <= obs_data.len() {
                &obs_data[start..end]
            } else {
                &obs_data[start..]
            };

            // Parse value (columns 0-13)
            let value_str = if obs_str.len() >= 14 {
                obs_str[0..14].trim()
            } else {
                obs_str.trim()
            };

            if value_str.is_empty() {
                continue;
            }

            let value: f64 = match value_str.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Parse LLI (column 14) and SSI (column 15)
            let lli = if obs_str.len() >= 15 {
                let c = obs_str.chars().nth(14).unwrap_or(' ');
                if c.is_ascii_digit() {
                    Some(c.to_digit(10).unwrap() as u8)
                } else {
                    None
                }
            } else {
                None
            };

            let ssi = if obs_str.len() >= 16 {
                let c = obs_str.chars().nth(15).unwrap_or(' ');
                if c.is_ascii_digit() {
                    Some(c.to_digit(10).unwrap() as u8)
                } else {
                    None
                }
            } else {
                None
            };

            observations.insert(obs_type.clone(), ObservationValue::with_flags(value, lli, ssi));
        }

        Ok(Some((satellite, observations)))
    }

    /// Parse epochs for RINEX 2
    fn parse_epochs_v2<I>(&self, lines: &mut I, header: &Header) -> Result<Vec<EpochObservations>>
    where
        I: Iterator<Item = (usize, std::io::Result<String>)>,
    {
        let mut epochs = Vec::new();
        let mut current_lines: Vec<String> = Vec::new();
        let mut epoch_started = false;

        for (_line_num, line_result) in lines {
            let line = line_result.map_err(|e| Error::Io(e))?;
            // Strip Windows CR
            let line = line.trim_end_matches('\r').to_string();

            // Check for epoch header (starts with space and year)
            if line.len() >= 26 && line.chars().nth(0) == Some(' ') {
                // Check if it looks like an epoch header
                let first_part = &line[1..3];
                if let Ok(_year) = first_part.trim().parse::<i32>() {
                    // Process previous epoch
                    if epoch_started && !current_lines.is_empty() {
                        if let Ok(epoch) = self.parse_epoch_block_v2(&current_lines, header) {
                            epochs.push(epoch);
                        }
                        current_lines.clear();
                    }
                    epoch_started = true;
                }
            }

            if epoch_started {
                current_lines.push(line);
            }
        }

        // Process last epoch
        if !current_lines.is_empty() {
            if let Ok(epoch) = self.parse_epoch_block_v2(&current_lines, header) {
                epochs.push(epoch);
            }
        }

        Ok(epochs)
    }

    /// Parse a complete RINEX 2 epoch block
    fn parse_epoch_block_v2(&self, lines: &[String], header: &Header) -> Result<EpochObservations> {
        if lines.is_empty() {
            return Err(Error::RinexParse {
                line: 0,
                message: "Empty epoch block".to_string(),
            });
        }

        let epoch_line = &lines[0];
        
        // Helper for safe parsing
        fn parse_or<T: std::str::FromStr>(s: &str, default: T) -> T {
            s.trim().parse().unwrap_or(default)
        }
        
        fn safe_substr(s: &str, start: usize, end: usize) -> &str {
            if start >= s.len() { "" } 
            else if end > s.len() { &s[start..] } 
            else { &s[start..end] }
        }
        
        // Parse epoch header
        // Format: yy mm dd hh mm ss.sssssss  flag num_sats sat_list...
        let year: i32 = parse_or(safe_substr(epoch_line, 1, 3), 0);
        let month: u32 = parse_or(safe_substr(epoch_line, 4, 6), 1);
        let day: u32 = parse_or(safe_substr(epoch_line, 7, 9), 1);
        let hour: u32 = parse_or(safe_substr(epoch_line, 10, 12), 0);
        let minute: u32 = parse_or(safe_substr(epoch_line, 13, 15), 0);
        let second: f64 = parse_or(safe_substr(epoch_line, 15, 26), 0.0);
        
        // Convert 2-digit year
        let year = if year < 80 { 2000 + year } else { 1900 + year };
        
        let epoch = Epoch::new(year, month, day, hour, minute, second);
        let mut epoch_obs = EpochObservations::new(epoch);

        let flag: u8 = parse_or(safe_substr(epoch_line, 26, 29), 0);
        epoch_obs.flag = flag;
        
        let num_sats: usize = parse_or(safe_substr(epoch_line, 29, 32), 0);

        // Parse satellite list from epoch header (and continuation lines if needed)
        let mut sat_list = Vec::new();
        let mut sat_str = if epoch_line.len() > 32 { epoch_line[32..].to_string() } else { String::new() };
        
        // Satellites are in 3-character groups
        let sats_per_line = 12;
        let mut lines_for_sats = 1;
        
        if num_sats > sats_per_line {
            lines_for_sats = (num_sats + sats_per_line - 1) / sats_per_line;
        }

        // Parse satellites from header continuation lines
        for i in 1..lines_for_sats {
            if i < lines.len() && lines[i].len() > 32 {
                sat_str.push_str(&lines[i][32..]);
            }
        }

        // Extract satellite IDs
        for i in 0..num_sats {
            let start = i * 3;
            let end = start + 3;
            if end <= sat_str.len() {
                let sat_id = &sat_str[start..end];
                if let Some(sat) = self.parse_sat_id_v2(sat_id) {
                    sat_list.push(sat);
                }
            }
        }

        // Parse observation data
        let num_obs_types = header.obs_types_v2.len();
        let obs_per_line = 5;
        let lines_per_sat = (num_obs_types + obs_per_line - 1) / obs_per_line;
        
        let data_start_line = lines_for_sats;
        
        for (sat_idx, sat) in sat_list.iter().enumerate() {
            let mut observations = HashMap::new();
            let sat_line_start = data_start_line + sat_idx * lines_per_sat;
            
            for (obs_idx, obs_code) in header.obs_types_v2.iter().enumerate() {
                let line_offset = obs_idx / obs_per_line;
                let col_offset = (obs_idx % obs_per_line) * 16;
                
                let line_idx = sat_line_start + line_offset;
                if line_idx >= lines.len() {
                    break;
                }
                
                let obs_line = &lines[line_idx];
                if col_offset + 14 > obs_line.len() {
                    continue;
                }
                
                let value_str = obs_line[col_offset..col_offset + 14].trim();
                if value_str.is_empty() {
                    continue;
                }
                
                if let Ok(value) = value_str.parse::<f64>() {
                    // Convert V2 code to V3 SignalCode
                    if let Some(signal_code) = self.convert_v2_to_v3_code(obs_code, &sat.system) {
                        let lli = if col_offset + 15 <= obs_line.len() {
                            obs_line.chars().nth(col_offset + 14)
                                .and_then(|c| c.to_digit(10).map(|d| d as u8))
                        } else {
                            None
                        };
                        
                        let ssi = if col_offset + 16 <= obs_line.len() {
                            obs_line.chars().nth(col_offset + 15)
                                .and_then(|c| c.to_digit(10).map(|d| d as u8))
                        } else {
                            None
                        };
                        
                        observations.insert(signal_code, ObservationValue::with_flags(value, lli, ssi));
                    }
                }
            }
            
            if !observations.is_empty() {
                epoch_obs.satellites.insert(*sat, observations);
            }
        }

        Ok(epoch_obs)
    }

    /// Parse RINEX 2 satellite ID
    fn parse_sat_id_v2(&self, s: &str) -> Option<Satellite> {
        let s = s.trim();
        if s.len() < 2 {
            return None;
        }
        
        let first_char = s.chars().next()?;
        let (system, prn_str) = if first_char.is_ascii_digit() {
            // GPS assumed if no system letter
            (GnssSystem::Gps, s)
        } else {
            let system = GnssSystem::from_char(first_char)?;
            (system, &s[1..])
        };
        
        let prn: u32 = prn_str.trim().parse().ok()?;
        Some(Satellite::new(system, prn))
    }

    /// Convert RINEX 2 observation code to RINEX 3 SignalCode
    fn convert_v2_to_v3_code(&self, v2_code: &str, system: &GnssSystem) -> Option<SignalCode> {
        // Common conversions
        let (obs_type, band, attr) = match v2_code.to_uppercase().as_str() {
            "C1" => (ObservationType::Code, 1, 'C'),
            "P1" => (ObservationType::Code, 1, 'P'),
            "L1" => (ObservationType::Phase, 1, 'C'),
            "D1" => (ObservationType::Doppler, 1, 'C'),
            "S1" => (ObservationType::Snr, 1, 'C'),
            "C2" => (ObservationType::Code, 2, 'C'),
            "P2" => (ObservationType::Code, 2, 'P'),
            "L2" => (ObservationType::Phase, 2, 'C'),
            "D2" => (ObservationType::Doppler, 2, 'C'),
            "S2" => (ObservationType::Snr, 2, 'C'),
            "C5" => (ObservationType::Code, 5, 'X'),
            "L5" => (ObservationType::Phase, 5, 'X'),
            "D5" => (ObservationType::Doppler, 5, 'X'),
            "S5" => (ObservationType::Snr, 5, 'X'),
            _ => return None,
        };
        
        Some(SignalCode::new(obs_type, band, attr))
    }
}

/// Convenience function to read RINEX observation file
pub fn read_rinex_obs<P: AsRef<Path>>(path: P) -> Result<ObservationData> {
    RinexObsReader::new().read(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_code_parse() {
        let code = SignalCode::parse("C1C").unwrap();
        assert_eq!(code.obs_type, ObservationType::Code);
        assert_eq!(code.band, 1);
        assert_eq!(code.attribute, 'C');
    }

    #[test]
    fn test_satellite_parse() {
        let sat = Satellite::parse("G15").unwrap();
        assert_eq!(sat.system, GnssSystem::Gps);
        assert_eq!(sat.prn, 15);
    }
}
