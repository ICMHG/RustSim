# 解析器算法

## 概述

RustSim的解析器模块负责将SPICE网表文件转换为内部数据结构。该模块采用了混合解析策略，结合正则表达式和nom组合器库，实现了对复杂SPICE语法的健壮解析。

## SPICE语法解析原理

### SPICE网表格式

SPICE网表文件包含以下主要元素：
- 标题行：电路描述
- 元件定义：电阻、电容、电感、电压源、电流源等
- 分析指令：.op、.tran、.dc等
- 参数定义：.param等
- 结束标记：.end

### 解析策略

RustSim采用分层解析策略：

1. **预处理层**：清理注释、空行，标准化格式
2. **分类层**：识别行类型（元件、指令、参数等）
3. **解析层**：根据类型进行具体解析
4. **验证层**：检查解析结果的正确性

## 混合解析器实现

### 正则表达式解析器

对于元件行，RustSim使用正则表达式进行快速解析：

```rust
lazy_static! {
    static ref COMPONENT_REGEX: Regex = Regex::new(
        r"^([VRCLID][A-Za-z0-9]*)\s+([A-Za-z0-9]+)\s+([A-Za-z0-9]+)\s+(.+)$"
    ).unwrap();
}
```

**优势**：
- 解析速度快
- 错误处理简单
- 支持复杂模式匹配

**实现细节**：
```rust
fn parse_component_regex(line: &str) -> Result<Component, String> {
    if let Some(captures) = COMPONENT_REGEX.captures(line) {
        let name = captures[1].to_string();
        let node1 = captures[2].to_string();
        let node2 = captures[3].to_string();
        let params = captures[4].trim();
        
        // 解析元件类型和参数
        let component_type = match name.chars().next().unwrap().to_ascii_uppercase() {
            'R' => ComponentType::Resistor,
            'C' => ComponentType::Capacitor,
            'L' => ComponentType::Inductor,
            'V' => ComponentType::VoltageSource,
            'I' => ComponentType::CurrentSource,
            'D' => ComponentType::Diode,
            _ => return Err(format!("Unknown component type: {}", name)),
        };
        
        // 解析数值参数
        let value = parse_value_with_unit(params)?;
        
        Ok(Component {
            name,
            component_type,
            nodes: vec![node1, node2],
            value,
            model: None,
        })
    } else {
        Err(format!("Failed to parse component line: {}", line))
    }
}
```

### Nom组合器解析器

对于复杂的语法结构，使用nom组合器库：

```rust
fn parse_netlist_line(input: &str) -> IResult<&str, NetlistLine> {
    let (input, _) = multispace0(input)?;
    
    let (input, result) = terminated(
        alt((
            map(parse_component, NetlistLine::Component),
            map(parse_analysis, NetlistLine::Analysis),
            map(parse_parameter, |(name, value)| NetlistLine::Parameter(name, value)),
            map(tag_no_case(".end"), |_| NetlistLine::End),
        )),
        multispace0,
    )(input)?;
    
    Ok((input, result))
}
```

## 单位转换算法

### 支持的单位

RustSim支持多种工程单位：

- **电阻**：Ω, k (kΩ), meg (MΩ), m (mΩ), u (μΩ)
- **电容**：F, m (mF), u (μF), n (nF), p (pF)
- **电感**：H, m (mH), u (μH), n (nH)
- **时间**：s, ms, us, ns, ps, fs
- **电压/电流**：V, mV, uV / A, mA, uA, nA

### 转换算法实现

```rust
fn parse_value_with_unit(value_str: &str) -> Result<f64> {
    let value_str = value_str.trim().to_lowercase();
    
    if let Some(num_str) = value_str.strip_suffix("meg") {
        Ok(num_str.parse::<f64>()? * 1e6)
    } else if let Some(num_str) = value_str.strip_suffix("k") {
        Ok(num_str.parse::<f64>()? * 1e3)
    } else if let Some(num_str) = value_str.strip_suffix("m") {
        Ok(num_str.parse::<f64>()? * 1e-3)
    } else if let Some(num_str) = value_str.strip_suffix("u") {
        Ok(num_str.parse::<f64>()? * 1e-6)
    } else if let Some(num_str) = value_str.strip_suffix("n") {
        Ok(num_str.parse::<f64>()? * 1e-9)
    } else if let Some(num_str) = value_str.strip_suffix("p") {
        Ok(num_str.parse::<f64>()? * 1e-12)
    } else if let Some(num_str) = value_str.strip_suffix("f") {
        Ok(num_str.parse::<f64>()? * 1e-15)
    } else if let Some(num_str) = value_str.strip_suffix("v") {
        Ok(num_str.parse::<f64>()?)
    } else {
        Ok(value_str.parse::<f64>()?)
    }
}
```

## 错误处理和恢复机制

### 错误类型分类

1. **语法错误**：格式不正确、缺少参数
2. **语义错误**：参数值超出范围、元件类型不匹配
3. **文件错误**：文件不存在、权限问题
4. **系统错误**：内存不足、解析器内部错误

### 错误恢复策略

```rust
pub fn parse_netlist(&self, content: &str) -> Result<SpiceNetlist> {
    let lines = self.preprocess_lines(content);
    let mut components = Vec::new();
    let mut analyses = Vec::new();
    let mut parameters = HashMap::new();
    
    for (line_num, line) in lines.iter().enumerate() {
        if line.trim().is_empty() || line.trim().starts_with('*') {
            continue; // 跳过空行和注释
        }
        
        // 首先尝试正则表达式解析
        if let Ok(component) = parse_component_regex(line) {
            components.push(component);
            continue;
        }
        
        // 正则解析失败，尝试nom解析
        match parse_netlist_line(line) {
            Ok((_, NetlistLine::Component(component))) => {
                components.push(component);
            }
            Ok((_, NetlistLine::Analysis(analysis))) => {
                analyses.push(analysis);
            }
            Ok((_, NetlistLine::Parameter(name, value))) => {
                parameters.insert(name, value);
            }
            Ok((_, NetlistLine::End)) => {
                break; // 停止解析
            }
            Err(e) => {
                // 记录错误但继续解析
                println!("DEBUG: Failed to parse line {}: '{}', error: {:?}", 
                         line_num + 1, line, e);
            }
        }
    }
    
    // 构建结果
    let mut node_names = std::collections::HashSet::new();
    for comp in &components {
        for node in &comp.nodes {
            node_names.insert(node.clone());
        }
    }
    
    let nodes: Vec<Node> = node_names.into_iter()
        .map(|name| Node::new(name))
        .collect();
    
    Ok(SpiceNetlist {
        title: "".to_string(),
        components,
        nodes,
        subcircuits: Vec::new(),
        parameters,
        analyses,
    })
}
```

## 性能优化

### 1. 缓存机制
- 编译时正则表达式
- 解析结果缓存
- 单位转换表缓存

### 2. 并行处理
- 多行并行解析（未来扩展）
- 异步文件读取

### 3. 内存优化
- 字符串复用
- 零拷贝解析
- 智能内存分配

## 扩展性设计

### 1. 新元件类型支持
通过扩展正则表达式模式和解析函数，可以轻松添加新的元件类型。

### 2. 新分析类型支持
通过扩展nom组合器，可以支持新的分析指令。

### 3. 新单位支持
通过扩展单位转换表，可以支持新的工程单位。

## 测试和验证

### 1. 单元测试
每个解析函数都有对应的单元测试，确保正确性。

### 2. 集成测试
完整的网表文件解析测试，验证端到端功能。

### 3. 性能测试
解析速度和内存使用测试，确保性能要求。

## 总结

RustSim的解析器模块通过混合解析策略，实现了对SPICE语法的健壮解析。正则表达式提供了快速的元件解析，而nom组合器处理复杂的语法结构。完善的错误处理和恢复机制确保了系统的稳定性，而性能优化和扩展性设计为未来的功能扩展提供了基础。 