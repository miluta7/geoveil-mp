#!/usr/bin/env Rscript
# GNSS Multipath Analysis - R Plotting Library
# 
# This script provides publication-quality plots for GNSS multipath analysis.
# Can be called from Rust or used standalone.
#
# Usage:
#   Rscript gnss_plots.R --data multipath.csv --output plots/
#
# Required packages: ggplot2, dplyr, tidyr, scales, viridis, gridExtra

# Check and install required packages
required_packages <- c("ggplot2", "dplyr", "tidyr", "scales", "viridis", "gridExtra", "optparse")

install_if_missing <- function(pkg) {
    if (!require(pkg, character.only = TRUE, quietly = TRUE)) {
        install.packages(pkg, repos = "https://cloud.r-project.org/", quiet = TRUE)
        library(pkg, character.only = TRUE)
    }
}

suppressMessages(lapply(required_packages, install_if_missing))

# Parse command line arguments
option_list <- list(
    make_option(c("-d", "--data"), type = "character", default = "multipath_data.csv",
                help = "Input CSV file with multipath data"),
    make_option(c("-o", "--output"), type = "character", default = "plots",
                help = "Output directory for plots"),
    make_option(c("-f", "--format"), type = "character", default = "png",
                help = "Output format (png, pdf, svg)"),
    make_option(c("-w", "--width"), type = "double", default = 10,
                help = "Plot width in inches"),
    make_option(c("-h", "--height"), type = "double", default = 8,
                help = "Plot height in inches"),
    make_option(c("--dpi"), type = "integer", default = 300,
                help = "Resolution in DPI")
)

opt_parser <- OptionParser(option_list = option_list)
opt <- parse_args(opt_parser)

# Create output directory
dir.create(opt$output, showWarnings = FALSE, recursive = TRUE)

# GNSS color palette
gnss_colors <- c(
    "G" = "#1f77b4",  # GPS - Blue
    "R" = "#ff7f0e",  # GLONASS - Orange
    "E" = "#2ca02c",  # Galileo - Green
    "C" = "#d62728",  # BeiDou - Red
    "J" = "#9467bd",  # QZSS - Purple
    "I" = "#8c564b",  # NavIC - Brown
    "S" = "#e377c2"   # SBAS - Pink
)

# Theme for GNSS plots
theme_gnss <- function(base_size = 12) {
    theme_minimal(base_size = base_size) +
        theme(
            plot.title = element_text(size = base_size + 2, face = "bold", hjust = 0.5),
            plot.subtitle = element_text(size = base_size, hjust = 0.5, color = "gray40"),
            axis.title = element_text(size = base_size),
            axis.text = element_text(size = base_size - 2),
            legend.position = "bottom",
            legend.title = element_text(size = base_size),
            legend.text = element_text(size = base_size - 2),
            panel.grid.minor = element_blank(),
            panel.grid.major = element_line(color = "gray90"),
            plot.margin = margin(10, 10, 10, 10)
        )
}

# Function to create polar coordinates for skyplot
create_polar_data <- function(data) {
    data %>%
        mutate(
            r = 90 - elevation,
            azimuth_rad = azimuth * pi / 180
        )
}

# Plot 1: Multipath vs Time
plot_mp_time <- function(data, signal) {
    sig_data <- filter(data, signal == !!signal)
    
    ggplot(sig_data, aes(x = epoch, y = mp_value, color = system)) +
        geom_point(alpha = 0.4, size = 0.8) +
        geom_hline(yintercept = 0, linetype = "dashed", color = "gray50", size = 0.5) +
        scale_color_manual(values = gnss_colors, name = "System") +
        labs(
            title = paste("Code Multipath vs Time -", signal),
            subtitle = paste("N =", nrow(sig_data), "observations"),
            x = "Time (UTC)",
            y = "Multipath Effect (m)"
        ) +
        theme_gnss() +
        scale_y_continuous(limits = c(-3, 3), oob = scales::squish)
}

# Plot 2: Multipath vs Elevation
plot_mp_elevation <- function(data, signal) {
    sig_data <- filter(data, signal == !!signal)
    
    ggplot(sig_data, aes(x = elevation, y = mp_value, color = system)) +
        geom_point(alpha = 0.3, size = 0.5) +
        geom_smooth(method = "loess", se = FALSE, size = 1, span = 0.5) +
        geom_hline(yintercept = 0, linetype = "dashed", color = "gray50", size = 0.5) +
        scale_color_manual(values = gnss_colors, name = "System") +
        labs(
            title = paste("Multipath vs Elevation -", signal),
            x = "Elevation Angle (degrees)",
            y = "Multipath Effect (m)"
        ) +
        theme_gnss() +
        scale_x_continuous(limits = c(0, 90), breaks = seq(0, 90, 15)) +
        scale_y_continuous(limits = c(-3, 3), oob = scales::squish)
}

# Plot 3: Polar Skyplot with Multipath
plot_skyplot_mp <- function(data, signal) {
    sig_data <- create_polar_data(filter(data, signal == !!signal))
    
    ggplot(sig_data, aes(x = azimuth, y = r)) +
        geom_point(aes(color = mp_value), alpha = 0.6, size = 1.5) +
        scale_color_gradient2(
            low = "#0571b0",
            mid = "white", 
            high = "#ca0020",
            midpoint = 0,
            limits = c(-2, 2),
            oob = scales::squish,
            name = "MP (m)"
        ) +
        coord_polar(theta = "x", start = -pi/2, direction = -1) +
        scale_x_continuous(
            breaks = seq(0, 315, 45),
            labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW"),
            limits = c(0, 360)
        ) +
        scale_y_continuous(limits = c(0, 90), breaks = c(30, 60, 90)) +
        labs(
            title = paste("Multipath Skyplot -", signal),
            x = "", y = ""
        ) +
        theme_gnss() +
        theme(
            axis.text.y = element_blank(),
            panel.grid.major = element_line(color = "gray80", size = 0.3)
        )
}

# Plot 4: SNR Skyplot
plot_skyplot_snr <- function(data, signal) {
    sig_data <- create_polar_data(filter(data, signal == !!signal, !is.na(snr)))
    
    if (nrow(sig_data) == 0) return(NULL)
    
    ggplot(sig_data, aes(x = azimuth, y = r)) +
        geom_point(aes(color = snr), alpha = 0.6, size = 1.5) +
        scale_color_viridis_c(
            name = "SNR\n(dB-Hz)",
            limits = c(20, 55),
            oob = scales::squish
        ) +
        coord_polar(theta = "x", start = -pi/2, direction = -1) +
        scale_x_continuous(
            breaks = seq(0, 315, 45),
            labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW"),
            limits = c(0, 360)
        ) +
        scale_y_continuous(limits = c(0, 90), breaks = c(30, 60, 90)) +
        labs(
            title = paste("SNR Skyplot -", signal),
            x = "", y = ""
        ) +
        theme_gnss() +
        theme(axis.text.y = element_blank())
}

# Plot 5: RMS Bar Plot
plot_rms_barplot <- function(data) {
    rms_data <- data %>%
        group_by(signal) %>%
        summarise(
            rms = sqrt(mean(mp_value^2, na.rm = TRUE)),
            weighted_rms = sqrt(sum(mp_value^2 / (1 + (30 - elevation)^2/900), na.rm = TRUE) / n()),
            count = n(),
            .groups = "drop"
        ) %>%
        arrange(desc(rms))
    
    ggplot(rms_data, aes(x = reorder(signal, -rms), y = rms, fill = signal)) +
        geom_bar(stat = "identity", show.legend = FALSE) +
        geom_text(aes(label = sprintf("%.3f", rms)), vjust = -0.5, size = 3.5) +
        scale_fill_brewer(palette = "Set2") +
        labs(
            title = "Multipath RMS by Signal",
            subtitle = paste("Total:", sum(rms_data$count), "observations"),
            x = "Signal",
            y = "RMS (m)"
        ) +
        theme_gnss() +
        theme(axis.text.x = element_text(angle = 45, hjust = 1)) +
        scale_y_continuous(expand = expansion(mult = c(0, 0.15)))
}

# Plot 6: Combined Multipath Plot (Time + Elevation)
plot_mp_combined <- function(data, signal) {
    p1 <- plot_mp_time(data, signal)
    p2 <- plot_mp_elevation(data, signal)
    
    gridExtra::grid.arrange(p1, p2, nrow = 2)
}

# Plot 7: Satellite Tracks
plot_satellite_tracks <- function(data, system) {
    sys_data <- create_polar_data(filter(data, system == !!system))
    
    if (nrow(sys_data) == 0) return(NULL)
    
    ggplot(sys_data, aes(x = azimuth, y = r, group = satellite, color = satellite)) +
        geom_path(alpha = 0.7, size = 0.5) +
        coord_polar(theta = "x", start = -pi/2, direction = -1) +
        scale_x_continuous(
            breaks = seq(0, 315, 45),
            labels = c("N", "NE", "E", "SE", "S", "SW", "W", "NW"),
            limits = c(0, 360)
        ) +
        scale_y_continuous(limits = c(0, 90), breaks = c(30, 60, 90)) +
        labs(
            title = paste("Satellite Tracks -", system),
            x = "", y = ""
        ) +
        theme_gnss() +
        theme(
            axis.text.y = element_blank(),
            legend.position = "right"
        ) +
        guides(color = guide_legend(ncol = 2, override.aes = list(size = 2)))
}

# Plot 8: SNR vs Elevation
plot_snr_elevation <- function(data, signal) {
    sig_data <- filter(data, signal == !!signal, !is.na(snr))
    
    if (nrow(sig_data) == 0) return(NULL)
    
    ggplot(sig_data, aes(x = elevation, y = snr, color = system)) +
        geom_point(alpha = 0.3, size = 0.5) +
        geom_smooth(method = "loess", se = FALSE, size = 1) +
        scale_color_manual(values = gnss_colors, name = "System") +
        labs(
            title = paste("SNR vs Elevation -", signal),
            x = "Elevation Angle (degrees)",
            y = "Signal-to-Noise Ratio (dB-Hz)"
        ) +
        theme_gnss() +
        scale_x_continuous(limits = c(0, 90), breaks = seq(0, 90, 15))
}

# Main execution
main <- function() {
    cat("GNSS Multipath Analysis - R Plotting\n")
    cat("=====================================\n\n")
    
    # Read data
    if (!file.exists(opt$data)) {
        cat("Error: Data file not found:", opt$data, "\n")
        quit(status = 1)
    }
    
    cat("Reading data from:", opt$data, "\n")
    data <- read.csv(opt$data, stringsAsFactors = FALSE)
    
    # Convert epoch to datetime
    data$epoch <- as.POSIXct(data$epoch, format = "%Y-%m-%dT%H:%M:%OSZ", tz = "UTC")
    
    # Extract system from satellite ID
    data$system <- substr(data$satellite, 1, 1)
    
    cat("  Observations:", nrow(data), "\n")
    cat("  Signals:", paste(unique(data$signal), collapse = ", "), "\n")
    cat("  Systems:", paste(unique(data$system), collapse = ", "), "\n\n")
    
    # Generate plots
    signals <- unique(data$signal)
    systems <- unique(data$system)
    
    cat("Generating plots...\n")
    
    # Per-signal plots
    for (sig in signals) {
        cat("  Processing signal:", sig, "\n")
        
        # Multipath vs Time
        p <- plot_mp_time(data, sig)
        ggsave(
            file.path(opt$output, paste0("mp_time_", sig, ".", opt$format)),
            p, width = opt$width, height = opt$height/2, dpi = opt$dpi
        )
        
        # Multipath vs Elevation
        p <- plot_mp_elevation(data, sig)
        ggsave(
            file.path(opt$output, paste0("mp_elevation_", sig, ".", opt$format)),
            p, width = opt$width, height = opt$height/2, dpi = opt$dpi
        )
        
        # Multipath Skyplot
        p <- plot_skyplot_mp(data, sig)
        ggsave(
            file.path(opt$output, paste0("skyplot_mp_", sig, ".", opt$format)),
            p, width = opt$height, height = opt$height, dpi = opt$dpi
        )
        
        # SNR Skyplot
        p <- plot_skyplot_snr(data, sig)
        if (!is.null(p)) {
            ggsave(
                file.path(opt$output, paste0("skyplot_snr_", sig, ".", opt$format)),
                p, width = opt$height, height = opt$height, dpi = opt$dpi
            )
        }
        
        # SNR vs Elevation
        p <- plot_snr_elevation(data, sig)
        if (!is.null(p)) {
            ggsave(
                file.path(opt$output, paste0("snr_elevation_", sig, ".", opt$format)),
                p, width = opt$width, height = opt$height/2, dpi = opt$dpi
            )
        }
    }
    
    # RMS Bar Plot
    cat("  Generating RMS bar plot\n")
    p <- plot_rms_barplot(data)
    ggsave(
        file.path(opt$output, paste0("rms_barplot.", opt$format)),
        p, width = opt$width, height = opt$height/2, dpi = opt$dpi
    )
    
    # Satellite tracks per system
    for (sys in systems) {
        cat("  Generating satellite tracks for:", sys, "\n")
        p <- plot_satellite_tracks(data, sys)
        if (!is.null(p)) {
            ggsave(
                file.path(opt$output, paste0("tracks_", sys, ".", opt$format)),
                p, width = opt$height, height = opt$height, dpi = opt$dpi
            )
        }
    }
    
    cat("\nPlotting complete!\n")
    cat("Output directory:", opt$output, "\n")
}

# Run main function
if (!interactive()) {
    main()
}
