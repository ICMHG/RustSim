# 仿真算法

## 概述

RustSim的仿真器模块是系统的核心控制模块，负责协调各个算法模块，实现不同类型的电路分析。该模块支持工作点分析、DC扫描分析和瞬态分析，并提供了完整的收敛性管理和结果收集功能。

## 仿真器架构

### 核心数据结构

```rust
pub struct Simulator {
    circuit: Option<Circuit>,
    mna_system: Option<MnaSystem>,
    solver: LinearSolver,
    results: Option<SimulationResult>,
    config: SimulatorConfig,
}

pub struct SimulatorConfig {
    pub solver_config: SolverConfig,
    pub max_iterations: usize,
    pub convergence_tolerance: f64,
    pub auto_select_solver: bool,
    pub store_intermediate_results: bool,
}

pub struct SimulationResult {
    pub analysis_type: AnalysisType,
    pub time_points: Vec<f64>,
    pub node_voltages: HashMap<String, Vec<f64>>,
    pub currents: HashMap<String, Vec<f64>>,
    pub convergence_info: Vec<ConvergenceInfo>,
    pub total_time: f64,
    pub success: bool,
}
```

### 仿真器初始化

```rust
impl Simulator {
    pub fn new() -> Self {
        Simulator {
            circuit: None,
            mna_system: None,
            solver: LinearSolver::new(),
            results: None,
            config: SimulatorConfig::default(),
        }
    }

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
}
```

## 工作点分析算法

### 算法原理

工作点分析（Operating Point Analysis）用于计算电路在DC条件下的稳态解，即所有节点电压和元件电流的静态值。

### 实现流程

```rust
impl Simulator {
    pub fn run_operating_point(&mut self) -> Result<()> {
        let start_time = Instant::now();
        
        // 获取电路
        let circuit = self.circuit.as_ref()
            .ok_or_else(|| anyhow!("No circuit loaded"))?;
        
        info!("Starting operating point analysis");
        
        // 验证电路
        circuit.validate()?;
        
        // 创建MNA系统
        let mut mna_system = MnaSystem::new(circuit)?;
        
        // 组装DC分析矩阵
        mna_system.assemble_dc(circuit)?;
        
        // 转换为稀疏矩阵
        let (sparse_matrix, rhs_vector) = mna_system.to_sparse();
        
        // 求解线性系统
        let (solution, solver_stats) = self.solver.solve_sparse(&sparse_matrix, &rhs_vector)?;
        
        // 更新MNA系统解
        mna_system.update_solution(&solution)?;
        
        // 收集结果
        let mut node_voltages = HashMap::new();
        let mut currents = HashMap::new();
        
        // 收集节点电压
        for node in &circuit.nodes {
            if Some(node.id) != circuit.ground_node {
                let voltage = mna_system.get_node_voltage(node.id)?;
                node_voltages.insert(node.name.clone(), vec![voltage]);
            }
        }
        
        // 收集电压源电流
        for vs in circuit.voltage_sources() {
            let current = mna_system.get_voltage_source_current(&vs.name)?;
            currents.insert(vs.name.clone(), vec![current]);
        }
        
        let total_time = start_time.elapsed().as_secs_f64();
        
        // 创建仿真结果
        let result = SimulationResult {
            analysis_type: AnalysisType::Operating,
            time_points: vec![0.0], // 工作点分析只有一个时间点
            node_voltages,
            currents,
            convergence_info: vec![ConvergenceInfo {
                iteration: 1,
                residual_norm: solver_stats.residual_norm,
                solve_time: solver_stats.solve_time,
                solver_method: format!("{:?}", solver_stats.method_used),
            }],
            total_time,
            success: solver_stats.success,
        };
        
        self.results = Some(result);
        
        info!("Operating point analysis completed in {:.3}ms", total_time * 1000.0);
        Ok(())
    }
}
```

## DC扫描分析算法

### 算法原理

DC扫描分析用于研究电路参数变化对输出响应的影响，通常用于分析电压源或电流源的变化对电路性能的影响。

### 实现流程

```rust
impl Simulator {
    pub fn run_dc_sweep(&mut self, source_name: &str, start: f64, stop: f64, step: f64) -> Result<()> {
        let start_time = Instant::now();
        
        // 获取电路
        let circuit = self.circuit.as_ref()
            .ok_or_else(|| anyhow!("No circuit loaded"))?;
        
        info!("Starting DC sweep analysis for source {}", source_name);
        
        // 验证电路
        circuit.validate()?;
        
        // 查找电压源
        let voltage_source = circuit.voltage_sources()
            .iter()
            .find(|vs| vs.name == source_name)
            .ok_or_else(|| anyhow!("Voltage source {} not found", source_name))?;
        
        // 计算扫描点数
        let num_points = ((stop - start) / step).ceil() as usize + 1;
        let mut time_points = Vec::with_capacity(num_points);
        let mut node_voltages = HashMap::new();
        let mut currents = HashMap::new();
        let mut convergence_info = Vec::new();
        
        // 初始化结果存储
        for node in &circuit.nodes {
            if Some(node.id) != circuit.ground_node {
                node_voltages.insert(node.name.clone(), Vec::with_capacity(num_points));
            }
        }
        
        for vs in circuit.voltage_sources() {
            currents.insert(vs.name.clone(), Vec::with_capacity(num_points));
        }
        
        // 执行扫描
        for i in 0..num_points {
            let source_value = start + (i as f64) * step;
            
            // 创建临时电路副本并修改电压源值
            let mut temp_circuit = circuit.clone();
            if let Some(vs) = temp_circuit.components.iter_mut()
                .find(|c| c.name == source_name) {
                vs.value = source_value;
            }
            
            // 创建MNA系统
            let mut mna_system = MnaSystem::new(&temp_circuit)?;
            
            // 组装DC分析矩阵
            mna_system.assemble_dc(&temp_circuit)?;
            
            // 转换为稀疏矩阵
            let (sparse_matrix, rhs_vector) = mna_system.to_sparse();
            
            // 求解线性系统
            let (solution, solver_stats) = self.solver.solve_sparse(&sparse_matrix, &rhs_vector)?;
            
            // 更新MNA系统解
            mna_system.update_solution(&solution)?;
            
            // 收集结果
            time_points.push(source_value);
            
            for node in &temp_circuit.nodes {
                if Some(node.id) != temp_circuit.ground_node {
                    let voltage = mna_system.get_node_voltage(node.id)?;
                    node_voltages.get_mut(&node.name).unwrap().push(voltage);
                }
            }
            
            for vs in temp_circuit.voltage_sources() {
                let current = mna_system.get_voltage_source_current(&vs.name)?;
                currents.get_mut(&vs.name).unwrap().push(current);
            }
            
            convergence_info.push(ConvergenceInfo {
                iteration: 1,
                residual_norm: solver_stats.residual_norm,
                solve_time: solver_stats.solve_time,
                solver_method: format!("{:?}", solver_stats.method_used),
            });
        }
        
        let total_time = start_time.elapsed().as_secs_f64();
        
        // 创建仿真结果
        let result = SimulationResult {
            analysis_type: AnalysisType::DcSweep {
                parameter: source_name.to_string(),
                start,
                stop,
                step,
            },
            time_points,
            node_voltages,
            currents,
            convergence_info,
            total_time,
            success: true,
        };
        
        self.results = Some(result);
        
        info!("DC sweep analysis completed in {:.3}ms", total_time * 1000.0);
        Ok(())
    }
}
```

## 瞬态分析算法

### 算法原理

瞬态分析用于研究电路在时间域内的动态响应，通过数值积分方法求解微分方程组。

### 时间步进算法

```rust
impl Simulator {
    pub fn run_transient_analysis(&mut self, tstep: f64, tstop: f64) -> Result<()> {
        let start_time = Instant::now();
        
        // 获取电路
        let circuit = self.circuit.as_ref()
            .ok_or_else(|| anyhow!("No circuit loaded"))?;
        
        info!("Starting transient analysis: tstep={}, tstop={}", tstep, tstop);
        
        // 验证电路
        circuit.validate()?;
        
        // 计算时间点数
        let num_points = (tstop / tstep).ceil() as usize + 1;
        let mut time_points = Vec::with_capacity(num_points);
        let mut node_voltages = HashMap::new();
        let mut currents = HashMap::new();
        let mut convergence_info = Vec::new();
        
        // 初始化结果存储
        for node in &circuit.nodes {
            if Some(node.id) != circuit.ground_node {
                node_voltages.insert(node.name.clone(), Vec::with_capacity(num_points));
            }
        }
        
        for vs in circuit.voltage_sources() {
            currents.insert(vs.name.clone(), Vec::with_capacity(num_points));
        }
        
        // 初始条件（工作点分析）
        let mut prev_voltages = self.get_initial_conditions(circuit)?;
        
        // 时间步进
        for i in 0..num_points {
            let time = i as f64 * tstep;
            time_points.push(time);
            
            // 创建MNA系统
            let mut mna_system = MnaSystem::new(circuit)?;
            
            // 组装瞬态分析矩阵
            mna_system.assemble_transient(circuit, tstep, &prev_voltages)?;
            
            // 转换为稀疏矩阵
            let (sparse_matrix, rhs_vector) = mna_system.to_sparse();
            
            // 求解线性系统
            let (solution, solver_stats) = self.solver.solve_sparse(&sparse_matrix, &rhs_vector)?;
            
            // 更新MNA系统解
            mna_system.update_solution(&solution)?;
            
            // 收集结果
            for node in &circuit.nodes {
                if Some(node.id) != circuit.ground_node {
                    let voltage = mna_system.get_node_voltage(node.id)?;
                    node_voltages.get_mut(&node.name).unwrap().push(voltage);
                    prev_voltages[mna_system.node_map[&node.id]] = voltage;
                }
            }
            
            for vs in circuit.voltage_sources() {
                let current = mna_system.get_voltage_source_current(&vs.name)?;
                currents.get_mut(&vs.name).unwrap().push(current);
            }
            
            convergence_info.push(ConvergenceInfo {
                iteration: 1,
                residual_norm: solver_stats.residual_norm,
                solve_time: solver_stats.solve_time,
                solver_method: format!("{:?}", solver_stats.method_used),
            });
        }
        
        let total_time = start_time.elapsed().as_secs_f64();
        
        // 创建仿真结果
        let result = SimulationResult {
            analysis_type: AnalysisType::Transient { tstep, tstop },
            time_points,
            node_voltages,
            currents,
            convergence_info,
            total_time,
            success: true,
        };
        
        self.results = Some(result);
        
        info!("Transient analysis completed in {:.3}ms", total_time * 1000.0);
        Ok(())
    }

    fn get_initial_conditions(&self, circuit: &Circuit) -> Result<DVector<f64>> {
        // 执行工作点分析获取初始条件
        let mut temp_simulator = Simulator::new();
        temp_simulator.load_netlist_from_parsed(circuit.clone().into())?;
        temp_simulator.run_operating_point()?;
        
        // 提取节点电压作为初始条件
        let num_nodes = circuit.node_count();
        let mut initial_voltages = DVector::zeros(num_nodes);
        
        if let Some(results) = temp_simulator.get_results() {
            for (node_name, voltages) in &results.node_voltages {
                if let Some(node_id) = circuit.get_node_id(node_name) {
                    if let Some(idx) = temp_simulator.mna_system.as_ref()
                        .and_then(|mna| mna.node_map.get(&node_id)) {
                        initial_voltages[*idx] = voltages[0];
                    }
                }
            }
        }
        
        Ok(initial_voltages)
    }
}
```

## 收敛性分析

### 收敛判据

```rust
impl Simulator {
    fn check_convergence(&self, residual_norm: f64, iteration: usize) -> bool {
        // 残差收敛判据
        let residual_converged = residual_norm < self.config.convergence_tolerance;
        
        // 迭代次数限制
        let iteration_limit = iteration < self.config.max_iterations;
        
        residual_converged && iteration_limit
    }

    fn adaptive_time_step(&self, convergence_info: &[ConvergenceInfo], 
                         current_step: f64) -> f64 {
        // 基于收敛历史调整时间步长
        if convergence_info.len() < 2 {
            return current_step;
        }
        
        let last_residual = convergence_info.last().unwrap().residual_norm;
        let prev_residual = convergence_info[convergence_info.len() - 2].residual_norm;
        
        // 如果残差增加，减小时间步长
        if last_residual > prev_residual * 1.1 {
            current_step * 0.8
        } else if last_residual < prev_residual * 0.9 {
            // 如果残差快速减小，可以增大时间步长
            current_step * 1.2
        } else {
            current_step
        }
    }
}
```

## 数值积分方法

### 后向欧拉法

对于电容的瞬态分析，使用后向欧拉法：

```
i(t) = C * (v(t) - v(t-dt)) / dt
```

这可以重写为：

```
v(t) = v(t-dt) + dt * i(t) / C
```

### 梯形法则

更精确的积分方法，但需要迭代求解：

```
v(t) = v(t-dt) + dt * (i(t) + i(t-dt)) / (2 * C)
```

## 性能优化

### 1. 矩阵重用
- 在瞬态分析中重用矩阵结构
- 只更新变化的元素

### 2. 自适应时间步长
- 根据收敛性调整时间步长
- 平衡精度和效率

### 3. 并行计算
- 支持多线程求解
- GPU加速（未来扩展）

## 扩展性设计

### 1. 新分析类型
通过扩展AnalysisType枚举，可以轻松添加新的分析类型。

### 2. 非线性分析
通过牛顿-拉夫森迭代扩展支持非线性元件。

### 3. 参数化分析
支持蒙特卡洛分析和参数扫描。

## 总结

RustSim的仿真器模块提供了完整的电路分析功能，支持工作点分析、DC扫描和瞬态分析。通过高效的算法实现和收敛性管理，为电路仿真提供了稳定、准确的数值解。模块化设计确保了良好的扩展性，可以轻松支持新的分析类型和算法改进。 