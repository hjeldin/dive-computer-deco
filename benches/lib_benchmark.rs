use criterion::{criterion_group, criterion_main, Criterion};
use dive_computer_deco::{
    ceiling::{ceiling, max_ceiling, binary_ceiling},
    ndl::{ndl, binary_ndl},
    simulate::simulate,
    tissue::calculate_tissue,
    tissue::Tissue,
    DiveParameters,
    water_vapor_pressure, FN2, FHE,
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
        b.iter(|| ceiling(params, tissue, 0, true))
    });

    // Benchmark binary ceiling calculation
    group.bench_function("single_tissue_binary_ceiling", |b| {
        b.iter(|| binary_ceiling(params, tissue, 0, true))
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

fn benchmark_ndl_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ndl_calculations");
    let params = DiveParameters::default();
    let temperature = 20.0;
    let target_depth = 30.0;
    let amb_pressure = target_depth / 10.0 + 1.0;

    // Helper function to prepare tissues for NDL calculation
    fn prepare_tissues(target_depth: f32, temperature: f32) -> [Tissue; 16] {
        let mut tissues = [Tissue::default(); 16];
        let start_amb_pressure = 1.0;
        
        // Reset tissues to surface conditions
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (start_amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (start_amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
        
        // Simulate descent to target depth
        let mut params = DiveParameters::default();
        simulate(
            &mut params,
            &mut tissues,
            1.0,
            target_depth,
            temperature,
            1.0,
            0.0,
        );
        tissues
    }

    // Benchmark regular NDL calculation
    group.bench_function("regular_ndl", |b| {
        b.iter(|| {
            let mut tissues = prepare_tissues(target_depth, temperature);
            ndl(params, &mut tissues, amb_pressure, temperature)
        })
    });

    // Benchmark binary NDL calculation
    group.bench_function("binary_ndl", |b| {
        b.iter(|| {
            let mut tissues = prepare_tissues(target_depth, temperature);
            binary_ndl(params, &mut tissues, amb_pressure, temperature)
        })
    });

    group.finish();
}

fn benchmark_method_comparisons(c: &mut Criterion) {
    let mut group = c.benchmark_group("method_comparisons");
    
    // Ceiling method comparison
    let tissue = Tissue {
        load_n2: 3.0,
        load_he: 0.0,
    };
    let params = DiveParameters::default();

    group.bench_function("ceiling_regular_vs_binary", |b| {
        b.iter(|| {
            let regular = ceiling(params, tissue, 0, true);
            let binary = binary_ceiling(params, tissue, 0, true);
            (regular, binary)
        })
    });

    // NDL method comparison (using shallow depth for faster execution)
    let temperature = 20.0;
    let target_depth = 21.0;  // Shallower depth for faster NDL calculation
    let amb_pressure = target_depth / 10.0 + 1.0;

    group.bench_function("ndl_regular_vs_binary", |b| {
        b.iter(|| {
            let mut tissues1 = [Tissue::default(); 16];
            let mut tissues2 = [Tissue::default(); 16];
            let start_amb_pressure = 1.0;
            
            // Prepare identical tissue states
            for i in 0..tissues1.len() {
                tissues1[i].load_n2 = (start_amb_pressure - water_vapor_pressure(temperature)) * FN2;
                tissues1[i].load_he = (start_amb_pressure - water_vapor_pressure(temperature)) * FHE;
                tissues2[i] = tissues1[i];
            }
            
            // Simulate descent for both
            let mut params1 = DiveParameters::default();
            let mut params2 = DiveParameters::default();
            simulate(&mut params1, &mut tissues1, 1.0, target_depth, temperature, 1.0, 0.0);
            simulate(&mut params2, &mut tissues2, 1.0, target_depth, temperature, 1.0, 0.0);
            
            let regular = ndl(params, &mut tissues1, amb_pressure, temperature);
            let binary = binary_ndl(params, &mut tissues2, amb_pressure, temperature);
            (regular, binary)
        })
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
    benchmark_ndl_calculations,
    benchmark_method_comparisons,
    benchmark_simulation
);
criterion_main!(benches);
