use criterion::{criterion_group, criterion_main, Criterion};
use dive_computer_deco::{
    ceiling::{ceiling, max_ceiling},
    simulate::simulate,
    tissue::calculate_tissue,
    tissue::Tissue,
    DiveParameters,
};

fn benchmark_tissue_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("tissue_calculations");
    let tissue = Tissue::default();

    // Benchmark tissue calculation (saturation)
    group.bench_function("tissue_saturation", |b| {
        b.iter(|| calculate_tissue(tissue, 0, 3.0, 20.0, 1.0 / 60.0))
    });

    // Benchmark tissue calculation (desaturation)
    group.bench_function("tissue_desaturation", |b| {
        let saturated_tissue = Tissue {
            load_n2: 3.0,
            load_he: 0.0,
        };
        b.iter(|| calculate_tissue(saturated_tissue, 0, 1.0, 20.0, 1.0 / 60.0))
    });

    group.finish();
}

fn benchmark_ceiling_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ceiling_calculations");
    let tissue = Tissue {
        load_n2: 3.0,
        load_he: 0.0,
    };
    let params = DiveParameters::default();

    // Benchmark single tissue ceiling calculation
    group.bench_function("single_tissue_ceiling", |b| {
        b.iter(|| ceiling(params, tissue, 0))
    });

    // Benchmark max ceiling across all tissues
    group.bench_function("max_ceiling", |b| {
        let mut tissues = [Tissue::default(); 16];
        for i in 0..16 {
            tissues[i] = Tissue {
                load_n2: 1.0 + (i as f32 * 0.1),
                load_he: 0.0,
            };
        }
        b.iter(|| max_ceiling(params, &tissues))
    });

    group.finish();
}

fn benchmark_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dive_simulation");

    // Benchmark a basic dive simulation
    group.bench_function("simulate_20m_20min", |b| {
        b.iter(|| {
            let mut params = DiveParameters::default();
            let mut tissues = [Tissue::default(); 16];
            let starting_pressure = 1.0; // 1 bar at surface
            let target_depth = 20.0; // 20m
            let temperature = 20.0; // 20°C
            let interval = 1.0; // 1 second intervals
            let bottom_time = 20.0 * 60.0; // 20 minutes

            simulate(
                &mut params,
                &mut tissues,
                starting_pressure,
                target_depth,
                temperature,
                interval,
                bottom_time,
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_tissue_calculations,
    benchmark_ceiling_calculations,
    benchmark_simulation
);
criterion_main!(benches);
