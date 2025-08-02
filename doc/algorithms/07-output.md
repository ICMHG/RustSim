# 输出处理

## 概述

RustSim的输出处理模块负责将仿真结果转换为多种格式，并提供统计分析功能。该模块支持CSV、JSON、MATLAB和Python等多种输出格式，能够生成可视化脚本和详细的统计报告。

## 输出处理器架构

### 核心数据结构

```rust
pub struct OutputProcessor {
    config: OutputConfig,
}

pub struct OutputConfig {
    pub precision: usize,
    pub scientific_notation: bool,
    pub include_metadata: bool,
    pub compress_output: bool,
}

pub struct ResultStatistics {
    pub node_voltage_stats: HashMap<String, SignalStats>,
    pub current_stats: HashMap<String, SignalStats>,
    pub analysis_metadata: AnalysisMetadata,
}

pub struct SignalStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std_dev: f64,
    pub rms: f64,
    pub peak_to_peak: f64,
}

pub struct AnalysisMetadata {
    pub analysis_type: String,
    pub total_points: usize,
    pub time_span: f64,
    pub convergence_rate: f64,
    pub simulation_time: f64,
}
```

### 输出处理器初始化

```rust
impl OutputProcessor {
    pub fn new() -> Self {
        OutputProcessor {
            config: OutputConfig::default(),
        }
    }

    pub fn with_config(config: OutputConfig) -> Self {
        OutputProcessor { config }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            precision: 6,
            scientific_notation: false,
            include_metadata: true,
            compress_output: false,
        }
    }
}
```

## 多格式导出算法

### CSV格式导出

CSV格式是最常用的数据交换格式，便于在电子表格软件中查看和分析。

```rust
impl OutputProcessor {
    pub fn export_csv(&self, results: &SimulationResult, filename: &str) -> Result<()> {
        let file = File::create(filename)?;
        let mut writer = csv::Writer::from_writer(file);

        // 写入元数据（如果启用）
        if self.config.include_metadata {
            self.write_csv_metadata(&mut writer, results)?;
        }

        // 创建表头
        let mut header = vec!["time".to_string()];
        
        // 排序节点名称以确保一致的输出
        let mut node_names: Vec<_> = results.node_voltages.keys().collect();
        node_names.sort();
        header.extend(node_names.iter().map(|name| format!("V({})", name)));
        
        // 添加电流列
        let mut current_names: Vec<_> = results.currents.keys().collect();
        current_names.sort();
        header.extend(current_names.iter().map(|name| format!("I({})", name)));
        
        writer.write_record(&header)?;

        // 写入数据行
        for (i, &time) in results.time_points.iter().enumerate() {
            let mut row = vec![self.format_number(time)];
            
            // 添加节点电压
            for node_name in &node_names {
                if let Some(voltages) = results.node_voltages.get(*node_name) {
                    if i < voltages.len() {
                        row.push(self.format_number(voltages[i]));
                    } else {
                        row.push("".to_string());
                    }
                }
            }
            
            // 添加电流
            for current_name in &current_names {
                if let Some(currents) = results.currents.get(*current_name) {
                    if i < currents.len() {
                        row.push(self.format_number(currents[i]));
                    } else {
                        row.push("".to_string());
                    }
                }
            }
            
            writer.write_record(&row)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn write_csv_metadata(&self, writer: &mut csv::Writer<File>, results: &SimulationResult) -> Result<()> {
        // 写入分析类型
        writer.write_record(&["# Analysis Type", &format!("{:?}", results.analysis_type)])?;
        
        // 写入时间点数
        writer.write_record(&["# Total Points", &results.time_points.len().to_string()])?;
        
        // 写入仿真时间
        writer.write_record(&["# Simulation Time", &format!("{:.6}s", results.total_time)])?;
        
        // 写入收敛信息
        if let Some(last_conv) = results.convergence_info.last() {
            writer.write_record(&["# Final Residual", &format!("{:.2e}", last_conv.residual_norm)])?;
            writer.write_record(&["# Solver Method", &last_conv.solver_method])?;
        }
        
        // 空行分隔
        writer.write_record(&["", ""])?;
        Ok(())
    }
}
```

### JSON格式导出

JSON格式便于程序化处理和数据交换，支持结构化数据存储。

```rust
impl OutputProcessor {
    pub fn export_json(&self, results: &SimulationResult, filename: &str) -> Result<()> {
        let file = File::create(filename)?;
        let mut writer = std::io::BufWriter::new(file);
        
        // 创建JSON结构
        let json_data = serde_json::json!({
            "metadata": {
                "analysis_type": format!("{:?}", results.analysis_type),
                "total_points": results.time_points.len(),
                "simulation_time": results.total_time,
                "success": results.success
            },
            "data": {
                "time_points": results.time_points,
                "node_voltages": results.node_voltages,
                "currents": results.currents
            },
            "convergence": {
                "iterations": results.convergence_info.len(),
                "final_residual": results.convergence_info.last()
                    .map(|c| c.residual_norm)
                    .unwrap_or(0.0),
                "solver_method": results.convergence_info.last()
                    .map(|c| c.solver_method.clone())
                    .unwrap_or_default()
            }
        });
        
        // 写入JSON文件
        serde_json::to_writer_pretty(&mut writer, &json_data)?;
        writer.flush()?;
        
        Ok(())
    }
}
```

### MATLAB脚本导出

MATLAB脚本格式便于在MATLAB中进行进一步分析和可视化。

```rust
impl OutputProcessor {
    pub fn export_matlab(&self, results: &SimulationResult, filename: &str) -> Result<()> {
        let file = File::create(filename)?;
        let mut writer = std::io::BufWriter::new(file);
        
        // 写入MATLAB脚本头部
        writeln!(writer, "% RustSim Simulation Results")?;
        writeln!(writer, "% Generated on: {}", chrono::Utc::now())?;
        writeln!(writer, "% Analysis Type: {:?}", results.analysis_type)?;
        writeln!(writer)?;
        
        // 写入时间向量
        writeln!(writer, "% Time vector")?;
        writeln!(writer, "t = [{}];", 
                results.time_points.iter()
                    .map(|&x| self.format_number(x))
                    .collect::<Vec<_>>()
                    .join(", "))?;
        writeln!(writer)?;
        
        // 写入节点电压数据
        writeln!(writer, "% Node voltages")?;
        for (node_name, voltages) in &results.node_voltages {
            writeln!(writer, "V_{} = [{}];", 
                    node_name.replace("-", "_"),
                    voltages.iter()
                        .map(|&x| self.format_number(x))
                        .collect::<Vec<_>>()
                        .join(", "))?;
        }
        writeln!(writer)?;
        
        // 写入电流数据
        writeln!(writer, "% Currents")?;
        for (current_name, currents) in &results.currents {
            writeln!(writer, "I_{} = [{}];", 
                    current_name.replace("-", "_"),
                    currents.iter()
                        .map(|&x| self.format_number(x))
                        .collect::<Vec<_>>()
                        .join(", "))?;
        }
        writeln!(writer)?;
        
        // 写入绘图命令
        writeln!(writer, "% Plotting commands")?;
        writeln!(writer, "figure;")?;
        writeln!(writer, "subplot(2,1,1);")?;
        writeln!(writer, "plot(t, [{}]);", 
                results.node_voltages.keys()
                    .map(|name| format!("V_{}", name.replace("-", "_")))
                    .collect::<Vec<_>>()
                    .join(", "))?;
        writeln!(writer, "title('Node Voltages');")?;
        writeln!(writer, "xlabel('Time (s)');")?;
        writeln!(writer, "ylabel('Voltage (V)');")?;
        writeln!(writer, "legend({});", 
                results.node_voltages.keys()
                    .map(|name| format!("'V({})'", name))
                    .collect::<Vec<_>>()
                    .join(", "))?;
        writeln!(writer, "grid on;")?;
        writeln!(writer)?;
        
        writeln!(writer, "subplot(2,1,2);")?;
        writeln!(writer, "plot(t, [{}]);", 
                results.currents.keys()
                    .map(|name| format!("I_{}", name.replace("-", "_")))
                    .collect::<Vec<_>>()
                    .join(", "))?;
        writeln!(writer, "title('Currents');")?;
        writeln!(writer, "xlabel('Time (s)');")?;
        writeln!(writer, "ylabel('Current (A)');")?;
        writeln!(writer, "legend({});", 
                results.currents.keys()
                    .map(|name| format!("'I({})'", name))
                    .collect::<Vec<_>>()
                    .join(", "))?;
        writeln!(writer, "grid on;")?;
        
        writer.flush()?;
        Ok(())
    }
}
```

### Python脚本导出

Python脚本格式便于在Python中进行数据分析和可视化，特别适合使用NumPy和Matplotlib。

```rust
impl OutputProcessor {
    pub fn export_python(&self, results: &SimulationResult, filename: &str) -> Result<()> {
        let file = File::create(filename)?;
        let mut writer = std::io::BufWriter::new(file);
        
        // 写入Python脚本头部
        writeln!(writer, "#!/usr/bin/env python3")?;
        writeln!(writer, "# -*- coding: utf-8 -*-")?;
        writeln!(writer, "\"\"\"")?;
        writeln!(writer, "RustSim Simulation Results")?;
        writeln!(writer, "Generated on: {}", chrono::Utc::now())?;
        writeln!(writer, "Analysis Type: {:?}", results.analysis_type)?;
        writeln!(writer, "\"\"\"")?;
        writeln!(writer)?;
        
        // 导入必要的库
        writeln!(writer, "import numpy as np")?;
        writeln!(writer, "import matplotlib.pyplot as plt")?;
        writeln!(writer, "from matplotlib import rcParams")?;
        writeln!(writer)?;
        
        // 设置绘图样式
        writeln!(writer, "# Set plot style")?;
        writeln!(writer, "rcParams['font.size'] = 10")?;
        writeln!(writer, "rcParams['figure.figsize'] = (10, 8)")?;
        writeln!(writer)?;
        
        // 写入时间数据
        writeln!(writer, "# Time data")?;
        writeln!(writer, "t = np.array([{}])", 
                results.time_points.iter()
                    .map(|&x| self.format_number(x))
                    .collect::<Vec<_>>()
                    .join(", "))?;
        writeln!(writer)?;
        
        // 写入节点电压数据
        writeln!(writer, "# Node voltage data")?);
        for (node_name, voltages) in &results.node_voltages {
            writeln!(writer, "V_{} = np.array([{}])", 
                    node_name.replace("-", "_"),
                    voltages.iter()
                        .map(|&x| self.format_number(x))
                        .collect::<Vec<_>>()
                        .join(", "))?;
        }
        writeln!(writer)?;
        
        // 写入电流数据
        writeln!(writer, "# Current data")?);
        for (current_name, currents) in &results.currents {
            writeln!(writer, "I_{} = np.array([{}])", 
                    current_name.replace("-", "_"),
                    currents.iter()
                        .map(|&x| self.format_number(x))
                        .collect::<Vec<_>>()
                        .join(", "))?;
        }
        writeln!(writer)?;
        
        // 写入绘图代码
        writeln!(writer, "# Create plots")?);
        writeln!(writer, "fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 8))")?;
        writeln!(writer)?;
        
        // 电压图
        writeln!(writer, "# Plot node voltages")?);
        for (node_name, voltages) in &results.node_voltages {
            writeln!(writer, "ax1.plot(t, V_{}, label='V({})')", 
                    node_name.replace("-", "_"), node_name)?;
        }
        writeln!(writer, "ax1.set_title('Node Voltages')")?;
        writeln!(writer, "ax1.set_xlabel('Time (s)')")?;
        writeln!(writer, "ax1.set_ylabel('Voltage (V)')")?;
        writeln!(writer, "ax1.legend()")?;
        writeln!(writer, "ax1.grid(True)")?;
        writeln!(writer)?;
        
        // 电流图
        writeln!(writer, "# Plot currents")?);
        for (current_name, currents) in &results.currents {
            writeln!(writer, "ax2.plot(t, I_{}, label='I({})')", 
                    current_name.replace("-", "_"), current_name)?;
        }
        writeln!(writer, "ax2.set_title('Currents')")?;
        writeln!(writer, "ax2.set_xlabel('Time (s)')")?;
        writeln!(writer, "ax2.set_ylabel('Current (A)')")?;
        writeln!(writer, "ax2.legend()")?;
        writeln!(writer, "ax2.grid(True)")?;
        writeln!(writer)?;
        
        // 保存和显示
        writeln!(writer, "# Save and show plot")?);
        writeln!(writer, "plt.tight_layout()")?;
        writeln!(writer, "plt.savefig('simulation_results.png', dpi=300, bbox_inches='tight')")?;
        writeln!(writer, "plt.show()")?;
        
        writer.flush()?;
        Ok(())
    }
}
```

## 统计分析算法

### 信号统计分析

```rust
impl OutputProcessor {
    pub fn calculate_statistics(&self, results: &SimulationResult) -> Result<ResultStatistics> {
        let mut node_voltage_stats = HashMap::new();
        let mut current_stats = HashMap::new();
        
        // 计算节点电压统计
        for (node_name, voltages) in &results.node_voltages {
            let stats = self.calculate_signal_stats(voltages);
            node_voltage_stats.insert(node_name.clone(), stats);
        }
        
        // 计算电流统计
        for (current_name, currents) in &results.currents {
            let stats = self.calculate_signal_stats(currents);
            current_stats.insert(current_name.clone(), stats);
        }
        
        // 计算分析元数据
        let analysis_metadata = AnalysisMetadata {
            analysis_type: format!("{:?}", results.analysis_type),
            total_points: results.time_points.len(),
            time_span: results.time_points.last().unwrap_or(&0.0) - results.time_points.first().unwrap_or(&0.0),
            convergence_rate: self.calculate_convergence_rate(&results.convergence_info),
            simulation_time: results.total_time,
        };
        
        Ok(ResultStatistics {
            node_voltage_stats,
            current_stats,
            analysis_metadata,
        })
    }

    fn calculate_signal_stats(&self, values: &[f64]) -> SignalStats {
        if values.is_empty() {
            return SignalStats {
                min: 0.0, max: 0.0, mean: 0.0, std_dev: 0.0, rms: 0.0, peak_to_peak: 0.0,
            };
        }
        
        let n = values.len() as f64;
        let sum: f64 = values.iter().sum();
        let mean = sum / n;
        
        let variance = values.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / n;
        let std_dev = variance.sqrt();
        
        let rms = (values.iter().map(|&x| x.powi(2)).sum::<f64>() / n).sqrt();
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let peak_to_peak = max - min;
        
        SignalStats {
            min,
            max,
            mean,
            std_dev,
            rms,
            peak_to_peak,
        }
    }

    fn calculate_convergence_rate(&self, convergence_info: &[ConvergenceInfo]) -> f64 {
        if convergence_info.len() < 2 {
            return 1.0;
        }
        
        let successful_iterations = convergence_info.iter()
            .filter(|info| info.residual_norm < 1e-6)
            .count();
        
        successful_iterations as f64 / convergence_info.len() as f64
    }
}
```

### 详细摘要输出

```rust
impl OutputProcessor {
    pub fn print_detailed_summary(&self, results: &SimulationResult) -> Result<()> {
        println!("=== Simulation Summary ===");
        println!("Analysis type: {:?}", results.analysis_type);
        println!("Total simulation time: {:.3}ms", results.total_time * 1000.0);
        println!("Success: {}", results.success);
        println!("Number of time points: {}", results.time_points.len());
        
        // 节点电压摘要
        println!("\nNode voltages (final values):");
        for (node_name, voltages) in &results.node_voltages {
            if let Some(&final_voltage) = voltages.last() {
                println!("  V({}): {:.6}V", node_name, final_voltage);
            }
        }
        
        // 电流摘要
        println!("\nSource currents (final values):");
        for (current_name, currents) in &results.currents {
            if let Some(&final_current) = currents.last() {
                println!("  I({}): {:.6}A", current_name, final_current);
            }
        }
        
        // 收敛信息
        if let Some(last_conv) = results.convergence_info.last() {
            println!("\nConvergence info:");
            println!("  Total iterations: {}", results.convergence_info.len());
            println!("  Average residual norm: {:.2e}", 
                    results.convergence_info.iter()
                        .map(|c| c.residual_norm)
                        .sum::<f64>() / results.convergence_info.len() as f64);
        }
        
        Ok(())
    }
}
```

## 数值格式化

### 格式化算法

```rust
impl OutputProcessor {
    fn format_number(&self, value: f64) -> String {
        if self.config.scientific_notation {
            format!("{:.prec$e}", value, prec = self.config.precision)
        } else {
            format!("{:.prec$}", value, prec = self.config.precision)
        }
    }
}
```

## 性能优化

### 1. 内存优化
- 流式写入大文件
- 避免不必要的字符串分配
- 重用缓冲区

### 2. I/O优化
- 使用缓冲写入
- 批量写入操作
- 异步I/O（未来扩展）

### 3. 格式化优化
- 预分配字符串容量
- 使用高效的格式化算法
- 缓存常用格式

## 扩展性设计

### 1. 新输出格式
通过扩展导出函数，可以轻松添加新的输出格式。

### 2. 自定义统计
可以添加更多统计指标，如频谱分析、功率计算等。

### 3. 实时输出
支持实时数据流输出，便于长时间仿真监控。

## 总结

RustSim的输出处理模块提供了完整的结果导出和统计分析功能。通过支持多种格式的输出，满足了不同用户的需求。高效的算法实现和灵活的配置选项，为电路仿真结果的分析和可视化提供了强大的工具。 