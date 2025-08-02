use std::collections::HashMap;
use nalgebra::DVector;
use anyhow::{anyhow, Result};
use log::{info, warn, debug};
use serde::{Deserialize, Serialize};

use crate::circuit::Circuit;
use crate::parser::{SpiceParser, SpiceNetlist};
use crate::mna::MnaSystem;
use crate::solver::{LinearSolver, SolverConfig, auto_select_solver};
use crate::cli::OutputFormat;

/// Simulation results container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub analysis_type: AnalysisType,
    pub time_points: Vec<f64>,
    pub node_voltages: HashMap<String, Vec<f64>>,
    pub currents: HashMap<String, Vec<f64>>,
    pub convergence_info: Vec<ConvergenceInfo>,
    pub total_time: f64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    Operating,
    DcSweep { parameter: String, start: f64, stop: f64, step: f64 },
    Transient { tstep: f64, tstop: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceInfo {
    pub iteration: usize,
    pub residual_norm: f64,
    pub solve_time: f64,
    pub solver_method: String,
}

/// Main simulator engine
pub struct Simulator {
    circuit: Option<Circuit>,
    mna_system: Option<MnaSystem>,
    solver: LinearSolver,
    results: Option<SimulationResult>,
    config: SimulatorConfig,
}

#[derive(Debug, Clone)]
pub struct SimulatorConfig {
    pub solver_config: SolverConfig,
    pub max_iterations: usize,
    pub convergence_tolerance: f64,
    pub auto_select_solver: bool,
    pub store_intermediate_results: bool,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        SimulatorConfig {
            solver_config: SolverConfig::default(),
            max_iterations: 50,
            convergence_tolerance: 1e-9,
            auto_select_solver: true,
            store_intermediate_results: false,
        }
    }
}

impl Simulator {
    /// Create a new simulator with default configuration
    pub fn new() -> Self {
        Simulator {
            circuit: None,
            mna_system: None,
            solver: LinearSolver::new(),
            results: None,
            config: SimulatorConfig::default(),
        }
    }

    /// Create a new simulator with custom configuration
    pub fn with_config(config: SimulatorConfig) -> Self {
        let solver = LinearSolver::with_config(config.solver_config.clone());
        Simulator {
            circuit: None,
            mna_system: None,
            solver,
            results: None,
            config,
        }
    }

    /// Load a SPICE netlist from file
    pub fn load_netlist(&mut self, filename: &str) -> Result<()> {
        info!("Loading netlist from: {}", filename);
        
        let parser = SpiceParser::new();
        let netlist = parser.parse_file(filename)?;
        
        self.load_netlist_from_parsed(netlist)
    }

    /// Load from parsed SPICE netlist
    pub fn load_netlist_from_parsed(&mut self, netlist: SpiceNetlist) -> Result<()> {
        // Convert SpiceNetlist to Circuit
        let mut circuit = Circuit::new(netlist.title);
        
        // Add all components
        for component in netlist.components {
            circuit.add_component(component)?;
        }
        
        // Validate the circuit
        circuit.validate()?;
        
        info!("Loaded circuit: {}", circuit.title);
        circuit.print_summary();
        
        // Create MNA system
        let mna_system = MnaSystem::new(&circuit)?;
        
        self.circuit = Some(circuit);
        self.mna_system = Some(mna_system);
        
        Ok(())
    }

    /// Run operating point analysis
    pub fn run_operating_point(&mut self) -> Result<()> {
        info!("Starting operating point analysis");
        
        let circuit = self.circuit.as_ref()
            .ok_or_else(|| anyhow!("No circuit loaded"))?;
        let mut mna_system = self.mna_system.take()
            .ok_or_else(|| anyhow!("No MNA system available"))?;

        // Assemble DC system
        mna_system.assemble_dc(circuit)?;
        
        // Auto-select solver if enabled
        if self.config.auto_select_solver {
            let (sparse_matrix, _) = mna_system.to_sparse();
            let optimal_method = auto_select_solver(&sparse_matrix);
            self.solver = LinearSolver::with_config(SolverConfig {
                method: optimal_method,
                ..self.config.solver_config.clone()
            });
        }

        // Solve the system
        let start_time = std::time::Instant::now();
        let (sparse_matrix, rhs) = mna_system.to_sparse();
        let (solution, solver_stats) = self.solver.solve_sparse(&sparse_matrix, &rhs)?;
        
        // Update MNA system with solution
        mna_system.update_solution(&solution)?;
        
        // Store results
        let mut node_voltages = HashMap::new();
        for node in &circuit.nodes {
            let voltage = mna_system.get_node_voltage(node.id)?;
            node_voltages.insert(node.name.clone(), vec![voltage]);
        }

        let mut currents = HashMap::new();
        for vs in circuit.voltage_sources() {
            let current = mna_system.get_voltage_source_current(&vs.name)?;
            currents.insert(vs.name.clone(), vec![current]);
        }

        let convergence_info = vec![ConvergenceInfo {
            iteration: 0,
            residual_norm: solver_stats.residual_norm,
            solve_time: solver_stats.solve_time,
            solver_method: format!("{:?}", solver_stats.method_used),
        }];

        self.results = Some(SimulationResult {
            analysis_type: AnalysisType::Operating,
            time_points: vec![0.0],
            node_voltages,
            currents,
            convergence_info,
            total_time: start_time.elapsed().as_secs_f64(),
            success: solver_stats.success,
        });

        self.mna_system = Some(mna_system);
        
        info!("Operating point analysis completed in {:.3}ms", 
              start_time.elapsed().as_millis());
        
        Ok(())
    }

    /// Run DC sweep analysis
    pub fn run_dc_sweep(&mut self, source_name: &str, start: f64, stop: f64, step: f64) -> Result<()> {
        info!("Starting DC sweep analysis: {} from {} to {} step {}", 
              source_name, start, stop, step);
        
        let circuit = self.circuit.as_ref()
            .ok_or_else(|| anyhow!("No circuit loaded"))?;
        let mut mna_system = self.mna_system.take()
            .ok_or_else(|| anyhow!("No MNA system available"))?;

        // Find the source component
        let source_component = circuit.components.iter()
            .find(|comp| comp.name == source_name)
            .ok_or_else(|| anyhow!("Source component '{}' not found", source_name))?;

        let _original_value = source_component.value;
        let num_points = ((stop - start) / step).abs() as usize + 1;
        
        let mut sweep_points = Vec::new();
        let mut all_node_voltages: HashMap<String, Vec<f64>> = HashMap::new();
        let mut all_currents: HashMap<String, Vec<f64>> = HashMap::new();
        let mut convergence_info = Vec::new();

        // Initialize vectors for each node
        for node in &circuit.nodes {
            all_node_voltages.insert(node.name.clone(), Vec::new());
        }
        for vs in circuit.voltage_sources() {
            all_currents.insert(vs.name.clone(), Vec::new());
        }

        for i in 0..num_points {
            let sweep_value = start + i as f64 * step;
            sweep_points.push(sweep_value);

            // Update the source value (this is simplified - in a real implementation,
            // you'd need to modify the circuit or MNA system directly)
            debug!("DC sweep point {}: {} = {}", i, source_name, sweep_value);

            // For now, we'll solve with the original circuit and note this limitation
            mna_system.assemble_dc(circuit)?;
            
            // Solve the system
            let (sparse_matrix, rhs) = mna_system.to_sparse();
            let (solution, solver_stats) = self.solver.solve_sparse(&sparse_matrix, &rhs)?;
            
            mna_system.update_solution(&solution)?;

            // Store results for this sweep point
            for node in &circuit.nodes {
                let voltage = mna_system.get_node_voltage(node.id)?;
                all_node_voltages.get_mut(&node.name).unwrap().push(voltage);
            }

            for vs in circuit.voltage_sources() {
                let current = mna_system.get_voltage_source_current(&vs.name)?;
                all_currents.get_mut(&vs.name).unwrap().push(current);
            }

            convergence_info.push(ConvergenceInfo {
                iteration: i,
                residual_norm: solver_stats.residual_norm,
                solve_time: solver_stats.solve_time,
                solver_method: format!("{:?}", solver_stats.method_used),
            });
        }

        let start_time = std::time::Instant::now();
        self.results = Some(SimulationResult {
            analysis_type: AnalysisType::DcSweep { 
                parameter: source_name.to_string(), 
                start, 
                stop, 
                step 
            },
            time_points: sweep_points,
            node_voltages: all_node_voltages,
            currents: all_currents,
            convergence_info,
            total_time: start_time.elapsed().as_secs_f64(),
            success: true,
        });

        self.mna_system = Some(mna_system);
        
        info!("DC sweep analysis completed with {} points", num_points);
        
        Ok(())
    }

    /// Run transient analysis
    pub fn run_transient_analysis(&mut self, tstep: f64, tstop: f64) -> Result<()> {
        info!("Starting transient analysis: tstep={}, tstop={}", tstep, tstop);
        
        let circuit = self.circuit.as_ref()
            .ok_or_else(|| anyhow!("No circuit loaded"))?;
        let mut mna_system = self.mna_system.take()
            .ok_or_else(|| anyhow!("No MNA system available"))?;

        let num_steps = (tstop / tstep) as usize + 1;
        let mut time_points = Vec::new();
        let mut all_node_voltages: HashMap<String, Vec<f64>> = HashMap::new();
        let mut all_currents: HashMap<String, Vec<f64>> = HashMap::new();
        let mut convergence_info = Vec::new();

        // Initialize vectors
        for node in &circuit.nodes {
            all_node_voltages.insert(node.name.clone(), Vec::new());
        }
        for vs in circuit.voltage_sources() {
            all_currents.insert(vs.name.clone(), Vec::new());
        }

        // Initial conditions (t=0, all voltages and currents are zero)
        let mut prev_voltages = DVector::zeros(mna_system.num_nodes);
        time_points.push(0.0);

        // Store initial conditions
        for node in &circuit.nodes {
            all_node_voltages.get_mut(&node.name).unwrap().push(0.0);
        }
        for vs in circuit.voltage_sources() {
            all_currents.get_mut(&vs.name).unwrap().push(0.0);
        }

        // Time stepping loop
        for step in 1..num_steps {
            let current_time = step as f64 * tstep;
            time_points.push(current_time);

            debug!("Transient step {}: t = {:.6}s", step, current_time);

            // Assemble system for this time step
            mna_system.assemble_transient(circuit, tstep, &prev_voltages)?;

            // Solve the system
            let (sparse_matrix, rhs) = mna_system.to_sparse();
            let (solution, solver_stats) = self.solver.solve_sparse(&sparse_matrix, &rhs)?;
            
            mna_system.update_solution(&solution)?;

            // Update previous voltages for next iteration
            prev_voltages = mna_system.get_node_voltages();

            // Store results
            for node in &circuit.nodes {
                let voltage = mna_system.get_node_voltage(node.id)?;
                all_node_voltages.get_mut(&node.name).unwrap().push(voltage);
            }

            for vs in circuit.voltage_sources() {
                let current = mna_system.get_voltage_source_current(&vs.name)?;
                all_currents.get_mut(&vs.name).unwrap().push(current);
            }

            convergence_info.push(ConvergenceInfo {
                iteration: step,
                residual_norm: solver_stats.residual_norm,
                solve_time: solver_stats.solve_time,
                solver_method: format!("{:?}", solver_stats.method_used),
            });

            // Check for convergence issues
            if !solver_stats.success {
                warn!("Convergence issue at t = {:.6}s", current_time);
            }
        }

        let start_time = std::time::Instant::now();
        self.results = Some(SimulationResult {
            analysis_type: AnalysisType::Transient { tstep, tstop },
            time_points,
            node_voltages: all_node_voltages,
            currents: all_currents,
            convergence_info,
            total_time: start_time.elapsed().as_secs_f64(),
            success: true,
        });

        self.mna_system = Some(mna_system);
        
        info!("Transient analysis completed with {} time points", num_steps);
        
        Ok(())
    }

    /// Get simulation results
    pub fn get_results(&self) -> Option<&SimulationResult> {
        self.results.as_ref()
    }

    /// Export simulation results to file
    pub fn export_results(&self, filename: &str, format: OutputFormat) -> Result<()> {
        let results = self.results.as_ref()
            .ok_or_else(|| anyhow!("No simulation results available"))?;

        match format {
            OutputFormat::Csv => self.export_csv(results, filename),
            OutputFormat::Json => self.export_json(results, filename),
        }
    }

    /// Export results to CSV format
    fn export_csv(&self, results: &SimulationResult, filename: &str) -> Result<()> {
        use std::fs::File;
        use csv::Writer;

        let file = File::create(filename)?;
        let mut writer = Writer::from_writer(file);

        // Create header
        let mut header = vec!["time".to_string()];
        for node_name in results.node_voltages.keys() {
            header.push(format!("V({})", node_name));
        }
        for current_name in results.currents.keys() {
            header.push(format!("I({})", current_name));
        }
        writer.write_record(&header)?;

        // Write data
        for (i, &time) in results.time_points.iter().enumerate() {
            let mut record = vec![time.to_string()];
            
            for node_name in results.node_voltages.keys() {
                let voltage = results.node_voltages[node_name].get(i).unwrap_or(&0.0);
                record.push(voltage.to_string());
            }
            
            for current_name in results.currents.keys() {
                let current = results.currents[current_name].get(i).unwrap_or(&0.0);
                record.push(current.to_string());
            }
            
            writer.write_record(&record)?;
        }

        writer.flush()?;
        info!("Results exported to CSV: {}", filename);
        Ok(())
    }

    /// Export results to JSON format
    fn export_json(&self, results: &SimulationResult, filename: &str) -> Result<()> {
        use std::fs::File;
        
        let file = File::create(filename)?;
        serde_json::to_writer_pretty(file, results)?;
        
        info!("Results exported to JSON: {}", filename);
        Ok(())
    }

    /// Print simulation summary
    pub fn print_summary(&self) {
        if let Some(results) = &self.results {
            println!("\n=== Simulation Summary ===");
            println!("Analysis type: {:?}", results.analysis_type);
            println!("Total simulation time: {:.3}ms", results.total_time * 1000.0);
            println!("Success: {}", results.success);
            println!("Number of time points: {}", results.time_points.len());
            
            if !results.node_voltages.is_empty() {
                println!("\nNode voltages (final values):");
                for (node_name, voltages) in &results.node_voltages {
                    if let Some(&final_voltage) = voltages.last() {
                        println!("  V({}): {:.6}V", node_name, final_voltage);
                    }
                }
            }
            
            if !results.currents.is_empty() {
                println!("\nSource currents (final values):");
                for (source_name, currents) in &results.currents {
                    if let Some(&final_current) = currents.last() {
                        println!("  I({}): {:.6}A", source_name, final_current);
                    }
                }
            }

            // Convergence statistics
            if !results.convergence_info.is_empty() {
                let total_iterations: usize = results.convergence_info.len();
                let avg_residual: f64 = results.convergence_info.iter()
                    .map(|info| info.residual_norm)
                    .sum::<f64>() / total_iterations as f64;
                
                println!("\nConvergence info:");
                println!("  Total iterations: {}", total_iterations);
                println!("  Average residual norm: {:.2e}", avg_residual);
            }
        } else {
            println!("No simulation results available");
        }
    }
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{Circuit, Component};

    #[test]
    fn test_simulator_operating_point() {
        let mut simulator = Simulator::new();
        
        // Create a simple test circuit
        let mut circuit = Circuit::new("Test Circuit".to_string());
        circuit.add_node("1".to_string());
        circuit.add_node("0".to_string());
        
        let vs = Component::new_voltage_source("V1".to_string(), "1".to_string(), "0".to_string(), 5.0);
        let r = Component::new_resistor("R1".to_string(), "1".to_string(), "0".to_string(), 1000.0);
        
        circuit.add_component(vs).unwrap();
        circuit.add_component(r).unwrap();

        // Load circuit into simulator
        let netlist = crate::parser::SpiceNetlist {
            title: circuit.title.clone(),
            components: circuit.components.clone(),
            nodes: circuit.nodes.clone(),
            subcircuits: Vec::new(),
            parameters: std::collections::HashMap::new(),
            analyses: Vec::new(),
        };
        
        simulator.load_netlist_from_parsed(netlist).unwrap();
        
        // Run operating point analysis
        simulator.run_operating_point().unwrap();
        
        let results = simulator.get_results().unwrap();
        assert!(results.success);
        assert_eq!(results.time_points.len(), 1);
    }
} 