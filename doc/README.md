# RustSim 算法实现原理文档

欢迎阅读RustSim高性能SPICE电路仿真器的算法实现原理文档！

## 📖 文档概述

本文档详细介绍了RustSim项目的核心算法实现原理，包括：

- **总体架构**：系统设计和模块关系
- **解析器算法**：SPICE网表解析原理
- **电路建模**：电路抽象和元件建模
- **MNA算法**：改进节点分析实现
- **求解器算法**：线性系统求解方法
- **仿真算法**：仿真控制和收敛分析
- **输出处理**：结果导出和统计分析

## 🚀 快速开始

### 在线阅读

文档已部署到GitHub Pages，您可以在线阅读：
https://icmhg.github.io/RustSim/

### 本地构建

如果您想在本地构建和查看文档：

```bash
# 进入文档目录
cd doc

# 安装GitBook CLI（如果未安装）
npm install -g gitbook-cli

# 安装GitBook
gitbook fetch latest

# 安装插件
gitbook install

# 构建文档
gitbook build

# 启动本地服务器
gitbook serve
```

然后在浏览器中访问 `http://localhost:4000` 查看文档。

### 部署到GitHub Pages

```bash
# 进入文档目录
cd doc

# 运行部署脚本
./deploy.sh deploy
```

## 📚 文档结构

```
doc/
├── README.md                 # 本文档
├── SUMMARY.md               # GitBook目录
├── book.json                # GitBook配置
├── deploy.sh                # 部署脚本
└── algorithms/              # 算法文档
    ├── README.md            # 算法文档索引
    ├── 01-architecture.md   # 总体架构
    ├── 02-parser.md         # 解析器算法
    ├── 03-circuit-modeling.md # 电路建模
    ├── 04-mna.md            # MNA算法
    ├── 05-solver.md         # 求解器算法
    ├── 06-simulation.md     # 仿真算法
    └── 07-output.md         # 输出处理
```

## 🎯 阅读建议

### 初学者

如果您是第一次接触电路仿真或RustSim项目，建议按以下顺序阅读：

1. **总体架构** - 了解系统整体设计
2. **解析器算法** - 理解输入处理
3. **电路建模** - 学习数据结构
4. **MNA算法** - 掌握核心仿真原理
5. **求解器算法** - 了解数值计算
6. **仿真算法** - 理解仿真流程
7. **输出处理** - 学习结果处理

### 开发者

如果您是开发者或研究人员，可以：

- 直接跳转到感兴趣的算法章节
- 查看代码示例和实现细节
- 参考性能优化和扩展性设计部分

### 研究人员

如果您是研究人员或学生，可以：

- 重点关注算法原理和数学推导
- 查看性能分析和复杂度分析
- 参考扩展性设计部分了解未来发展方向

## 🔧 技术特性

### 支持的格式

- **Markdown** - 主要文档格式
- **Mermaid图表** - 系统架构图
- **代码高亮** - Rust代码示例
- **数学公式** - LaTeX数学表达式

### 功能特性

- **全文搜索** - 快速查找内容
- **目录导航** - 清晰的章节结构
- **响应式设计** - 支持移动端
- **代码复制** - 一键复制代码示例

## 🤝 贡献指南

如果您发现文档中的错误或有改进建议，欢迎贡献：

1. **提交Issue** - 报告问题或建议
2. **提交Pull Request** - 修复错误或改进内容
3. **参与讨论** - 在GitHub Discussions中交流

### 文档编写规范

- 使用清晰的中文描述
- 包含详细的代码示例
- 添加必要的图表和公式
- 保持文档结构的一致性

## 📞 支持

如果您在使用文档时遇到问题：

- 📧 **邮箱**：通过GitHub Issues联系
- 💬 **讨论**：[GitHub Discussions](https://github.com/ICMHG/RustSim/discussions)
- 🐛 **问题**：[GitHub Issues](https://github.com/ICMHG/RustSim/issues)

## 📄 许可证

本文档采用与RustSim项目相同的许可证。

---

**祝您阅读愉快！** 🎉

如果您觉得这个文档对您有帮助，请给项目点个⭐️！ 