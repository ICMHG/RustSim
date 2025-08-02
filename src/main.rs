use clap::{Arg, ArgMatches, Command};
use colored::*;
use log::{error, info};
use std::path::Path;

mod circuit;
mod cli;
mod mna;
mod output;
mod parser;
mod simulator;
mod solver;

use crate::cli::CliArgs;
use crate::simulator::Simulator;

fn main() {
    env_logger::init();

    let matches = create_cli().get_matches();
    
    if let Err(e) = run_application(&matches) {
        error!("{}", format!("Error: {}", e).red());
        std::process::exit(1);
    }
}

fn create_cli() -> Command {
    Command::new("RustSim")
        .version("0.1.0")
        .about("A high-performance SPICE circuit simulator written in Rust")
        .author("SPICE-Rust Team")
        .arg(
            Arg::new("input")
                .help("Input SPICE netlist file (.sp)")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file for simulation results"),
        )
        .arg(
            Arg::new("tran")
                .long("tran")
                .value_names(["TSTEP", "TSTOP"])
                .num_args(2)
                .help("Transient analysis: time step and stop time"),
        )
        .arg(
            Arg::new("dc")
                .long("dc")
                .value_names(["SOURCE", "START", "STOP", "STEP"])
                .num_args(4)
                .help("DC sweep analysis"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity level"),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .default_value("csv")
                .value_parser(["csv", "json"])
                .help("Output format"),
        )
}

fn run_application(matches: &ArgMatches) -> anyhow::Result<()> {
    let args = CliArgs::from_matches(matches)?;
    
    info!("{}", "Starting RustSim - Rust SPICE Simulator".green().bold());
    info!("Input file: {}", args.input_file.bright_blue());
    
    // Validate input file exists
    if !Path::new(&args.input_file).exists() {
        return Err(anyhow::anyhow!("Input file '{}' not found", args.input_file));
    }
    
    // Create and run simulator
    let mut simulator = Simulator::new();
    simulator.load_netlist(&args.input_file)?;
    
    match args.analysis_type {
        cli::AnalysisType::Transient { tstep, tstop } => {
            info!("Running transient analysis: tstep={}, tstop={}", tstep, tstop);
            simulator.run_transient_analysis(tstep, tstop)?;
        }
        cli::AnalysisType::DcSweep { source, start, stop, step } => {
            info!("Running DC sweep: source={}, range=[{}, {}], step={}", 
                  source, start, stop, step);
            simulator.run_dc_sweep(&source, start, stop, step)?;
        }
        cli::AnalysisType::Operating => {
            info!("Running operating point analysis");
            simulator.run_operating_point()?;
        }
    }
    
    // Export results
    if let Some(output_file) = args.output_file {
        simulator.export_results(&output_file, args.output_format)?;
        info!("Results exported to: {}", output_file.bright_green());
    } else {
        simulator.print_summary();
    }
    
    info!("{}", "Simulation completed successfully!".green().bold());
    Ok(())
} 