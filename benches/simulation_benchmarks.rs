use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use RustSim::*;

fn bench_simple_circuit(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_circuit");
    
    // Create a simple RC circuit for benchmarking
    let netlist_content = r#"
Simple RC Circuit
V1 1 0 DC 5V
R1 1 2 1k
C1 2 0 1n
.op
.end
"#;
    
    let parser = SpiceParser::new();
    let netlist = parser.parse_netlist(netlist_content).unwrap();
    
    group.bench_function("parse_netlist", |b| {
        b.iter(|| {
            parser.parse_netlist(netlist_content).unwrap()
        });
    });
    
    group.bench_function("operating_point", |b| {
        b.iter(|| {
            let mut simulator = Simulator::new();
            simulator.load_netlist_from_parsed(netlist.clone()).unwrap();
            simulator.run_operating_point().unwrap();
        });
    });
    
    group.finish();
}

fn bench_matrix_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_solver");
    
    // Test different matrix sizes
    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::new("lu_solver", size), size, |b, &size| {
            use nalgebra::{DMatrix, DVector};
            use RustSim::solver::LinearSolver;
            
            // Create a random positive definite matrix
            let mut matrix = DMatrix::<f64>::identity(size, size);
            for i in 0..size {
                matrix[(i, i)] = 2.0;
                if i > 0 {
                    matrix[(i, i-1)] = -1.0;
                    matrix[(i-1, i)] = -1.0;
                }
            }
            
            let rhs = DVector::from_element(size, 1.0);
            let solver = LinearSolver::new();
            
            b.iter(|| {
                solver.solve_dense(&matrix, &rhs).unwrap()
            });
        });
    }
    
    group.finish();
}

fn bench_transient_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("transient_analysis");
    
    let netlist_content = r#"
RC Circuit Transient
V1 1 0 DC 5V
R1 1 2 1k
C1 2 0 1n
.tran 1ns 100ns
.end
"#;
    
    let parser = SpiceParser::new();
    let netlist = parser.parse_netlist(netlist_content).unwrap();
    
    group.bench_function("transient_100_steps", |b| {
        b.iter(|| {
            let mut simulator = Simulator::new();
            simulator.load_netlist_from_parsed(netlist.clone()).unwrap();
            simulator.run_transient_analysis(1e-9, 100e-9).unwrap();
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_simple_circuit, bench_matrix_sizes, bench_transient_analysis);
criterion_main!(benches); 