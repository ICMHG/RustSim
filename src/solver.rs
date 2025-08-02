use nalgebra::{DMatrix, DVector};
use sprs::CsMat;
use anyhow::{anyhow, Result};
use std::time::Instant;

/// Solver configuration
#[derive(Debug, Clone)]
pub struct SolverConfig {
    pub method: SolverMethod,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub use_pivoting: bool,
    pub check_condition_number: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        SolverConfig {
            method: SolverMethod::Lu,
            tolerance: 1e-12,
            max_iterations: 1000,
            use_pivoting: true,
            check_condition_number: false,
        }
    }
}

/// Available solver methods
#[derive(Debug, Clone, PartialEq)]
pub enum SolverMethod {
    /// Direct LU decomposition
    Lu,
    /// QR decomposition  
    Qr,
    /// Conjugate Gradient (for symmetric positive definite matrices)
    Cg,
    /// BiCGSTAB (for general sparse matrices)
    BiCgStab,
}

/// Solver statistics
#[derive(Debug, Clone)]
pub struct SolverStats {
    pub method_used: SolverMethod,
    pub iterations: usize,
    pub residual_norm: f64,
    pub solve_time: f64,
    pub success: bool,
    pub condition_number: Option<f64>,
}

/// Linear system solver
pub struct LinearSolver {
    config: SolverConfig,
}

impl LinearSolver {
    /// Create a new solver with default configuration
    pub fn new() -> Self {
        LinearSolver {
            config: SolverConfig::default(),
        }
    }

    /// Create a new solver with custom configuration
    pub fn with_config(config: SolverConfig) -> Self {
        LinearSolver { config }
    }

    /// Solve the linear system Ax = b using dense matrices
    pub fn solve_dense(&self, matrix: &DMatrix<f64>, rhs: &DVector<f64>) -> Result<(DVector<f64>, SolverStats)> {
        let start_time = Instant::now();
        
        if matrix.nrows() != matrix.ncols() {
            return Err(anyhow!("Matrix must be square"));
        }
        
        if matrix.nrows() != rhs.len() {
            return Err(anyhow!("Matrix and RHS dimensions don't match"));
        }

        let (solution, stats) = match self.config.method {
            SolverMethod::Lu => self.solve_lu_dense(matrix, rhs)?,
            SolverMethod::Qr => self.solve_qr_dense(matrix, rhs)?,
            _ => {
                // Fall back to LU for unsupported methods with dense matrices
                self.solve_lu_dense(matrix, rhs)?
            }
        };

        let solve_time = start_time.elapsed().as_secs_f64();
        let final_stats = SolverStats {
            solve_time,
            ..stats
        };

        Ok((solution, final_stats))
    }

    /// Solve the linear system Ax = b using sparse matrices
    pub fn solve_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let start_time = Instant::now();

        if matrix.rows() != matrix.cols() {
            return Err(anyhow!("Matrix must be square"));
        }

        if matrix.rows() != rhs.len() {
            return Err(anyhow!("Matrix and RHS dimensions don't match"));
        }

        let (solution, stats) = match self.config.method {
            SolverMethod::Lu => self.solve_lu_sparse(matrix, rhs)?,
            SolverMethod::BiCgStab => self.solve_bicgstab_sparse(matrix, rhs)?,
            SolverMethod::Cg => self.solve_cg_sparse(matrix, rhs)?,
            _ => {
                // Fall back to direct solve
                self.solve_lu_sparse(matrix, rhs)?
            }
        };

        let solve_time = start_time.elapsed().as_secs_f64();
        let final_stats = SolverStats {
            solve_time,
            ..stats
        };

        Ok((solution, final_stats))
    }

    /// LU decomposition solve for dense matrices
    fn solve_lu_dense(&self, matrix: &DMatrix<f64>, rhs: &DVector<f64>) -> Result<(DVector<f64>, SolverStats)> {
        let lu = matrix.clone().lu();
        
        match lu.solve(rhs) {
            Some(solution) => {
                let residual = matrix * &solution - rhs;
                let residual_norm = residual.norm();
                
                Ok((solution, SolverStats {
                    method_used: SolverMethod::Lu,
                    iterations: 1,
                    residual_norm,
                    solve_time: 0.0, // Will be set by caller
                    success: residual_norm < self.config.tolerance * 1000.0, // More lenient for direct methods
                    condition_number: None,
                }))
            }
            None => Err(anyhow!("LU decomposition failed - matrix may be singular")),
        }
    }

    /// QR decomposition solve for dense matrices
    fn solve_qr_dense(&self, matrix: &DMatrix<f64>, rhs: &DVector<f64>) -> Result<(DVector<f64>, SolverStats)> {
        let qr = matrix.clone().qr();
        
        match qr.solve(rhs) {
            Some(solution) => {
                let residual = matrix * &solution - rhs;
                let residual_norm = residual.norm();
                
                Ok((solution, SolverStats {
                    method_used: SolverMethod::Qr,
                    iterations: 1,
                    residual_norm,
                    solve_time: 0.0,
                    success: residual_norm < self.config.tolerance * 1000.0,
                    condition_number: None,
                }))
            }
            None => Err(anyhow!("QR decomposition failed")),
        }
    }

    /// Sparse LU solve (simplified - using conversion to dense for now)
    fn solve_lu_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        // Convert to dense for now - in a real implementation, you'd use a sparse LU library
        let dense_matrix = sparse_to_dense(matrix);
        let dense_rhs = DVector::from_vec(rhs.to_vec());
        
        let (solution, stats) = self.solve_lu_dense(&dense_matrix, &dense_rhs)?;
        
        Ok((solution.as_slice().to_vec(), stats))
    }

    /// BiCGSTAB iterative solver for sparse matrices
    fn solve_bicgstab_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let n = matrix.rows();
        let mut x = vec![0.0; n]; // Initial guess
        let mut r = rhs.to_vec();
        
        // r = b - A*x (initial residual)
        let ax = sparse_matrix_vector_multiply(matrix, &x);
        for i in 0..n {
            r[i] -= ax[i];
        }
        
        let r_hat = r.clone();
        let mut p = r.clone();
        let mut v = vec![0.0; n];
        let mut h = vec![0.0; n];
        let mut s = vec![0.0; n];
        let mut _t = vec![0.0; n];
        
        let mut rho = 1.0;
        let mut alpha = 1.0;
        let mut omega = 1.0;
        
        let mut residual_norm = vector_norm(&r);
        let _initial_residual = residual_norm;
        
        for iteration in 0..self.config.max_iterations {
            if residual_norm < self.config.tolerance {
                return Ok((x, SolverStats {
                    method_used: SolverMethod::BiCgStab,
                    iterations: iteration,
                    residual_norm,
                    solve_time: 0.0,
                    success: true,
                    condition_number: None,
                }));
            }
            
            let rho_new = vector_dot(&r_hat, &r);
            
            if rho_new.abs() < 1e-15 {
                break; // BiCGSTAB breakdown
            }
            
            let beta = (rho_new / rho) * (alpha / omega);
            rho = rho_new;
            
            // p = r + beta * (p - omega * v)
            for i in 0..n {
                p[i] = r[i] + beta * (p[i] - omega * v[i]);
            }
            
            // v = A * p
            v = sparse_matrix_vector_multiply(matrix, &p);
            
            alpha = rho / vector_dot(&r_hat, &v);
            
            // h = x + alpha * p
            for i in 0..n {
                h[i] = x[i] + alpha * p[i];
            }
            
            // s = r - alpha * v
            for i in 0..n {
                s[i] = r[i] - alpha * v[i];
            }
            
            // t = A * s
            _t = sparse_matrix_vector_multiply(matrix, &s);
            
            omega = vector_dot(&_t, &s) / vector_dot(&_t, &_t);
            
            // x = h + omega * s
            for i in 0..n {
                x[i] = h[i] + omega * s[i];
            }
            
            // r = s - omega * t
            for i in 0..n {
                r[i] = s[i] - omega * _t[i];
            }
            
            residual_norm = vector_norm(&r);
            
            if omega.abs() < 1e-15 {
                break; // BiCGSTAB breakdown
            }
        }
        
        Ok((x, SolverStats {
            method_used: SolverMethod::BiCgStab,
            iterations: self.config.max_iterations,
            residual_norm,
            solve_time: 0.0,
            success: residual_norm < self.config.tolerance,
            condition_number: None,
        }))
    }

    /// Conjugate Gradient solver for symmetric positive definite matrices
    fn solve_cg_sparse(&self, matrix: &CsMat<f64>, rhs: &[f64]) -> Result<(Vec<f64>, SolverStats)> {
        let n = matrix.rows();
        let mut x = vec![0.0; n]; // Initial guess
        let mut r = rhs.to_vec();
        
        // r = b - A*x (initial residual)
        let ax = sparse_matrix_vector_multiply(matrix, &x);
        for i in 0..n {
            r[i] -= ax[i];
        }
        
        let mut p = r.clone();
        let mut rsold = vector_dot(&r, &r);
        
        for iteration in 0..self.config.max_iterations {
            let residual_norm = rsold.sqrt();
            
            if residual_norm < self.config.tolerance {
                return Ok((x, SolverStats {
                    method_used: SolverMethod::Cg,
                    iterations: iteration,
                    residual_norm,
                    solve_time: 0.0,
                    success: true,
                    condition_number: None,
                }));
            }
            
            let ap = sparse_matrix_vector_multiply(matrix, &p);
            let alpha = rsold / vector_dot(&p, &ap);
            
            // x = x + alpha * p
            for i in 0..n {
                x[i] += alpha * p[i];
            }
            
            // r = r - alpha * Ap
            for i in 0..n {
                r[i] -= alpha * ap[i];
            }
            
            let rsnew = vector_dot(&r, &r);
            let beta = rsnew / rsold;
            
            // p = r + beta * p
            for i in 0..n {
                p[i] = r[i] + beta * p[i];
            }
            
            rsold = rsnew;
        }
        
        Ok((x, SolverStats {
            method_used: SolverMethod::Cg,
            iterations: self.config.max_iterations,
            residual_norm: rsold.sqrt(),
            solve_time: 0.0,
            success: rsold.sqrt() < self.config.tolerance,
            condition_number: None,
        }))
    }
}

impl Default for LinearSolver {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

/// Convert sparse matrix to dense matrix
fn sparse_to_dense(sparse: &CsMat<f64>) -> DMatrix<f64> {
    let mut dense = DMatrix::zeros(sparse.rows(), sparse.cols());
    
    for (value, (row, col)) in sparse.iter() {
        dense[(row, col)] = *value;
    }
    
    dense
}

/// Sparse matrix-vector multiplication
fn sparse_matrix_vector_multiply(matrix: &CsMat<f64>, vector: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; matrix.rows()];
    
    for (value, (row, col)) in matrix.iter() {
        result[row] += value * vector[col];
    }
    
    result
}

/// Vector dot product
fn vector_dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Vector L2 norm
fn vector_norm(vector: &[f64]) -> f64 {
    vector.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Check if matrix is symmetric (for CG solver selection)
pub fn is_symmetric(matrix: &CsMat<f64>, tolerance: f64) -> bool {
    if matrix.rows() != matrix.cols() {
        return false;
    }
    
    // This is a simplified check - for a proper implementation,
    // you'd need to check all non-zero elements
    for (value, (row, col)) in matrix.iter() {
        if row != col {
            // Find the transpose element
            let transpose_value = matrix.get(col, row).unwrap_or(&0.0);
            if (value - transpose_value).abs() > tolerance {
                return false;
            }
        }
    }
    
    true
}

/// Auto-select best solver method based on matrix properties
pub fn auto_select_solver(matrix: &CsMat<f64>) -> SolverMethod {
    let size = matrix.rows();
    let nnz = matrix.nnz();
    let density = nnz as f64 / (size * size) as f64;
    
    // Use heuristics to select solver
    if size < 100 || density > 0.1 {
        // Small or dense matrices - use direct solver
        SolverMethod::Lu
    } else if is_symmetric(matrix, 1e-12) {
        // Symmetric matrices - use CG
        SolverMethod::Cg
    } else {
        // Large sparse non-symmetric - use BiCGSTAB
        SolverMethod::BiCgStab
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sprs::TriMat;

    #[test]
    fn test_dense_lu_solver() {
        let solver = LinearSolver::new();
        
        // Create a simple 2x2 system: [2 1; 1 2] * [x; y] = [3; 3]
        // Solution should be [1; 1]
        let matrix = DMatrix::from_row_slice(2, 2, &[2.0, 1.0, 1.0, 2.0]);
        let rhs = DVector::from_vec(vec![3.0, 3.0]);
        
        let (solution, stats) = solver.solve_dense(&matrix, &rhs).unwrap();
        
        assert!((solution[0] - 1.0).abs() < 1e-10);
        assert!((solution[1] - 1.0).abs() < 1e-10);
        assert!(stats.success);
        assert_eq!(stats.method_used, SolverMethod::Lu);
    }

    #[test]
    fn test_sparse_solver() {
        let solver = LinearSolver::new();
        
        // Create the same system in sparse format
        let mut triplets = Vec::new();
        triplets.push((0, 0, 2.0));
        triplets.push((0, 1, 1.0));
        triplets.push((1, 0, 1.0));
        triplets.push((1, 1, 2.0));
        
        let mut triplet_mat = TriMat::new((2, 2));
        for (row, col, value) in triplets {
            triplet_mat.add_triplet(row, col, value);
        }
        let matrix = triplet_mat.to_csr();
        let rhs = vec![3.0, 3.0];
        
        let (solution, stats) = solver.solve_sparse(&matrix, &rhs).unwrap();
        
        assert!((solution[0] - 1.0).abs() < 1e-10);
        assert!((solution[1] - 1.0).abs() < 1e-10);
        assert!(stats.success);
    }

    #[test]
    fn test_auto_solver_selection() {
        // Small matrix should select LU
        let mut small_triplet = TriMat::new((2, 2));
        small_triplet.add_triplet(0, 0, 1.0);
        small_triplet.add_triplet(1, 1, 1.0);
        let small_matrix = small_triplet.to_csr();
        assert_eq!(auto_select_solver(&small_matrix), SolverMethod::Lu);
    }
} 