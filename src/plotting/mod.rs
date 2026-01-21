//! R plotting integration for GNSS multipath visualization.
//!
//! This module provides integration with R for creating publication-quality
//! plots of GNSS multipath analysis results.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::analysis::{AnalysisResults, MultipathEstimate, MultipathStatistics};
use crate::utils::{Error, Result};

/// Plot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotConfig {
    /// Output directory
    pub output_dir: String,
    /// Plot width (inches)
    pub width: f64,
    /// Plot height (inches)
    pub height: f64,
    /// DPI for raster outputs
    pub dpi: u32,
    /// Output format (png, pdf, svg)
    pub format: String,
    /// Use LaTeX for text rendering
    pub use_latex: bool,
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Font size
    pub font_size: f64,
    /// Title font size
    pub title_size: f64,
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            output_dir: "plots".to_string(),
            width: 10.0,
            height: 8.0,
            dpi: 300,
            format: "png".to_string(),
            use_latex: false,
            color_scheme: ColorScheme::default(),
            font_size: 12.0,
            title_size: 14.0,
        }
    }
}

/// Color scheme for plots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub gps: String,
    pub glonass: String,
    pub galileo: String,
    pub beidou: String,
    pub qzss: String,
    pub background: String,
    pub grid: String,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            gps: "#1f77b4".to_string(),      // Blue
            glonass: "#ff7f0e".to_string(),  // Orange
            galileo: "#2ca02c".to_string(),  // Green
            beidou: "#d62728".to_string(),   // Red
            qzss: "#9467bd".to_string(),     // Purple
            background: "#ffffff".to_string(),
            grid: "#e0e0e0".to_string(),
        }
    }
}

/// R plotting interface
pub struct RPlotter {
    /// Configuration
    config: PlotConfig,
    /// R script path
    r_path: String,
}

impl RPlotter {
    /// Create a new plotter
    pub fn new(config: PlotConfig) -> Self {
        Self {
            config,
            r_path: "Rscript".to_string(),
        }
    }

    /// Set custom R path
    pub fn with_r_path(mut self, path: &str) -> Self {
        self.r_path = path.to_string();
        self
    }

    /// Check if R is available
    pub fn is_r_available(&self) -> bool {
        Command::new(&self.r_path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Generate all plots for analysis results
    pub fn generate_plots(&self, results: &AnalysisResults) -> Result<Vec<String>> {
        // Create output directory
        fs::create_dir_all(&self.config.output_dir)?;

        let mut generated = Vec::new();

        // Export data to CSV for R
        let data_path = format!("{}/multipath_data.csv", self.config.output_dir);
        self.export_data_for_r(results, &data_path)?;

        // Generate R script
        let script_path = format!("{}/plot_script.R", self.config.output_dir);
        self.generate_r_script(&data_path, &script_path)?;

        // Execute R script
        if self.is_r_available() {
            let output = Command::new(&self.r_path)
                .arg(&script_path)
                .output()
                .map_err(|e| Error::PlottingError(format!("Failed to run R: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::PlottingError(format!("R script failed: {}", stderr)));
            }

            // List generated files
            for entry in fs::read_dir(&self.config.output_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == self.config.format.as_str()).unwrap_or(false) {
                    generated.push(path.to_string_lossy().to_string());
                }
            }
        } else {
            // Return script path if R not available
            generated.push(script_path);
        }

        Ok(generated)
    }

    /// Export data to CSV for R
    fn export_data_for_r(&self, results: &AnalysisResults, path: &str) -> Result<()> {
        let mut file = File::create(path)?;
        
        // Header
        writeln!(file, "signal,satellite,epoch,mp_value,elevation,azimuth,snr")?;

        // Data
        for (signal, estimates) in &results.estimates {
            for est in estimates {
                writeln!(
                    file,
                    "{},{},{},{:.6},{:.2},{:.2},{}",
                    signal,
                    est.satellite,
                    est.epoch.to_iso_string(),
                    est.mp_value,
                    est.elevation,
                    est.azimuth,
                    est.snr.map(|s| format!("{:.1}", s)).unwrap_or_default()
                )?;
            }
        }

        Ok(())
    }

    /// Generate R plotting script
    fn generate_r_script(&self, data_path: &str, script_path: &str) -> Result<()> {
        let mut file = File::create(script_path)?;
        
        let r_script = format!(r#"
# GNSS Multipath Analysis Plots
# Auto-generated by gnss_multipath Rust library

library(ggplot2)
library(dplyr)
library(tidyr)

# Configuration
output_dir <- "{output_dir}"
width <- {width}
height <- {height}
dpi <- {dpi}
format <- "{format}"

# Read data
data <- read.csv("{data_path}")
data$epoch <- as.POSIXct(data$epoch, format="%Y-%m-%dT%H:%M:%OS")

# Color palette
colors <- c(
    "G" = "{gps}",
    "R" = "{glonass}",
    "E" = "{galileo}",
    "C" = "{beidou}",
    "J" = "{qzss}"
)

# Theme
theme_gnss <- theme_minimal() +
    theme(
        text = element_text(size = {font_size}),
        plot.title = element_text(size = {title_size}, face = "bold"),
        panel.grid.minor = element_blank(),
        legend.position = "bottom"
    )

# Extract system from satellite ID
data$system <- substr(data$satellite, 1, 1)

# 1. Multipath vs Time
for (sig in unique(data$signal)) {{
    sig_data <- filter(data, signal == sig)
    p <- ggplot(sig_data, aes(x = epoch, y = mp_value, color = system)) +
        geom_point(alpha = 0.5, size = 0.5) +
        geom_hline(yintercept = 0, linetype = "dashed", color = "gray50") +
        scale_color_manual(values = colors) +
        labs(
            title = paste("Multipath Effect -", sig),
            x = "Time",
            y = "Multipath (m)",
            color = "System"
        ) +
        theme_gnss
    
    ggsave(
        file.path(output_dir, paste0("mp_time_", sig, ".", format)),
        p, width = width, height = height / 2, dpi = dpi
    )
}}

# 2. Multipath vs Elevation
for (sig in unique(data$signal)) {{
    sig_data <- filter(data, signal == sig)
    p <- ggplot(sig_data, aes(x = elevation, y = mp_value, color = system)) +
        geom_point(alpha = 0.3, size = 0.5) +
        geom_smooth(method = "loess", se = FALSE, size = 1) +
        geom_hline(yintercept = 0, linetype = "dashed", color = "gray50") +
        scale_color_manual(values = colors) +
        labs(
            title = paste("Multipath vs Elevation -", sig),
            x = "Elevation (degrees)",
            y = "Multipath (m)",
            color = "System"
        ) +
        xlim(0, 90) +
        theme_gnss
    
    ggsave(
        file.path(output_dir, paste0("mp_elevation_", sig, ".", format)),
        p, width = width, height = height / 2, dpi = dpi
    )
}}

# 3. Polar Skyplot with Multipath
for (sig in unique(data$signal)) {{
    sig_data <- filter(data, signal == sig)
    sig_data$r <- 90 - sig_data$elevation
    
    p <- ggplot(sig_data, aes(x = azimuth, y = r)) +
        geom_point(aes(color = mp_value), alpha = 0.5, size = 1) +
        scale_color_gradient2(
            low = "blue", mid = "white", high = "red",
            midpoint = 0,
            limits = c(-2, 2),
            oob = scales::squish,
            name = "MP (m)"
        ) +
        coord_polar(theta = "x", start = -pi/2, direction = -1) +
        scale_x_continuous(
            breaks = seq(0, 315, 45),
            labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW")
        ) +
        scale_y_continuous(limits = c(0, 90), breaks = seq(0, 90, 30)) +
        labs(
            title = paste("Multipath Skyplot -", sig),
            x = "",
            y = "Zenith Angle"
        ) +
        theme_gnss +
        theme(
            axis.text.y = element_blank(),
            panel.grid.major = element_line(color = "gray80")
        )
    
    ggsave(
        file.path(output_dir, paste0("skyplot_mp_", sig, ".", format)),
        p, width = height, height = height, dpi = dpi
    )
}}

# 4. RMS Bar Plot
rms_data <- data %>%
    group_by(signal) %>%
    summarise(
        rms = sqrt(mean(mp_value^2)),
        count = n()
    )

p <- ggplot(rms_data, aes(x = reorder(signal, -rms), y = rms)) +
    geom_bar(stat = "identity", fill = "steelblue") +
    geom_text(aes(label = sprintf("%.3f", rms)), vjust = -0.5, size = 3) +
    labs(
        title = "Multipath RMS by Signal",
        x = "Signal",
        y = "RMS (m)"
    ) +
    theme_gnss +
    theme(axis.text.x = element_text(angle = 45, hjust = 1))

ggsave(
    file.path(output_dir, paste0("rms_barplot.", format)),
    p, width = width, height = height / 2, dpi = dpi
)

# 5. SNR Polar Plot (if SNR data available)
if (!all(is.na(data$snr))) {{
    for (sig in unique(data$signal)) {{
        sig_data <- filter(data, signal == sig, !is.na(snr))
        if (nrow(sig_data) > 0) {{
            sig_data$r <- 90 - sig_data$elevation
            
            p <- ggplot(sig_data, aes(x = azimuth, y = r)) +
                geom_point(aes(color = snr), alpha = 0.5, size = 1) +
                scale_color_viridis_c(
                    name = "SNR\n(dB-Hz)",
                    limits = c(20, 50),
                    oob = scales::squish
                ) +
                coord_polar(theta = "x", start = -pi/2, direction = -1) +
                scale_x_continuous(
                    breaks = seq(0, 315, 45),
                    labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW")
                ) +
                scale_y_continuous(limits = c(0, 90), breaks = seq(0, 90, 30)) +
                labs(
                    title = paste("SNR Skyplot -", sig),
                    x = "",
                    y = ""
                ) +
                theme_gnss +
                theme(axis.text.y = element_blank())
            
            ggsave(
                file.path(output_dir, paste0("skyplot_snr_", sig, ".", format)),
                p, width = height, height = height, dpi = dpi
            )
        }}
    }}
}}

# 6. Satellite Tracks
sats_per_system <- data %>%
    group_by(system) %>%
    summarise(satellites = list(unique(satellite)))

for (sys in unique(data$system)) {{
    sys_data <- filter(data, system == sys)
    sys_data$r <- 90 - sys_data$elevation
    
    p <- ggplot(sys_data, aes(x = azimuth, y = r, group = satellite)) +
        geom_path(aes(color = satellite), alpha = 0.7, size = 0.5) +
        coord_polar(theta = "x", start = -pi/2, direction = -1) +
        scale_x_continuous(
            breaks = seq(0, 315, 45),
            labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW")
        ) +
        scale_y_continuous(limits = c(0, 90), breaks = seq(0, 90, 30)) +
        labs(
            title = paste("Satellite Tracks -", sys),
            x = "",
            y = ""
        ) +
        theme_gnss +
        theme(
            axis.text.y = element_blank(),
            legend.position = "none"
        )
    
    ggsave(
        file.path(output_dir, paste0("tracks_", sys, ".", format)),
        p, width = height, height = height, dpi = dpi
    )
}}

print("Plotting complete!")
"#,
            output_dir = self.config.output_dir,
            width = self.config.width,
            height = self.config.height,
            dpi = self.config.dpi,
            format = self.config.format,
            data_path = data_path,
            gps = self.config.color_scheme.gps,
            glonass = self.config.color_scheme.glonass,
            galileo = self.config.color_scheme.galileo,
            beidou = self.config.color_scheme.beidou,
            qzss = self.config.color_scheme.qzss,
            font_size = self.config.font_size,
            title_size = self.config.title_size,
        );

        file.write_all(r_script.as_bytes())?;
        Ok(())
    }

    /// Generate a single skyplot
    pub fn generate_skyplot(
        &self,
        estimates: &[MultipathEstimate],
        title: &str,
        output_path: &str,
    ) -> Result<()> {
        let data_path = format!("{}.csv", output_path.trim_end_matches(&format!(".{}", self.config.format)));
        
        // Export data
        let mut file = File::create(&data_path)?;
        writeln!(file, "satellite,elevation,azimuth,mp_value,snr")?;
        for est in estimates {
            writeln!(
                file,
                "{},{:.2},{:.2},{:.6},{}",
                est.satellite,
                est.elevation,
                est.azimuth,
                est.mp_value,
                est.snr.map(|s| format!("{:.1}", s)).unwrap_or_default()
            )?;
        }
        drop(file);

        // Generate R script
        let script = format!(r#"
library(ggplot2)
data <- read.csv("{data_path}")
data$r <- 90 - data$elevation

p <- ggplot(data, aes(x = azimuth, y = r)) +
    geom_point(aes(color = mp_value), alpha = 0.5, size = 1) +
    scale_color_gradient2(
        low = "blue", mid = "white", high = "red",
        midpoint = 0, limits = c(-2, 2),
        oob = scales::squish, name = "MP (m)"
    ) +
    coord_polar(theta = "x", start = -pi/2, direction = -1) +
    scale_x_continuous(
        breaks = seq(0, 315, 45),
        labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW")
    ) +
    scale_y_continuous(limits = c(0, 90)) +
    labs(title = "{title}") +
    theme_minimal()

ggsave("{output_path}", p, width = {height}, height = {height}, dpi = {dpi})
"#,
            data_path = data_path,
            title = title,
            output_path = output_path,
            height = self.config.height,
            dpi = self.config.dpi,
        );

        let script_path = format!("{}.R", output_path.trim_end_matches(&format!(".{}", self.config.format)));
        fs::write(&script_path, &script)?;

        if self.is_r_available() {
            Command::new(&self.r_path)
                .arg(&script_path)
                .output()
                .map_err(|e| Error::PlottingError(format!("R execution failed: {}", e)))?;
        }

        Ok(())
    }
}

/// Placeholder types for plot configuration
pub struct SkyPlot;
pub struct TimeSeries;
pub struct PolarPlot;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_config_default() {
        let config = PlotConfig::default();
        assert_eq!(config.format, "png");
        assert_eq!(config.dpi, 300);
    }

    #[test]
    fn test_color_scheme() {
        let scheme = ColorScheme::default();
        assert!(scheme.gps.starts_with('#'));
    }
}
