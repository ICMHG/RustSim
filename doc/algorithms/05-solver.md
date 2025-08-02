# 求解器算法

## 概述

RustSim的求解器模块实现了多种线性系统求解算法，用于解决MNA生成的线性方程组。该模块支持直接法和迭代法，并具有自动求解器选择功能，能够根据矩阵特性选择最优的求解策略。

## 线性系统求解原理

### 问题定义

求解器需要解决形如 `Ax = b` 的线性方程组，其中：
- **A**：n×n 系数矩阵（通常稀疏）
- **b**：n×1 右侧向量
- **x**：n×1 未知变量向量

### 求解方法分类

1. **直接法**：通过矩阵分解直接求解
   - LU分解
   - QR分解
2. **迭代法**：通过迭代逼近求解
   - 共轭梯度法（CG）
   - BiCGSTAB方法

## 数据结构设计

### 求解器配置

```rust
pub struct SolverConfig {
    pub method: SolverMethod,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub use_pivoting: bool,
    pub check_condition_number: bool,
}

pub enum SolverMethod {
    Lu,        // LU分解
    Qr,        // QR分解
    Cg,        // 共轭梯度法
    BiCgStab,  // BiCGSTAB方法
}

pub struct SolverStats {
    pub method_used: SolverMethod,
    pub iterations: usize,
    pub residual_norm: f64,
    pub solve_time: f64,
    pub success: bool,
    pub condition_number: Option<f64>,
}
```

### 求解器实现

```rust
pub struct LinearSolver {
    config: SolverConfig,
}

impl LinearSolver {
    pub fn new() -> Self {
        LinearSolver {
            config: SolverConfig::default(),
        }
    }

    pub fn with_config(config: SolverConfig) -> Self {
        LinearSolver { config }
    }
}
```

## 直接法求解器

### LU分解求解器

LU分解将矩阵A分解为 `A = LU`，其中L是下三角矩阵，U是上三角矩阵。

```rust
impl LinearSolver {
    fn solve_lu_dense(&self, matrix: &DMatrix<f64>, rhs: &DVector<f64>) -> Result<(DVector<f64>, SolverStats)> {
        let start_time = Instant::now();
        
        // 执行LU分解
        let lu = matrix.lu();
        
        // 求解 Ly = b
        let y = lu.solve_lower_triangular(rhs)?;
        
        // 求解 Ux = y
        let solution = lu.solve_upper_triangular(&y)?;
        
        // 计算残差
        let residual = matrix * &solution - rhs;
        let residual_norm = residual.norm();
        
        let solve_time = start_time.elapsed().as_secs_f64();
        
        let stats = SolverStats {
            method_used: SolverMethod::Lu,
            iterations: 1,
            residual_norm,
            solve_time,
            success: residual_norm < self.config.tolerance,
            condition_number: None,
        };
        
        Ok((solution, stats))
    }

    fn solve_lu_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let start_time = Instant::now();
        
        // 转换为密集矩阵进行LU分解
        let dense_matrix = sparse_to_dense(matrix);
        let dense_rhs = DVector::from_column_slice(rhs);
        
        let (solution, stats) = self.solve_lu_dense(&dense_matrix, &dense_rhs)?;
        
        Ok((solution.as_slice().to_vec(), stats))
    }
}
```

### QR分解求解器

QR分解将矩阵A分解为 `A = QR`，其中Q是正交矩阵，R是上三角矩阵。

```rust
impl LinearSolver {
    fn solve_qr_dense(&self, matrix: &DMatrix<f64>, rhs: &DVector<f64>) -> Result<(DVector<f64>, SolverStats)> {
        let start_time = Instant::now();
        
        // 执行QR分解
        let qr = matrix.qr();
        
        // 求解 Rx = Q^T b
        let qt_b = qr.q().transpose() * rhs;
        let solution = qr.solve(&qt_b)?;
        
        // 计算残差
        let residual = matrix * &solution - rhs;
        let residual_norm = residual.norm();
        
        let solve_time = start_time.elapsed().as_secs_f64();
        
        let stats = SolverStats {
            method_used: SolverMethod::Qr,
            iterations: 1,
            residual_norm,
            solve_time,
            success: residual_norm < self.config.tolerance,
            condition_number: None,
        };
        
        Ok((solution, stats))
    }
}
```

## 迭代法求解器

### 共轭梯度法（CG）

共轭梯度法适用于对称正定矩阵，具有最优的收敛性质。

```rust
impl LinearSolver {
    fn solve_cg_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let start_time = Instant::now();
        let n = matrix.rows();
        
        // 初始化解向量
        let mut x = vec![0.0; n];
        let mut r = rhs.to_vec(); // 初始残差 r = b - Ax
        let mut p = r.clone(); // 搜索方向
        
        let mut r_dot_r = vector_dot(&r, &r);
        let initial_residual_norm = r_dot_r.sqrt();
        
        let mut iteration = 0;
        let mut residual_norm = initial_residual_norm;
        
        while iteration < self.config.max_iterations && residual_norm > self.config.tolerance {
            // 计算 Ap
            let ap = sparse_matrix_vector_multiply(matrix, &p);
            
            // 计算步长 α = (r·r) / (p·Ap)
            let p_dot_ap = vector_dot(&p, &ap);
            let alpha = r_dot_r / p_dot_ap;
            
            // 更新解向量 x = x + αp
            for i in 0..n {
                x[i] += alpha * p[i];
            }
            
            // 更新残差 r = r - αAp
            for i in 0..n {
                r[i] -= alpha * ap[i];
            }
            
            // 计算新的残差内积
            let r_dot_r_new = vector_dot(&r, &r);
            residual_norm = r_dot_r_new.sqrt();
            
            // 计算搜索方向更新系数 β = (r_new·r_new) / (r_old·r_old)
            let beta = r_dot_r_new / r_dot_r;
            
            // 更新搜索方向 p = r + βp
            for i in 0..n {
                p[i] = r[i] + beta * p[i];
            }
            
            r_dot_r = r_dot_r_new;
            iteration += 1;
        }
        
        let solve_time = start_time.elapsed().as_secs_f64();
        
        let stats = SolverStats {
            method_used: SolverMethod::Cg,
            iterations: iteration,
            residual_norm,
            solve_time,
            success: residual_norm < self.config.tolerance,
            condition_number: None,
        };
        
        Ok((x, stats))
    }
}
```

### BiCGSTAB方法

BiCGSTAB方法适用于非对称矩阵，是BiCG方法的改进版本。

```rust
impl LinearSolver {
    fn solve_bicgstab_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let start_time = Instant::now();
        let n = matrix.rows();
        
        // 初始化解向量
        let mut x = vec![0.0; n];
        let mut r = rhs.to_vec(); // 初始残差
        let mut r_hat = r.clone(); // 伪残差
        
        let mut rho = 1.0;
        let mut alpha = 1.0;
        let mut omega = 1.0;
        
        let mut p = vec![0.0; n];
        let mut v = vec![0.0; n];
        
        let initial_residual_norm = vector_norm(&r);
        let mut residual_norm = initial_residual_norm;
        let mut iteration = 0;
        
        while iteration < self.config.max_iterations && residual_norm > self.config.tolerance {
            let rho_old = rho;
            rho = vector_dot(&r_hat, &r);
            
            let beta = (rho / rho_old) * (alpha / omega);
            
            // 更新 p
            for i in 0..n {
                p[i] = r[i] + beta * (p[i] - omega * v[i]);
            }
            
            // 计算 v = Ap
            v = sparse_matrix_vector_multiply(matrix, &p);
            
            alpha = rho / vector_dot(&r_hat, &v);
            
            // 计算 s = r - αv
            let mut s = vec![0.0; n];
            for i in 0..n {
                s[i] = r[i] - alpha * v[i];
            }
            
            // 计算 t = As
            let t = sparse_matrix_vector_multiply(matrix, &s);
            
            omega = vector_dot(&t, &s) / vector_dot(&t, &t);
            
            // 更新解向量和残差
            for i in 0..n {
                x[i] += alpha * p[i] + omega * s[i];
                r[i] = s[i] - omega * t[i];
            }
            
            residual_norm = vector_norm(&r);
            iteration += 1;
        }
        
        let solve_time = start_time.elapsed().as_secs_f64();
        
        let stats = SolverStats {
            method_used: SolverMethod::BiCgStab,
            iterations: iteration,
            residual_norm,
            solve_time,
            success: residual_norm < self.config.tolerance,
            condition_number: None,
        };
        
        Ok((x, stats))
    }
}
```

## 自动求解器选择

### 矩阵特性分析

```rust
pub fn is_symmetric(matrix: &CsMat<f64>, tolerance: f64) -> bool {
    for (i, j, &value) in matrix.iter() {
        if let Some(&other_value) = matrix.get(j, i) {
            if (value - other_value).abs() > tolerance {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

pub fn auto_select_solver(matrix: &CsMat<f64>) -> SolverMethod {
    // 检查矩阵是否对称
    if is_symmetric(matrix, 1e-12) {
        // 对于对称矩阵，优先使用CG方法
        SolverMethod::Cg
    } else {
        // 对于非对称矩阵，使用BiCGSTAB方法
        SolverMethod::BiCgStab
    }
}
```

### 求解器选择逻辑

```rust
impl LinearSolver {
    pub fn solve_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let method = if self.config.auto_select_solver {
            auto_select_solver(matrix)
        } else {
            self.config.method.clone()
        };
        
        match method {
            SolverMethod::Lu => self.solve_lu_sparse(matrix, rhs),
            SolverMethod::Qr => {
                // QR分解需要转换为密集矩阵
                let dense_matrix = sparse_to_dense(matrix);
                let dense_rhs = DVector::from_column_slice(rhs);
                let (solution, stats) = self.solve_qr_dense(&dense_matrix, &dense_rhs)?;
                Ok((solution.as_slice().to_vec(), stats))
            }
            SolverMethod::Cg => self.solve_cg_sparse(matrix, rhs),
            SolverMethod::BiCgStab => self.solve_bicgstab_sparse(matrix, rhs),
        }
    }
}
```

## 辅助函数

### 稀疏矩阵操作

```rust
fn sparse_to_dense(sparse: &CsMat<f64>) -> DMatrix<f64> {
    let (rows, cols) = sparse.shape();
    let mut dense = DMatrix::zeros(rows, cols);
    
    for (i, j, &value) in sparse.iter() {
        dense[(i, j)] = value;
    }
    
    dense
}

fn sparse_matrix_vector_multiply(matrix: &CsMat<f64>, vector: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; matrix.rows()];
    
    for (i, j, &value) in matrix.iter() {
        result[i] += value * vector[j];
    }
    
    result
}

fn vector_dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

fn vector_norm(vector: &[f64]) -> f64 {
    vector_dot(vector, vector).sqrt()
}
```

## 性能优化

### 1. 稀疏矩阵优化
- 使用压缩稀疏行（CSR）格式
- 避免不必要的矩阵转换
- 优化稀疏矩阵-向量乘法

### 2. 数值稳定性
- 条件数检查
- 主元选择策略
- 残差监控

### 3. 内存管理
- 重用向量空间
- 避免频繁分配
- 缓存友好访问模式

## 收敛性分析

### 收敛判据

1. **残差范数**：`||r|| < tolerance`
2. **相对残差**：`||r|| / ||b|| < tolerance`
3. **最大迭代次数**：防止无限循环

### 收敛加速

1. **预处理器**：改善矩阵条件数
2. **重启策略**：防止数值误差累积
3. **自适应容差**：根据问题规模调整

## 扩展性设计

### 1. 新求解方法
通过扩展SolverMethod枚举和相应的求解函数，可以轻松添加新的求解方法。

### 2. 预处理器支持
可以添加ILU、IC等预处理器来改善收敛性。

### 3. 并行求解
支持多线程和GPU加速的并行求解器。

## 总结

RustSim的求解器模块提供了完整的线性系统求解功能，支持直接法和迭代法。通过自动求解器选择，能够根据矩阵特性选择最优的求解策略。高效的稀疏矩阵操作和数值稳定性保证，为电路仿真提供了可靠的数值计算基础。 