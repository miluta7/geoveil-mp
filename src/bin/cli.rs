//! Command-line interface for GeoVeil-MP multipath analysis.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use geoveil_mp::{
    prelude::*,
    RinexObsReader, Sp3Reader,
    navigation::SatellitePositionProvider,
    plotting::{PlotConfig, RPlotter},
};

#[derive(Parser)]
#[command(name = "geoveil-mp")]
#[command(author = "Miluta-Dulea Flueras <miluta.flueras@cartografie.ro>")]
#[command(version)]
#[command(about = "GeoVeil-MP: GNSS multipath analysis tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze RINEX observation file for multipath
    Analyze {
        /// RINEX observation file
        #[arg(short, long)]
        obs: PathBuf,

        /// Navigation file (RINEX or SP3)
        #[arg(short, long)]
        nav: Option<PathBuf>,

        /// SP3 precise ephemeris file
        #[arg(long)]
        sp3: Option<PathBuf>,

        /// Output directory
        #[arg(short, long, default_value = "output")]
        output: PathBuf,

        /// Elevation cutoff angle (degrees)
        #[arg(short, long, default_value = "10")]
        elevation: f64,

        /// Systems to analyze (G,R,E,C,J)
        #[arg(short, long)]
        systems: Option<String>,

        /// Generate plots
        #[arg(long)]
        plot: bool,

        /// Export to CSV
        #[arg(long)]
        csv: bool,
    },

    /// Read and display RINEX file info
    Info {
        /// RINEX file to inspect
        file: PathBuf,
    },

    /// Estimate receiver position
    Position {
        /// RINEX observation file
        #[arg(short, long)]
        obs: PathBuf,

        /// Navigation file
        #[arg(short, long)]
        nav: PathBuf,

        /// Epoch to compute (YYYY MM DD HH MM SS)
        #[arg(short, long)]
        epoch: Option<String>,

        /// System to use (G, R, E, C)
        #[arg(short, long, default_value = "G")]
        system: String,
    },
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            obs,
            nav,
            sp3,
            output,
            elevation,
            systems,
            plot,
            csv,
        } => {
            analyze_command(obs, nav, sp3, output, elevation, systems, plot, csv)?;
        }
        Commands::Info { file } => {
            info_command(file)?;
        }
        Commands::Position { obs, nav, epoch, system } => {
            position_command(obs, nav, epoch, system)?;
        }
    }

    Ok(())
}

fn analyze_command(
    obs_path: PathBuf,
    nav_path: Option<PathBuf>,
    sp3_path: Option<PathBuf>,
    output: PathBuf,
    elevation: f64,
    systems: Option<String>,
    plot: bool,
    csv: bool,
) -> Result<()> {
    println!("{}", "GeoVeil-MP Multipath Analysis".bold().blue());
    println!("{}", "=".repeat(50));

    // Progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}% {msg}")?
            .progress_chars("#>-"),
    );

    // Read observation file
    pb.set_message("Reading observation file...");
    pb.set_position(10);
    
    let obs_data = RinexObsReader::new().read(&obs_path)?;
    
    println!("  {} {}", "File:".green(), obs_path.display());
    println!("  {} {}", "Version:".green(), obs_data.header.version);
    println!("  {} {}", "Marker:".green(), obs_data.header.marker_name);
    println!("  {} {}", "Epochs:".green(), obs_data.num_epochs());
    println!("  {} {}", "Satellites:".green(), obs_data.satellites().len());

    pb.set_position(30);
    pb.set_message("Processing navigation data...");

    // Build satellite position provider
    let sat_provider = if let Some(sp3) = sp3_path {
        let sp3_data = Sp3Reader::read(&sp3)?;
        SatellitePositionProvider::with_sp3(sp3_data)
    } else if let Some(_nav) = nav_path {
        // TODO: Add navigation file reader
        println!("  {} Navigation file reading not yet implemented", "Note:".yellow());
        return Ok(());
    } else {
        println!("  {} Need either --nav or --sp3 for ephemeris data", "Error:".red());
        return Ok(());
    };

    pb.set_position(50);
    pb.set_message("Running multipath analysis...");

    // Configure analysis
    let mut config = AnalysisConfig::default()
        .with_elevation_cutoff(elevation);

    if let Some(sys) = systems {
        let sys_list: Vec<&str> = sys.split(',').collect();
        config = config.with_systems(&sys_list);
    }

    // Run analysis
    let analyzer = MultipathAnalyzer::new(obs_data, config);
    let results = analyzer.analyze()?;

    pb.set_position(80);
    pb.set_message("Generating output...");

    // Create output directory
    std::fs::create_dir_all(&output)?;

    // Print results
    println!("\n{}", "Results".bold().green());
    println!("{}", "-".repeat(50));
    
    for (signal, stats) in &results.statistics {
        println!(
            "  {} RMS: {:.4} m, Count: {}, Weighted RMS: {:.4} m",
            signal.cyan(),
            stats.rms,
            stats.count,
            stats.weighted_rms
        );
    }

    // Export CSV
    if csv {
        let csv_path = output.join("multipath_results.csv");
        results.to_csv(csv_path.to_str().unwrap())?;
        println!("\n  {} {}", "CSV exported:".green(), csv_path.display());
    }

    // Generate plots
    if plot {
        pb.set_message("Generating plots...");
        let plot_config = PlotConfig {
            output_dir: output.to_str().unwrap().to_string(),
            ..Default::default()
        };
        
        let plotter = RPlotter::new(plot_config);
        if plotter.is_r_available() {
            let plots = plotter.generate_plots(&results)?;
            println!("\n  {} Generated {} plots", "Plots:".green(), plots.len());
        } else {
            println!("\n  {} R not available, skipping plots", "Note:".yellow());
        }
    }

    pb.set_position(100);
    pb.finish_with_message("Complete!");

    println!("\n{}", "Analysis complete!".bold().green());

    Ok(())
}

fn info_command(file: PathBuf) -> Result<()> {
    println!("{}", "RINEX File Information".bold().blue());
    println!("{}", "=".repeat(50));

    let obs_data = RinexObsReader::new().read(&file)?;
    let header = &obs_data.header;

    println!("  {} {}", "File:".green(), file.display());
    println!("  {} {}", "Version:".green(), header.version);
    println!("  {} {}", "Type:".green(), header.file_type);
    println!("  {} {}", "Marker:".green(), header.marker_name);
    println!("  {} {}", "Receiver:".green(), header.receiver_type);
    println!("  {} {}", "Antenna:".green(), header.antenna_type);
    
    if let Some(pos) = &header.approx_position {
        println!("  {} X={:.3} Y={:.3} Z={:.3}", 
            "Position:".green(), pos.x, pos.y, pos.z);
    }

    println!("\n  {} {}", "Epochs:".green(), obs_data.num_epochs());
    if let Some(interval) = obs_data.interval() {
        println!("  {} {:.1} s", "Interval:".green(), interval);
    }

    println!("\n  {}", "Satellites:".green());
    for system in obs_data.systems() {
        let sats = obs_data.satellites_for_system(system);
        println!("    {} ({}): {}", 
            system.name(),
            system.to_char(),
            sats.len()
        );
    }

    println!("\n  {}", "Observation Types:".green());
    for (system, codes) in &header.obs_types {
        let code_strs: Vec<String> = codes.iter().map(|c| c.to_string()).collect();
        println!("    {}: {}", system.to_char(), code_strs.join(", "));
    }

    Ok(())
}

fn position_command(
    obs_path: PathBuf,
    nav_path: PathBuf,
    epoch_str: Option<String>,
    system: String,
) -> Result<()> {
    println!("{}", "Position Estimation".bold().blue());
    println!("{}", "=".repeat(50));

    let obs_data = RinexObsReader::new().read(&obs_path)?;
    
    // Determine epoch
    let _target_epoch = if let Some(ep) = epoch_str {
        Epoch::parse(&ep)?
    } else {
        // Use first epoch
        obs_data.epochs.first()
            .map(|e| e.epoch)
            .context("No epochs in file")?
    };

    println!("  {} Position estimation requires navigation data", "Note:".yellow());
    println!("  Full implementation coming soon!");

    Ok(())
}
