use nalgebra::{DMatrix, DVector};
use sprs::{CsMat, TriMat};
use std::collections::HashMap;
use anyhow::{anyhow, Result};

use crate::circuit::{Circuit, Component, ComponentType};

/// MNA system representation: [A][x] = [z]
/// where A is the system matrix, x is the unknown vector, and z is the RHS vector
#[derive(Debug, Clone)]
pub struct MnaSystem {
    /// System matrix A (coefficient matrix)
    pub matrix: DMatrix<f64>,
    /// Right-hand side vector z
    pub rhs: DVector<f64>,
    /// Unknown vector x (node voltages + voltage source currents)
    pub unknowns: DVector<f64>,
    /// Mapping from node IDs to matrix row/column indices
    pub node_map: HashMap<usize, usize>,
    /// Mapping from voltage source names to current variable indices
    pub voltage_source_map: HashMap<String, usize>,
    /// Total system size
    pub size: usize,
    /// Number of nodes (excluding ground)
    pub num_nodes: usize,
    /// Number of voltage sources
    pub num_voltage_sources: usize,
}

impl MnaSystem {
    /// Create a new MNA system for the given circuit
    pub fn new(circuit: &Circuit) -> Result<Self> {
        let num_nodes = circuit.node_count(); // Non-ground nodes
        let voltage_sources = circuit.voltage_sources();
        let num_voltage_sources = voltage_sources.len();
        let size = num_nodes + num_voltage_sources;

        if size == 0 {
            return Err(anyhow!("Circuit has no nodes or voltage sources to analyze"));
        }

        // Create node mapping (non-ground nodes only)
        let mut node_map = HashMap::new();
        let mut index = 0;
        for node in &circuit.nodes {
            if Some(node.id) != circuit.ground_node {
                node_map.insert(node.id, index);
                index += 1;
            }
        }

        // Create voltage source mapping
        let mut voltage_source_map = HashMap::new();
        for (i, vs) in voltage_sources.iter().enumerate() {
            voltage_source_map.insert(vs.name.clone(), num_nodes + i);
        }

        let matrix = DMatrix::zeros(size, size);
        let rhs = DVector::zeros(size);
        let unknowns = DVector::zeros(size);

        Ok(MnaSystem {
            matrix,
            rhs,
            unknowns,
            node_map,
            voltage_source_map,
            size,
            num_nodes,
            num_voltage_sources,
        })
    }

    /// Assemble the MNA system for DC analysis
    pub fn assemble_dc(&mut self, circuit: &Circuit) -> Result<()> {
        // Clear existing system
        self.matrix.fill(0.0);
        self.rhs.fill(0.0);

        // Process linear components (R, L, C)
        for component in circuit.linear_components() {
            self.add_linear_component(circuit, component)?;
        }

        // Process current sources
        for component in circuit.current_sources() {
            self.add_current_source(circuit, component)?;
        }

        // Process voltage sources
        for component in circuit.voltage_sources() {
            self.add_voltage_source(circuit, component)?;
        }

        Ok(())
    }

    /// Assemble the MNA system for transient analysis
    pub fn assemble_transient(&mut self, circuit: &Circuit, dt: f64, prev_voltages: &DVector<f64>) -> Result<()> {
        // Start with DC assembly
        self.assemble_dc(circuit)?;

        // Add capacitor contributions for transient analysis
        for component in &circuit.components {
            if let ComponentType::Capacitor = component.component_type {
                self.add_capacitor_transient(circuit, component, dt, prev_voltages)?;
            }
        }

        // Add inductor contributions for transient analysis
        for component in &circuit.components {
            if let ComponentType::Inductor = component.component_type {
                self.add_inductor_transient(circuit, component, dt, prev_voltages)?;
            }
        }

        Ok(())
    }

    /// Add a linear component (R, L, C) to the system
    fn add_linear_component(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        let node1_name = &component.nodes[0];
        let node2_name = &component.nodes[1];

        let node1_id = circuit.get_node_id(node1_name)
            .ok_or_else(|| anyhow!("Node {} not found", node1_name))?;
        let node2_id = circuit.get_node_id(node2_name)
            .ok_or_else(|| anyhow!("Node {} not found", node2_name))?;

        let node1_idx = self.node_map.get(&node1_id);
        let node2_idx = self.node_map.get(&node2_id);

        match component.component_type {
            ComponentType::Resistor => {
                let conductance = component.conductance()?;
                
                // Add to diagonal elements
                if let Some(&idx1) = node1_idx {
                    self.matrix[(idx1, idx1)] += conductance;
                }
                if let Some(&idx2) = node2_idx {
                    self.matrix[(idx2, idx2)] += conductance;
                }

                // Add to off-diagonal elements
                if let (Some(&idx1), Some(&idx2)) = (node1_idx, node2_idx) {
                    self.matrix[(idx1, idx2)] -= conductance;
                    self.matrix[(idx2, idx1)] -= conductance;
                }
            }
            ComponentType::Capacitor => {
                // For DC analysis, capacitors are open circuits (infinite impedance)
                // No contribution to the conductance matrix
            }
            ComponentType::Inductor => {
                // For DC analysis, inductors are short circuits (zero impedance)
                // This requires special handling with current variables
                self.add_inductor_dc(circuit, component)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Add inductor for DC analysis (treated as short circuit)
    fn add_inductor_dc(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        // For DC analysis, inductor acts like a voltage source with 0V
        // This constrains the voltage across the inductor to be zero
        
        let node1_name = &component.nodes[0];
        let node2_name = &component.nodes[1];

        let node1_id = circuit.get_node_id(node1_name)
            .ok_or_else(|| anyhow!("Node {} not found", node1_name))?;
        let node2_id = circuit.get_node_id(node2_name)
            .ok_or_else(|| anyhow!("Node {} not found", node2_name))?;

        let node1_idx = self.node_map.get(&node1_id);
        let node2_idx = self.node_map.get(&node2_id);

        // Find or create current variable for this inductor
        let current_idx = if let Some(&idx) = self.voltage_source_map.get(&component.name) {
            idx
        } else {
            // Add as additional unknown
            let _idx = self.size;
            // Note: This would require expanding the system dynamically
            // For simplicity, we'll treat inductors as very small resistors for DC
            let small_resistance = 1e-12;
            let conductance = 1.0 / small_resistance;
            
            if let Some(&idx1) = node1_idx {
                self.matrix[(idx1, idx1)] += conductance;
            }
            if let Some(&idx2) = node2_idx {
                self.matrix[(idx2, idx2)] += conductance;
            }
            if let (Some(&idx1), Some(&idx2)) = (node1_idx, node2_idx) {
                self.matrix[(idx1, idx2)] -= conductance;
                self.matrix[(idx2, idx1)] -= conductance;
            }
            return Ok(());
        };

        // Add voltage constraint: V_node1 - V_node2 = 0
        if let Some(&idx1) = node1_idx {
            self.matrix[(current_idx, idx1)] = 1.0;
            self.matrix[(idx1, current_idx)] = 1.0;
        }
        if let Some(&idx2) = node2_idx {
            self.matrix[(current_idx, idx2)] = -1.0;
            self.matrix[(idx2, current_idx)] = -1.0;
        }

        // RHS is 0 for inductor voltage constraint
        self.rhs[current_idx] = 0.0;

        Ok(())
    }

    /// Add a current source to the system
    fn add_current_source(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        let node1_name = &component.nodes[0]; // Positive terminal
        let node2_name = &component.nodes[1]; // Negative terminal

        let node1_id = circuit.get_node_id(node1_name)
            .ok_or_else(|| anyhow!("Node {} not found", node1_name))?;
        let node2_id = circuit.get_node_id(node2_name)
            .ok_or_else(|| anyhow!("Node {} not found", node2_name))?;

        let current = component.value;

        // Add current to RHS vector
        if let Some(&idx1) = self.node_map.get(&node1_id) {
            self.rhs[idx1] += current; // Current flows into positive node
        }
        if let Some(&idx2) = self.node_map.get(&node2_id) {
            self.rhs[idx2] -= current; // Current flows out of negative node
        }

        Ok(())
    }

    /// Add a voltage source to the system
    fn add_voltage_source(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        let node1_name = &component.nodes[0]; // Positive terminal
        let node2_name = &component.nodes[1]; // Negative terminal

        let node1_id = circuit.get_node_id(node1_name)
            .ok_or_else(|| anyhow!("Node {} not found", node1_name))?;
        let node2_id = circuit.get_node_id(node2_name)
            .ok_or_else(|| anyhow!("Node {} not found", node2_name))?;

        let voltage = component.value;
        let vs_idx = self.voltage_source_map.get(&component.name)
            .ok_or_else(|| anyhow!("Voltage source {} not found in mapping", component.name))?;

        // Add voltage constraint: V_node1 - V_node2 = V_source
        if let Some(&idx1) = self.node_map.get(&node1_id) {
            self.matrix[(*vs_idx, idx1)] = 1.0;
            self.matrix[(idx1, *vs_idx)] = 1.0;
        }
        if let Some(&idx2) = self.node_map.get(&node2_id) {
            self.matrix[(*vs_idx, idx2)] = -1.0;
            self.matrix[(idx2, *vs_idx)] = -1.0;
        }

        // Set RHS for voltage source constraint
        self.rhs[*vs_idx] = voltage;

        Ok(())
    }

    /// Add capacitor contribution for transient analysis
    fn add_capacitor_transient(&mut self, circuit: &Circuit, component: &Component, dt: f64, prev_voltages: &DVector<f64>) -> Result<()> {
        let node1_name = &component.nodes[0];
        let node2_name = &component.nodes[1];

        let node1_id = circuit.get_node_id(node1_name)
            .ok_or_else(|| anyhow!("Node {} not found", node1_name))?;
        let node2_id = circuit.get_node_id(node2_name)
            .ok_or_else(|| anyhow!("Node {} not found", node2_name))?;

        let node1_idx = self.node_map.get(&node1_id);
        let node2_idx = self.node_map.get(&node2_id);

        let capacitance = component.value;
        let conductance = capacitance / dt; // Backward Euler

        // Add to matrix (same as resistor with G = C/dt)
        if let Some(&idx1) = node1_idx {
            self.matrix[(idx1, idx1)] += conductance;
        }
        if let Some(&idx2) = node2_idx {
            self.matrix[(idx2, idx2)] += conductance;
        }
        if let (Some(&idx1), Some(&idx2)) = (node1_idx, node2_idx) {
            self.matrix[(idx1, idx2)] -= conductance;
            self.matrix[(idx2, idx1)] -= conductance;
        }

        // Add current source term based on previous voltage
        if prev_voltages.len() >= self.num_nodes {
            let prev_v1 = if let Some(&idx1) = node1_idx { prev_voltages[idx1] } else { 0.0 };
            let prev_v2 = if let Some(&idx2) = node2_idx { prev_voltages[idx2] } else { 0.0 };
            let prev_voltage_across = prev_v1 - prev_v2;
            let current_source = conductance * prev_voltage_across;

            if let Some(&idx1) = node1_idx {
                self.rhs[idx1] += current_source;
            }
            if let Some(&idx2) = node2_idx {
                self.rhs[idx2] -= current_source;
            }
        }

        Ok(())
    }

    /// Add inductor contribution for transient analysis
    fn add_inductor_transient(&mut self, _circuit: &Circuit, _component: &Component, _dt: f64, _prev_currents: &DVector<f64>) -> Result<()> {
        // Inductor transient analysis requires tracking current through the inductor
        // This is more complex and would require expanding the system
        // For now, we'll skip this implementation
        Ok(())
    }

    /// Convert to sparse matrix format for efficient solving
    pub fn to_sparse(&self) -> (CsMat<f64>, Vec<f64>) {
        let mut triplets = Vec::new();
        
        // Convert dense matrix to triplet format
        for i in 0..self.size {
            for j in 0..self.size {
                let value = self.matrix[(i, j)];
                if value.abs() > 1e-15 { // Skip near-zero values
                    triplets.push((i, j, value));
                }
            }
        }

        let mut triplet_mat = TriMat::new((self.size, self.size));
        
        for (row, col, value) in triplets {
            triplet_mat.add_triplet(row, col, value);
        }
        
        let sparse_matrix = triplet_mat.to_csr();
        let rhs_vec = self.rhs.as_slice().to_vec();

        (sparse_matrix, rhs_vec)
    }

    /// Update the solution vector
    pub fn update_solution(&mut self, solution: &[f64]) -> Result<()> {
        if solution.len() != self.size {
            return Err(anyhow!("Solution vector size mismatch: expected {}, got {}", self.size, solution.len()));
        }

        for (i, &value) in solution.iter().enumerate() {
            self.unknowns[i] = value;
        }

        Ok(())
    }

    /// Get node voltage by node ID
    pub fn get_node_voltage(&self, node_id: usize) -> Result<f64> {
        if let Some(&matrix_idx) = self.node_map.get(&node_id) {
            Ok(self.unknowns[matrix_idx])
        } else {
            // This might be the ground node
            Ok(0.0)
        }
    }

    /// Get voltage source current by name
    pub fn get_voltage_source_current(&self, name: &str) -> Result<f64> {
        if let Some(&idx) = self.voltage_source_map.get(name) {
            Ok(self.unknowns[idx])
        } else {
            Err(anyhow!("Voltage source {} not found", name))
        }
    }

    /// Get all node voltages as a vector
    pub fn get_node_voltages(&self) -> DVector<f64> {
        // Return only the node voltage portion of the solution
        if self.num_nodes > 0 {
            self.unknowns.rows(0, self.num_nodes).into()
        } else {
            DVector::zeros(0)
        }
    }

    /// Print system information for debugging
    pub fn print_system_info(&self) {
        println!("MNA System Information:");
        println!("  Size: {} x {}", self.size, self.size);
        println!("  Nodes: {}", self.num_nodes);
        println!("  Voltage sources: {}", self.num_voltage_sources);
        println!("  Matrix condition: {:.2e}", self.matrix_condition_number());
    }

    /// Estimate matrix condition number (rough approximation)
    fn matrix_condition_number(&self) -> f64 {
        // This is a very rough approximation
        // For a proper condition number, we'd need SVD
        let max_element = self.matrix.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));
        let min_element = self.matrix.iter()
            .filter(|&&x| x.abs() > 1e-15)
            .fold(f64::INFINITY, |acc, &x| acc.min(x.abs()));
        
        if min_element.is_finite() && min_element > 0.0 {
            max_element / min_element
        } else {
            f64::INFINITY
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{Circuit, Component};

    #[test]
    fn test_mna_simple_resistor_circuit() {
        let mut circuit = Circuit::new("Test".to_string());
        
        // Create simple voltage divider: V1 - R1 - R2 - GND
        circuit.add_node("1".to_string());
        circuit.add_node("0".to_string());
        
        let v1 = Component::new_voltage_source("V1".to_string(), "1".to_string(), "0".to_string(), 5.0);
        let r1 = Component::new_resistor("R1".to_string(), "1".to_string(), "0".to_string(), 1000.0);
        
        circuit.add_component(v1).unwrap();
        circuit.add_component(r1).unwrap();

        let mut mna = MnaSystem::new(&circuit).unwrap();
        mna.assemble_dc(&circuit).unwrap();

        assert_eq!(mna.size, 2); // 1 node + 1 voltage source
        assert_eq!(mna.num_nodes, 1);
        assert_eq!(mna.num_voltage_sources, 1);
    }

    #[test]
    fn test_mna_current_source() {
        let mut circuit = Circuit::new("Test".to_string());
        
        circuit.add_node("1".to_string());
        circuit.add_node("0".to_string());
        
        let i1 = Component::new_current_source("I1".to_string(), "1".to_string(), "0".to_string(), 0.001);
        let r1 = Component::new_resistor("R1".to_string(), "1".to_string(), "0".to_string(), 1000.0);
        
        circuit.add_component(i1).unwrap();
        circuit.add_component(r1).unwrap();

        let mut mna = MnaSystem::new(&circuit).unwrap();
        mna.assemble_dc(&circuit).unwrap();

        assert_eq!(mna.size, 1); // Only 1 node (no voltage sources)
        assert_eq!(mna.rhs[0], 0.001); // Current source contributes to RHS
    }
} 