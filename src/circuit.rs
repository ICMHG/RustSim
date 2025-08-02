use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};

/// Represents a node in the circuit
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub id: usize,
    pub voltage: f64,
}

impl Node {
    pub fn new(name: String) -> Self {
        Node {
            name,
            id: 0, // Will be assigned when added to circuit
            voltage: 0.0,
        }
    }

    pub fn with_id(name: String, id: usize) -> Self {
        Node {
            name,
            id,
            voltage: 0.0,
        }
    }

    pub fn is_ground(&self) -> bool {
        self.name == "0" || self.name.to_lowercase() == "gnd" || self.name.to_lowercase() == "ground"
    }
}

/// Types of circuit components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    Resistor,
    Capacitor,
    Inductor,
    VoltageSource,
    CurrentSource,
    Diode,
    Mosfet { 
        model_type: String,
        width: Option<f64>,
        length: Option<f64>,
    },
    Bjt { 
        model_type: String,
        area: Option<f64>,
    },
}

impl ComponentType {
    /// Returns true if this component is linear
    pub fn is_linear(&self) -> bool {
        matches!(self, 
            ComponentType::Resistor | 
            ComponentType::Capacitor | 
            ComponentType::Inductor | 
            ComponentType::VoltageSource | 
            ComponentType::CurrentSource
        )
    }

    /// Returns true if this component is a source
    pub fn is_source(&self) -> bool {
        matches!(self, ComponentType::VoltageSource | ComponentType::CurrentSource)
    }

    #[allow(dead_code)]
    pub fn tracks_current(&self) -> bool {
        matches!(self, ComponentType::VoltageSource | ComponentType::Inductor)
    }
}

/// Circuit component/element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub component_type: ComponentType,
    pub nodes: Vec<String>,
    pub value: f64,
    pub model: Option<String>,
}

impl Component {
    pub fn new_resistor(name: String, node1: String, node2: String, resistance: f64) -> Self {
        Component {
            name,
            component_type: ComponentType::Resistor,
            nodes: vec![node1, node2],
            value: resistance,
            model: None,
        }
    }

    pub fn new_capacitor(name: String, node1: String, node2: String, capacitance: f64) -> Self {
        Component {
            name,
            component_type: ComponentType::Capacitor,
            nodes: vec![node1, node2],
            value: capacitance,
            model: None,
        }
    }

    pub fn new_inductor(name: String, node1: String, node2: String, inductance: f64) -> Self {
        Component {
            name,
            component_type: ComponentType::Inductor,
            nodes: vec![node1, node2],
            value: inductance,
            model: None,
        }
    }

    pub fn new_voltage_source(name: String, node_pos: String, node_neg: String, voltage: f64) -> Self {
        Component {
            name,
            component_type: ComponentType::VoltageSource,
            nodes: vec![node_pos, node_neg],
            value: voltage,
            model: None,
        }
    }

    pub fn new_current_source(name: String, node_pos: String, node_neg: String, current: f64) -> Self {
        Component {
            name,
            component_type: ComponentType::CurrentSource,
            nodes: vec![node_pos, node_neg],
            value: current,
            model: None,
        }
    }

    /// Get the conductance for resistive elements
    pub fn conductance(&self) -> Result<f64> {
        match self.component_type {
            ComponentType::Resistor => {
                if self.value <= 0.0 {
                    Err(anyhow!("Resistor {} has non-positive resistance: {}", self.name, self.value))
                } else {
                    Ok(1.0 / self.value)
                }
            }
            _ => Err(anyhow!("Component {} is not a resistor", self.name))
        }
    }

    /// Get the number of terminals for this component
    pub fn terminal_count(&self) -> usize {
        match self.component_type {
            ComponentType::Resistor | 
            ComponentType::Capacitor | 
            ComponentType::Inductor | 
            ComponentType::VoltageSource | 
            ComponentType::CurrentSource | 
            ComponentType::Diode => 2,
            ComponentType::Mosfet { .. } => 4, // Drain, Gate, Source, Bulk
            ComponentType::Bjt { .. } => 3,    // Collector, Base, Emitter
        }
    }

    /// Validate that the component has the correct number of nodes
    pub fn validate(&self) -> Result<()> {
        let expected_nodes = self.terminal_count();
        if self.nodes.len() != expected_nodes {
            return Err(anyhow!(
                "Component {} expects {} nodes, but has {}",
                self.name,
                expected_nodes,
                self.nodes.len()
            ));
        }

        // Additional validation based on component type
        match self.component_type {
            ComponentType::Resistor => {
                if self.value <= 0.0 {
                    return Err(anyhow!("Resistor {} must have positive resistance", self.name));
                }
            }
            ComponentType::Capacitor => {
                if self.value <= 0.0 {
                    return Err(anyhow!("Capacitor {} must have positive capacitance", self.name));
                }
            }
            ComponentType::Inductor => {
                if self.value <= 0.0 {
                    return Err(anyhow!("Inductor {} must have positive inductance", self.name));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Complete circuit representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub title: String,
    pub nodes: Vec<Node>,
    pub components: Vec<Component>,
    pub node_map: HashMap<String, usize>,
    pub ground_node: Option<usize>,
}

impl Circuit {
    pub fn new(title: String) -> Self {
        Circuit {
            title,
            nodes: Vec::new(),
            components: Vec::new(),
            node_map: HashMap::new(),
            ground_node: None,
        }
    }

    /// Add a node to the circuit and return its ID
    pub fn add_node(&mut self, name: String) -> usize {
        if let Some(&existing_id) = self.node_map.get(&name) {
            return existing_id;
        }

        let node_id = self.nodes.len();
        let node = Node::with_id(name.clone(), node_id);
        
        // Check if this is a ground node
        if node.is_ground() && self.ground_node.is_none() {
            self.ground_node = Some(node_id);
        }

        self.nodes.push(node);
        self.node_map.insert(name, node_id);
        node_id
    }

    /// Add a component to the circuit
    pub fn add_component(&mut self, component: Component) -> Result<()> {
        // Validate component
        component.validate()?;

        // Ensure all nodes exist
        for node_name in &component.nodes {
            self.add_node(node_name.clone());
        }

        self.components.push(component);
        Ok(())
    }

    /// Get node by name
    pub fn get_node(&self, name: &str) -> Option<&Node> {
        if let Some(&node_id) = self.node_map.get(name) {
            self.nodes.get(node_id)
        } else {
            None
        }
    }

    /// Get node by ID
    pub fn get_node_by_id(&self, id: usize) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get mutable node by ID
    pub fn get_node_by_id_mut(&mut self, id: usize) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Get node ID by name
    pub fn get_node_id(&self, name: &str) -> Option<usize> {
        self.node_map.get(name).copied()
    }

    /// Get the number of non-ground nodes
    pub fn node_count(&self) -> usize {
        if self.ground_node.is_some() {
            self.nodes.len() - 1
        } else {
            self.nodes.len()
        }
    }

    /// Get all non-ground node IDs
    pub fn non_ground_nodes(&self) -> Vec<usize> {
        self.nodes
            .iter()
            .filter(|node| Some(node.id) != self.ground_node)
            .map(|node| node.id)
            .collect()
    }

    /// Get components of a specific type
    pub fn components_of_type(&self, component_type: &ComponentType) -> Vec<&Component> {
        self.components
            .iter()
            .filter(|comp| std::mem::discriminant(&comp.component_type) == std::mem::discriminant(component_type))
            .collect()
    }

    /// Get all voltage sources
    pub fn voltage_sources(&self) -> Vec<&Component> {
        self.components
            .iter()
            .filter(|comp| matches!(comp.component_type, ComponentType::VoltageSource))
            .collect()
    }

    /// Get all current sources  
    pub fn current_sources(&self) -> Vec<&Component> {
        self.components
            .iter()
            .filter(|comp| matches!(comp.component_type, ComponentType::CurrentSource))
            .collect()
    }

    /// Get all linear components (R, L, C)
    pub fn linear_components(&self) -> Vec<&Component> {
        self.components
            .iter()
            .filter(|comp| comp.component_type.is_linear() && !comp.component_type.is_source())
            .collect()
    }

    /// Get all nonlinear components (D, M, Q)
    pub fn nonlinear_components(&self) -> Vec<&Component> {
        self.components
            .iter()
            .filter(|comp| !comp.component_type.is_linear())
            .collect()
    }

    /// Update node voltage
    pub fn set_node_voltage(&mut self, node_id: usize, voltage: f64) -> Result<()> {
        if let Some(node) = self.get_node_by_id_mut(node_id) {
            node.voltage = voltage;
            Ok(())
        } else {
            Err(anyhow!("Node ID {} not found", node_id))
        }
    }

    /// Get node voltage
    pub fn get_node_voltage(&self, node_id: usize) -> Result<f64> {
        if let Some(node) = self.get_node_by_id(node_id) {
            Ok(node.voltage)
        } else {
            Err(anyhow!("Node ID {} not found", node_id))
        }
    }

    /// Validate the entire circuit
    pub fn validate(&self) -> Result<()> {
        // Check for ground node
        if self.ground_node.is_none() {
            return Err(anyhow!("Circuit must have a ground node (named '0', 'gnd', or 'ground')"));
        }

        // Validate all components
        for component in &self.components {
            component.validate()?;
        }

        // Check for floating nodes
        let mut connected_nodes = std::collections::HashSet::new();
        for component in &self.components {
            for node_name in &component.nodes {
                if let Some(node_id) = self.get_node_id(node_name) {
                    connected_nodes.insert(node_id);
                }
            }
        }

        for node in &self.nodes {
            if !connected_nodes.contains(&node.id) && Some(node.id) != self.ground_node {
                return Err(anyhow!("Floating node detected: {}", node.name));
            }
        }

        Ok(())
    }

    /// Print circuit summary
    pub fn print_summary(&self) {
        println!("Circuit: {}", self.title);
        println!("Nodes: {}", self.nodes.len());
        println!("Components: {}", self.components.len());
        
        if let Some(ground_id) = self.ground_node {
            if let Some(ground_node) = self.get_node_by_id(ground_id) {
                println!("Ground node: {}", ground_node.name);
            }
        }

        // Component count by type
        let mut type_counts = HashMap::new();
        for component in &self.components {
            let type_name = match component.component_type {
                ComponentType::Resistor => "Resistors",
                ComponentType::Capacitor => "Capacitors", 
                ComponentType::Inductor => "Inductors",
                ComponentType::VoltageSource => "Voltage Sources",
                ComponentType::CurrentSource => "Current Sources",
                ComponentType::Diode => "Diodes",
                ComponentType::Mosfet { .. } => "MOSFETs",
                ComponentType::Bjt { .. } => "BJTs",
            };
            *type_counts.entry(type_name).or_insert(0) += 1;
        }

        for (type_name, count) in type_counts {
            println!("  {}: {}", type_name, count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new("test".to_string());
        assert_eq!(node.name, "test");
        assert_eq!(node.voltage, 0.0);
        assert!(!node.is_ground());

        let ground_node = Node::new("0".to_string());
        assert!(ground_node.is_ground());
    }

    #[test]
    fn test_component_creation() {
        let resistor = Component::new_resistor("R1".to_string(), "1".to_string(), "2".to_string(), 1000.0);
        assert_eq!(resistor.name, "R1");
        assert_eq!(resistor.value, 1000.0);
        assert_eq!(resistor.conductance().unwrap(), 0.001);
    }

    #[test]
    fn test_circuit_building() {
        let mut circuit = Circuit::new("Test Circuit".to_string());
        
        // Add nodes
        let node1 = circuit.add_node("1".to_string());
        let node2 = circuit.add_node("2".to_string());
        let ground = circuit.add_node("0".to_string());

        assert_eq!(node1, 0);
        assert_eq!(node2, 1);
        assert_eq!(ground, 2);
        assert_eq!(circuit.ground_node, Some(2));

        // Add components
        let resistor = Component::new_resistor("R1".to_string(), "1".to_string(), "2".to_string(), 1000.0);
        circuit.add_component(resistor).unwrap();

        assert_eq!(circuit.components.len(), 1);
        assert!(circuit.validate().is_ok());
    }
} 