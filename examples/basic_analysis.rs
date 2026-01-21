//! Basic GNSS Multipath Analysis Example
//!
//! This example demonstrates how to:
//! 1. Read a RINEX observation file
//! 2. Read SP3 precise ephemeris
//! 3. Run multipath analysis
//! 4. Export results

use anyhow::Result;
use geoveil_mp::{
    prelude::*,
    RinexObsReader, Sp3Reader,
    navigation::SatellitePositionProvider,
    plotting::{PlotConfig, RPlotter},
};

fn main() -> Result<()> {
    println!("GeoVeil-MP Multipath Analysis Example");
    println!("======================================\n");

    // Example file paths (replace with actual files)
    let obs_file = "example.24o";
    let sp3_file = "example.sp3";

    // Check if example files exist
    if !std::path::Path::new(obs_file).exists() {
        println!("Note: Example files not found.");
        println!("To run this example, provide:");
        println!("  - A RINEX observation file (*.XXo)");
        println!("  - An SP3 precise ephemeris file (*.sp3)");
        println!("\nDemonstrating API usage instead...\n");
        
        demonstrate_api();
        return Ok(());
    }

    // Read observation file
    println!("Reading observation file...");
    let obs_data = RinexObsReader::new().read(obs_file)?;
    
    println!("  Version: {}", obs_data.header.version);
    println!("  Marker: {}", obs_data.header.marker_name);
    println!("  Epochs: {}", obs_data.num_epochs());
    println!("  Satellites: {}", obs_data.satellites().len());

    // Read SP3 file
    println!("\nReading SP3 file...");
    let sp3_data = Sp3Reader::read(sp3_file)?;
    
    println!("  Satellites: {}", sp3_data.num_satellites);
    println!("  Interval: {} s", sp3_data.interval);

    // Create satellite position provider
    let sat_provider = SatellitePositionProvider::with_sp3(sp3_data);

    // Configure analysis
    println!("\nConfiguring analysis...");
    let config = AnalysisConfig::default()
        .with_elevation_cutoff(10.0)
        .with_systems(&["G", "E", "R", "C"]);

    // Run analysis
    println!("Running multipath analysis...");
    let analyzer = MultipathAnalyzer::new(obs_data, config);
    let results = analyzer.analyze()?;

    // Print results
    println!("\n{:=<50}", "");
    println!("RESULTS");
    println!("{:=<50}\n", "");

    for (signal, stats) in &results.statistics {
        println!("Signal: {}", signal);
        println!("  Count:        {}", stats.count);
        println!("  RMS:          {:.4} m", stats.rms);
        println!("  Weighted RMS: {:.4} m", stats.weighted_rms);
        println!("  Mean:         {:.4} m", stats.mean);
        println!("  Std Dev:      {:.4} m", stats.std_dev);
        println!("  Min:          {:.4} m", stats.min);
        println!("  Max:          {:.4} m", stats.max);
        println!();
    }

    println!("Summary:");
    println!("  Total estimates: {}", results.summary.total_estimates);
    println!("  Signals analyzed: {}", results.summary.num_signals);
    println!("  Average RMS: {:.4} m", results.summary.average_rms);

    // Export to CSV
    println!("\nExporting results to CSV...");
    results.to_csv("multipath_results.csv")?;
    println!("  Saved to: multipath_results.csv");

    // Generate plots (if R available)
    let plot_config = PlotConfig::default();
    let plotter = RPlotter::new(plot_config);
    
    if plotter.is_r_available() {
        println!("\nGenerating plots...");
        let plots = plotter.generate_plots(&results)?;
        println!("  Generated {} plots", plots.len());
    } else {
        println!("\nNote: R not available, skipping plot generation");
    }

    println!("\nAnalysis complete!");
    
    Ok(())
}

/// Demonstrate API usage without actual files
fn demonstrate_api() {
    use geoveil_mp::utils::Epoch;
    
    // Creating epochs
    let epoch = Epoch::new(2024, 1, 15, 12, 30, 0.0);
    println!("Created epoch: {}", epoch);
    
    // GPS time conversion
    let gps_time = epoch.to_gps_time();
    println!("GPS Time: Week {}, TOW {:.3}s", gps_time.week, gps_time.tow);
    
    // Julian Date
    let jd = epoch.to_julian_date();
    println!("Julian Date: {:.6}", jd);
    
    // Day of year
    println!("Day of Year: {}", epoch.day_of_year());
    
    // Creating satellites
    let sat = geoveil_mp::Satellite::parse("G15").unwrap();
    println!("\nParsed satellite: {} (PRN {})", sat, sat.prn);
    
    // GNSS systems
    for sys in geoveil_mp::GnssSystem::all() {
        println!("  {} ({})", sys.name(), sys.to_char());
    }
    
    // Signal codes
    let code = geoveil_mp::SignalCode::parse("C1C").unwrap();
    println!("\nSignal code: {}", code);
    println!("  Type: {:?}", code.obs_type);
    println!("  Band: {}", code.band);
    println!("  Is code: {}", code.is_code());
    println!("  Is phase: {}", code.is_phase());
    
    // Coordinate conversions
    use geoveil_mp::utils::Ecef;
    
    let ecef = Ecef::new(4000000.0, 1000000.0, 4800000.0);
    let geo = ecef.to_geodetic();
    println!("\nCoordinate conversion:");
    println!("  ECEF: X={:.1} Y={:.1} Z={:.1}", ecef.x, ecef.y, ecef.z);
    println!("  Geodetic: Lat={:.4}° Lon={:.4}° H={:.1}m", 
             geo.lat, geo.lon, geo.height);
    
    // Frequencies
    use geoveil_mp::utils::constants::frequencies::gps;
    println!("\nGPS Frequencies:");
    println!("  L1: {:.3} MHz", gps::L1 / 1e6);
    println!("  L2: {:.3} MHz", gps::L2 / 1e6);
    println!("  L5: {:.3} MHz", gps::L5 / 1e6);
    println!("  L1 wavelength: {:.6} m", gps::L1_WAVELENGTH);
    
    // Analysis configuration
    let config = AnalysisConfig::default()
        .with_elevation_cutoff(15.0)
        .with_systems(&["G", "E"]);
    
    println!("\nAnalysis config:");
    println!("  Elevation cutoff: {}°", config.elevation_cutoff);
    println!("  Systems: {:?}", config.systems);
    println!("  Ion threshold: {} m/s", config.ion_threshold);
    
    println!("\n✓ API demonstration complete!");
}
