# RustSim - 高性能SPICE电路仿真器

[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-green.svg)](LICENSE)
[![SPICE](https://img.shields.io/badge/SPICE-Compatible-red.svg)](https://en.wikipedia.org/wiki/SPICE)
[![Linear Algebra](https://img.shields.io/badge/Linear-Algebra-9cf.svg)](https://nalgebra.org/)
[![Documentation](https://img.shields.io/badge/Documentation-Online-blue.svg)](https://icmhg.github.io/RustSim/)

## 📝 概述

RustSim是一个用Rust编写的现代、高性能SPICE电路仿真器。它提供准确高效的电子电路仿真，专注于类型安全、内存效率和计算性能。

## ✨ 特性

- **全面的SPICE支持**：解析标准SPICE网表格式（.sp文件）
- **多种分析类型**：
  - 工作点（DC）分析
  - DC扫描分析  
  - 瞬态分析
- **组件支持**：
  - 电阻（R）
  - 电容（C）
  - 电感（L）
  - 电压源（V）
  - 电流源（I）
  - 二极管（D）
- **高级求解引擎**：
  - 多种线性系统求解器（LU、QR、BiCGSTAB、CG）
  - 基于矩阵特性的自动求解器选择
  - 稀疏矩阵优化
- **多种输出格式**：
  - CSV用于数据分析
  - JSON用于结构化数据
  - MATLAB/Octave脚本
  - Python/NumPy脚本（带绘图功能）
- **现代Rust实现**：
  - 内存安全和线程安全
  - 零成本抽象
  - 全面的错误处理

## 🚀 快速开始

### 安装

确保已安装Rust，然后克隆并构建项目：

```bash
git clone https://github.com/ICMHG/RustSim.git
cd RustSim
cargo build --release
```

### 基本用法

```bash
# 运行工作点分析
./target/release/rustSim examples/simple_circuit.sp

# 运行瞬态分析
./target/release/rustSim examples/rc_circuit.sp --tran 1ns 1us --output results.csv

# 运行DC扫描分析  
./target/release/rustSim examples/voltage_divider.sp --dc V1 0V 5V 0.1V --format json

# 显示帮助
./target/release/rustSim --help
```

## 📋 命令行界面

```
rustSim [选项] <输入文件>

参数：
  <输入文件>  输入SPICE网表文件（.sp）

选项：
  -o, --output <文件>        仿真结果输出文件
      --tran <时间步> <停止时间>  瞬态分析：时间步长和停止时间
      --dc <源> <开始> <停止> <步长>  DC扫描分析
  -f, --format <格式>        输出格式 [默认: csv] [可选值: csv, json]
  -v, --verbose              增加详细级别
  -h, --help                 打印帮助
  -V, --version              打印版本
```

## 📖 SPICE网表格式

RustSim支持标准SPICE网表语法：

```spice
简单RC电路
* 这是注释
V1 1 0 DC 5V
R1 1 2 1k
C1 2 0 1u
.tran 10n 1u
.end
```

### 支持的组件

| 组件 | 语法 | 示例 |
|------|------|------|
| 电阻 | `R<名称> <节点1> <节点2> <值>` | `R1 1 2 1k` |
| 电容 | `C<名称> <节点1> <节点2> <值>` | `C1 2 0 10u` |
| 电感 | `L<名称> <节点1> <节点2> <值>` | `L1 2 3 1m` |
| 电压源 | `V<名称> <正节点> <负节点> DC <值>` | `V1 1 0 DC 5V` |
| 电流源 | `I<名称> <正节点> <负节点> DC <值>` | `I1 1 0 DC 1mA` |

### 支持的单位

- **电阻**：Ω, kΩ, MΩ, mΩ, uΩ
- **电容**：F, mF, uF, nF, pF
- **电感**：H, mH, uH, nH, pH
- **电压**：V, mV, uV, kV
- **电流**：A, mA, uA, nA, pA
- **时间**：s, ms, us, ns, ps, fs

## 🔧 技术架构

### 核心组件

1. **解析器**：基于正则表达式的SPICE网表解析器
2. **电路模型**：节点和组件的抽象表示
3. **MNA系统**：修正节点分析矩阵构建
4. **求解器**：多种线性系统求解算法
5. **仿真引擎**：协调分析流程
6. **输出处理器**：结果格式化和导出

### 求解器算法

- **LU分解**：适用于大多数线性系统
- **QR分解**：数值稳定性更好
- **BiCGSTAB**：适用于大型稀疏系统
- **共轭梯度**：适用于对称正定系统

## 📊 性能特性

- **内存效率**：稀疏矩阵存储
- **计算速度**：优化的线性代数运算
- **数值稳定性**：条件数检查和自动求解器选择
- **并行处理**：支持多线程计算

## 🧪 测试和验证

项目包含全面的测试套件：

```bash
# 运行所有测试
cargo test

# 运行基准测试
cargo bench

# 运行特定测试
cargo test test_voltage_divider
```

## 📈 基准测试结果

在标准硬件上，RustSim的性能表现：

- **简单电路**：< 1ms仿真时间
- **中等电路**：1-10ms仿真时间
- **复杂电路**：10-100ms仿真时间
- **内存使用**：高效稀疏存储

## 🤝 贡献

欢迎贡献！请查看[贡献指南](CONTRIBUTING.md)了解详情。

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/ICMHG/RustSim
cd rustsim

# 安装依赖
cargo build

# 运行测试
cargo test

# 检查代码质量
cargo clippy
cargo fmt
```

## 📄 许可证

本项目采用MIT许可证 - 查看[LICENSE](LICENSE)文件了解详情。

## 🙏 致谢

感谢以下开源项目的支持：

- [nalgebra](https://nalgebra.org/) - 线性代数库
- [sprs](https://github.com/vbarrielle/sprs) - 稀疏矩阵库
- [nom](https://github.com/Geal/nom) - 解析器组合子库
- [clap](https://github.com/clap-rs/clap) - 命令行参数解析

## 📞 联系方式

- 项目主页：https://github.com/ICMHG/RustSim
- 问题报告：https://github.com/ICMHG/RustSim/issues
- 讨论区：https://github.com/ICMHG/RustSim/discussions

---

**RustSim** - 用Rust构建的高性能电路仿真器 ⚡ 