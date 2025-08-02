# 电路建模

## 概述

RustSim的电路建模模块提供了完整的电路抽象表示，包括节点、元件、拓扑结构的管理。该模块是连接解析器和仿真器的桥梁，为后续的MNA分析提供了必要的数据结构。

## 电路抽象表示原理

### 基本概念

电路建模基于以下核心概念：
- **节点（Node）**：电路中的连接点，具有电压属性
- **元件（Component）**：电路中的功能单元，连接节点
- **拓扑结构**：节点和元件之间的连接关系
- **接地节点**：参考电位点，通常标记为"0"或"gnd"

### 数据结构设计

```rust
pub struct Circuit {
    pub title: String,
    pub nodes: Vec<Node>,
    pub components: Vec<Component>,
    pub node_map: HashMap<String, usize>,
    pub ground_node: Option<usize>,
}

pub struct Node {
    pub name: String,
    pub id: usize,
    pub voltage: f64,
}

pub struct Component {
    pub name: String,
    pub component_type: ComponentType,
    pub nodes: Vec<String>,
    pub value: f64,
    pub model: Option<String>,
}
```

## 元件建模和分类算法

### 元件类型定义

RustSim支持多种元件类型，每种类型都有特定的特性和行为：

```rust
pub enum ComponentType {
    Resistor,           // 电阻
    Capacitor,          // 电容
    Inductor,           // 电感
    VoltageSource,      // 电压源
    CurrentSource,      // 电流源
    Diode,              // 二极管
    Mosfet {            // MOSFET
        model_type: String,
        width: Option<f64>,
        length: Option<f64>,
    },
    Bjt {               // BJT
        model_type: String,
        area: Option<f64>,
    },
}
```

### 元件分类算法

```rust
impl ComponentType {
    /// 判断是否为线性元件
    pub fn is_linear(&self) -> bool {
        matches!(self, 
            ComponentType::Resistor | 
            ComponentType::Capacitor | 
            ComponentType::Inductor | 
            ComponentType::VoltageSource | 
            ComponentType::CurrentSource
        )
    }

    /// 判断是否为电源
    pub fn is_source(&self) -> bool {
        matches!(self, ComponentType::VoltageSource | ComponentType::CurrentSource)
    }

    /// 判断是否需要电流跟踪
    pub fn tracks_current(&self) -> bool {
        matches!(self, ComponentType::VoltageSource | ComponentType::Inductor)
    }
}
```

### 元件创建算法

```rust
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

    pub fn new_voltage_source(name: String, node_pos: String, node_neg: String, voltage: f64) -> Self {
        Component {
            name,
            component_type: ComponentType::VoltageSource,
            nodes: vec![node_pos, node_neg],
            value: voltage,
            model: None,
        }
    }
}
```

## 拓扑结构管理

### 节点管理算法

```rust
impl Circuit {
    /// 添加节点到电路
    pub fn add_node(&mut self, name: String) -> usize {
        // 检查节点是否已存在
        if let Some(&id) = self.node_map.get(&name) {
            return id;
        }
        
        let id = self.nodes.len();
        let node = Node::with_id(name.clone(), id);
        self.nodes.push(node);
        self.node_map.insert(name, id);
        
        // 检查是否为接地节点
        if name == "0" || name.to_lowercase() == "gnd" || name.to_lowercase() == "ground" {
            self.ground_node = Some(id);
        }
        
        id
    }

    /// 获取节点ID
    pub fn get_node_id(&self, name: &str) -> Option<usize> {
        self.node_map.get(name).copied()
    }

    /// 获取非接地节点
    pub fn non_ground_nodes(&self) -> Vec<usize> {
        self.nodes.iter()
            .filter(|node| Some(node.id) != self.ground_node)
            .map(|node| node.id)
            .collect()
    }
}
```

### 元件管理算法

```rust
impl Circuit {
    /// 添加元件到电路
    pub fn add_component(&mut self, component: Component) -> Result<()> {
        // 验证元件
        component.validate()?;
        
        // 确保所有节点都存在
        for node_name in &component.nodes {
            if !self.node_map.contains_key(node_name) {
                self.add_node(node_name.clone());
            }
        }
        
        self.components.push(component);
        Ok(())
    }

    /// 按类型获取元件
    pub fn components_of_type(&self, component_type: &ComponentType) -> Vec<&Component> {
        self.components.iter()
            .filter(|comp| std::mem::discriminant(&comp.component_type) == 
                           std::mem::discriminant(component_type))
            .collect()
    }

    /// 获取电压源
    pub fn voltage_sources(&self) -> Vec<&Component> {
        self.components.iter()
            .filter(|comp| matches!(comp.component_type, ComponentType::VoltageSource))
            .collect()
    }

    /// 获取电流源
    pub fn current_sources(&self) -> Vec<&Component> {
        self.components.iter()
            .filter(|comp| matches!(comp.component_type, ComponentType::CurrentSource))
            .collect()
    }

    /// 获取线性元件
    pub fn linear_components(&self) -> Vec<&Component> {
        self.components.iter()
            .filter(|comp| comp.component_type.is_linear())
            .collect()
    }
}
```

## 电路验证和完整性检查

### 验证算法

```rust
impl Component {
    pub fn validate(&self) -> Result<()> {
        // 检查节点数量
        match self.component_type {
            ComponentType::Resistor | 
            ComponentType::Capacitor | 
            ComponentType::Inductor |
            ComponentType::VoltageSource |
            ComponentType::CurrentSource => {
                if self.nodes.len() != 2 {
                    return Err(anyhow!("Component {} must have exactly 2 nodes", self.name));
                }
            }
            ComponentType::Diode => {
                if self.nodes.len() != 2 {
                    return Err(anyhow!("Diode {} must have exactly 2 nodes", self.name));
                }
            }
            _ => {
                // 其他元件类型的验证
            }
        }
        
        // 检查参数值
        match self.component_type {
            ComponentType::Resistor => {
                if self.value <= 0.0 {
                    return Err(anyhow!("Resistor {} must have positive resistance", self.name));
                }
            }
            ComponentType::Capacitor => {
                if self.value < 0.0 {
                    return Err(anyhow!("Capacitor {} must have non-negative capacitance", self.name));
                }
            }
            ComponentType::Inductor => {
                if self.value < 0.0 {
                    return Err(anyhow!("Inductor {} must have non-negative inductance", self.name));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

impl Circuit {
    pub fn validate(&self) -> Result<()> {
        // 检查是否有接地节点
        if self.ground_node.is_none() {
            return Err(anyhow!("Circuit must have a ground node (named '0', 'gnd', or 'ground')"));
        }
        
        // 检查是否有元件
        if self.components.is_empty() {
            return Err(anyhow!("Circuit must have at least one component"));
        }
        
        // 验证所有元件
        for component in &self.components {
            component.validate()?;
        }
        
        // 检查电路连通性
        self.check_connectivity()?;
        
        Ok(())
    }

    fn check_connectivity(&self) -> Result<()> {
        // 构建邻接表
        let mut adjacency = vec![Vec::new(); self.nodes.len()];
        
        for component in &self.components {
            if component.nodes.len() >= 2 {
                let node1 = self.get_node_id(&component.nodes[0]);
                let node2 = self.get_node_id(&component.nodes[1]);
                
                if let (Some(id1), Some(id2)) = (node1, node2) {
                    adjacency[id1].push(id2);
                    adjacency[id2].push(id1);
                }
            }
        }
        
        // 检查从接地节点可达所有节点
        if let Some(ground_id) = self.ground_node {
            let mut visited = vec![false; self.nodes.len()];
            self.dfs(ground_id, &adjacency, &mut visited);
            
            for (i, &visited) in visited.iter().enumerate() {
                if !visited {
                    return Err(anyhow!("Node {} is not connected to ground", self.nodes[i].name));
                }
            }
        }
        
        Ok(())
    }

    fn dfs(&self, node: usize, adjacency: &[Vec<usize>], visited: &mut [bool]) {
        visited[node] = true;
        for &neighbor in &adjacency[node] {
            if !visited[neighbor] {
                self.dfs(neighbor, adjacency, visited);
            }
        }
    }
}
```

## 性能优化

### 1. 索引优化
- 使用HashMap进行节点名称到ID的快速映射
- 按元件类型预分类，减少运行时过滤开销

### 2. 内存优化
- 字符串复用，减少重复存储
- 智能指针管理大型数据结构

### 3. 算法优化
- 高效的连通性检查算法
- 增量式电路更新

## 扩展性设计

### 1. 新元件类型支持
通过扩展ComponentType枚举和相应的验证逻辑，可以轻松添加新的元件类型。

### 2. 多端口元件支持
当前设计主要支持二端口元件，可以通过扩展nodes字段支持多端口元件。

### 3. 参数化模型支持
通过扩展model字段，可以支持复杂的参数化元件模型。

## 测试和验证

### 1. 单元测试
每个元件类型和电路操作都有对应的单元测试。

### 2. 集成测试
完整的电路构建和验证测试。

### 3. 性能测试
大规模电路的构建和验证性能测试。

## 总结

RustSim的电路建模模块提供了完整、高效的电路抽象表示。通过清晰的类型定义、完善的验证算法和优化的数据结构，为后续的MNA分析提供了坚实的基础。模块化设计确保了良好的扩展性，可以轻松支持新的元件类型和分析需求。 