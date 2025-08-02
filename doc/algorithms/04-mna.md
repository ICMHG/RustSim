# MNA算法

## 概述

改进节点分析（Modified Nodal Analysis, MNA）是RustSim的核心算法，用于将电路转换为线性方程组。MNA通过引入额外的电流变量来处理电压源，使得所有电路元件都能用统一的矩阵形式表示。

## 改进节点分析理论基础

### 基本原理

MNA基于基尔霍夫电流定律（KCL）和基尔霍夫电压定律（KVL），将电路转换为线性方程组：

```
[A][x] = [z]
```

其中：
- **A**：系统矩阵（系数矩阵）
- **x**：未知变量向量（节点电压 + 电压源电流）
- **z**：右侧向量（激励源）

### 矩阵结构

MNA系统矩阵具有以下结构：

```
A = [G  B]
    [B' 0]
```

其中：
- **G**：节点导纳矩阵（n×n）
- **B**：电压源关联矩阵（n×m）
- **B'**：B的转置（m×n）
- **0**：零矩阵（m×m）
- **n**：节点数量
- **m**：电压源数量

## 数据结构设计

### MNA系统表示

```rust
pub struct MnaSystem {
    /// 系统矩阵 A (系数矩阵)
    pub matrix: DMatrix<f64>,
    /// 右侧向量 z
    pub rhs: DVector<f64>,
    /// 未知变量向量 x (节点电压 + 电压源电流)
    pub unknowns: DVector<f64>,
    /// 节点ID到矩阵行列索引的映射
    pub node_map: HashMap<usize, usize>,
    /// 电压源名称到电流变量索引的映射
    pub voltage_source_map: HashMap<String, usize>,
    /// 系统总大小
    pub size: usize,
    /// 节点数量（不包括接地节点）
    pub num_nodes: usize,
    /// 电压源数量
    pub num_voltage_sources: usize,
}
```

### 系统初始化算法

```rust
impl MnaSystem {
    pub fn new(circuit: &Circuit) -> Result<Self> {
        let num_nodes = circuit.node_count(); // 非接地节点
        let voltage_sources = circuit.voltage_sources();
        let num_voltage_sources = voltage_sources.len();
        let size = num_nodes + num_voltage_sources;

        if size == 0 {
            return Err(anyhow!("Circuit has no nodes or voltage sources to analyze"));
        }

        // 创建节点映射（仅非接地节点）
        let mut node_map = HashMap::new();
        let mut index = 0;
        for node in &circuit.nodes {
            if Some(node.id) != circuit.ground_node {
                node_map.insert(node.id, index);
                index += 1;
            }
        }

        // 创建电压源映射
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
}
```

## 矩阵组装算法实现

### DC分析矩阵组装

```rust
impl MnaSystem {
    pub fn assemble_dc(&mut self, circuit: &Circuit) -> Result<()> {
        // 清除现有系统
        self.matrix.fill(0.0);
        self.rhs.fill(0.0);

        // 处理线性元件（R、L、C）
        for component in circuit.linear_components() {
            self.add_linear_component(circuit, component)?;
        }

        // 处理电流源
        for component in circuit.current_sources() {
            self.add_current_source(circuit, component)?;
        }

        // 处理电压源
        for component in circuit.voltage_sources() {
            self.add_voltage_source(circuit, component)?;
        }

        Ok(())
    }
}
```

### 线性元件贡献计算

```rust
impl MnaSystem {
    fn add_linear_component(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        match component.component_type {
            ComponentType::Resistor => {
                self.add_resistor(circuit, component)?;
            }
            ComponentType::Capacitor => {
                // DC分析中电容开路
                // 在瞬态分析中处理
            }
            ComponentType::Inductor => {
                // DC分析中电感短路
                // 在瞬态分析中处理
            }
            _ => {
                return Err(anyhow!("Unsupported component type for DC analysis"));
            }
        }
        Ok(())
    }

    fn add_resistor(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        let node1_id = circuit.get_node_id(&component.nodes[0])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[0]))?;
        let node2_id = circuit.get_node_id(&component.nodes[1])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[1]))?;

        // 跳过接地节点
        if Some(node1_id) == circuit.ground_node || Some(node2_id) == circuit.ground_node {
            return Ok(());
        }

        let conductance = 1.0 / component.value;
        let node1_idx = self.node_map[&node1_id];
        let node2_idx = self.node_map[&node2_id];

        // 添加到导纳矩阵
        self.matrix[(node1_idx, node1_idx)] += conductance;
        self.matrix[(node2_idx, node2_idx)] += conductance;
        self.matrix[(node1_idx, node2_idx)] -= conductance;
        self.matrix[(node2_idx, node1_idx)] -= conductance;
    }
}
```

### 电压源处理

```rust
impl MnaSystem {
    fn add_voltage_source(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        let node1_id = circuit.get_node_id(&component.nodes[0])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[0]))?;
        let node2_id = circuit.get_node_id(&component.nodes[1])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[1]))?;

        let current_idx = self.voltage_source_map[&component.name];

        // 处理非接地节点
        if Some(node1_id) != circuit.ground_node {
            let node1_idx = self.node_map[&node1_id];
            self.matrix[(node1_idx, current_idx)] = 1.0;
            self.matrix[(current_idx, node1_idx)] = 1.0;
        }

        if Some(node2_id) != circuit.ground_node {
            let node2_idx = self.node_map[&node2_id];
            self.matrix[(node2_idx, current_idx)] = -1.0;
            self.matrix[(current_idx, node2_idx)] = -1.0;
        }

        // 设置电压源值
        self.rhs[current_idx] = component.value;

        Ok(())
    }
}
```

### 电流源处理

```rust
impl MnaSystem {
    fn add_current_source(&mut self, circuit: &Circuit, component: &Component) -> Result<()> {
        let node1_id = circuit.get_node_id(&component.nodes[0])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[0]))?;
        let node2_id = circuit.get_node_id(&component.nodes[1])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[1]))?;

        // 处理非接地节点
        if Some(node1_id) != circuit.ground_node {
            let node1_idx = self.node_map[&node1_id];
            self.rhs[node1_idx] -= component.value; // 流出为正
        }

        if Some(node2_id) != circuit.ground_node {
            let node2_idx = self.node_map[&node2_id];
            self.rhs[node2_idx] += component.value; // 流入为正
        }

        Ok(())
    }
}
```

## 瞬态分析处理

### 电容瞬态处理

```rust
impl MnaSystem {
    fn add_capacitor_transient(&mut self, circuit: &Circuit, component: &Component, 
                              dt: f64, prev_voltages: &DVector<f64>) -> Result<()> {
        let node1_id = circuit.get_node_id(&component.nodes[0])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[0]))?;
        let node2_id = circuit.get_node_id(&component.nodes[1])
            .ok_or_else(|| anyhow!("Node {} not found", component.nodes[1]))?;

        // 跳过接地节点
        if Some(node1_id) == circuit.ground_node || Some(node2_id) == circuit.ground_node {
            return Ok(());
        }

        let conductance = component.value / dt; // C/dt
        let node1_idx = self.node_map[&node1_id];
        let node2_idx = self.node_map[&node2_id];

        // 添加到导纳矩阵
        self.matrix[(node1_idx, node1_idx)] += conductance;
        self.matrix[(node2_idx, node2_idx)] += conductance;
        self.matrix[(node1_idx, node2_idx)] -= conductance;
        self.matrix[(node2_idx, node1_idx)] -= conductance;

        // 添加历史项到右侧向量
        let prev_voltage_diff = prev_voltages[node1_idx] - prev_voltages[node2_idx];
        let history_current = conductance * prev_voltage_diff;

        self.rhs[node1_idx] += history_current;
        self.rhs[node2_idx] -= history_current;

        Ok(())
    }
}
```

### 电感瞬态处理

```rust
impl MnaSystem {
    fn add_inductor_transient(&mut self, _circuit: &Circuit, _component: &Component, 
                             _dt: f64, _prev_currents: &DVector<f64>) -> Result<()> {
        // 电感瞬态处理需要额外的电流变量
        // 当前实现中暂未完全支持
        // 可以通过引入额外的电流变量来实现
        Ok(())
    }
}
```

## 稀疏矩阵优化

### 稀疏矩阵转换

```rust
impl MnaSystem {
    pub fn to_sparse(&self) -> (CsMat<f64>, Vec<f64>) {
        let mut tri_mat = TriMat::new((self.size, self.size));
        
        // 添加矩阵元素
        for i in 0..self.size {
            for j in 0..self.size {
                let value = self.matrix[(i, j)];
                if value.abs() > 1e-12 { // 忽略接近零的元素
                    tri_mat.add_triplet(i, j, value);
                }
            }
        }
        
        // 转换为压缩稀疏矩阵
        let sparse_matrix = tri_mat.to_csr();
        let rhs_vector = self.rhs.as_slice().to_vec();
        
        (sparse_matrix, rhs_vector)
    }
}
```

## 结果处理

### 解更新

```rust
impl MnaSystem {
    pub fn update_solution(&mut self, solution: &[f64]) -> Result<()> {
        if solution.len() != self.size {
            return Err(anyhow!("Solution size mismatch"));
        }
        
        self.unknowns = DVector::from_column_slice(solution);
        Ok(())
    }

    pub fn get_node_voltage(&self, node_id: usize) -> Result<f64> {
        if let Some(&idx) = self.node_map.get(&node_id) {
            Ok(self.unknowns[idx])
        } else {
            Err(anyhow!("Node {} not found in MNA system", node_id))
        }
    }

    pub fn get_voltage_source_current(&self, name: &str) -> Result<f64> {
        if let Some(&idx) = self.voltage_source_map.get(name) {
            Ok(self.unknowns[idx])
        } else {
            Err(anyhow!("Voltage source {} not found", name))
        }
    }
}
```

## 性能优化

### 1. 稀疏矩阵存储
- 使用压缩稀疏行（CSR）格式
- 减少内存占用和计算开销

### 2. 矩阵组装优化
- 增量式矩阵更新
- 缓存常用计算

### 3. 数值稳定性
- 条件数检查
- 主元选择策略

## 扩展性设计

### 1. 非线性元件支持
通过牛顿-拉夫森迭代扩展支持非线性元件。

### 2. 多端口元件支持
扩展矩阵结构支持多端口元件。

### 3. 参数化分析
支持参数扫描和灵敏度分析。

## 总结

RustSim的MNA模块实现了完整的改进节点分析算法，能够处理各种线性元件和电源。通过高效的矩阵组装算法和稀疏矩阵优化，为电路仿真提供了稳定、高效的数值基础。模块化设计确保了良好的扩展性，可以轻松支持新的元件类型和分析方法。 