use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use dive_computer_deco::{
    DiveParameters,
    tissue::Tissue,
    simulate::{simulate_with_ascent, SimulationOutputs},
    ceiling::max_ceiling_with_gf,
    water_vapor_pressure, FN2, FHE,
};

#[derive(Clone)]
struct DiveStep {
    depth: f32,
    duration: f32, // in minutes
}

impl DiveStep {
    fn new() -> Self {
        Self {
            depth: 18.0,
            duration: 20.0,
        }
    }
}

struct DivePlannerApp {
    // Dive parameters
    gf_low: f32,
    gf_high: f32,
    surface_pressure: f32,
    
    // Dive profile
    dive_steps: Vec<DiveStep>,
    
    // Simulation results
    simulation_results: Option<SimulationOutputs>,
    simulation_text: String,
    
    // UI state
    show_ceiling: bool,
    show_depth: bool,
    show_pressure: bool,
}

impl Default for DivePlannerApp {
    fn default() -> Self {
        Self {
            gf_low: 0.30,
            gf_high: 0.85,
            surface_pressure: 1.0,
            dive_steps: vec![DiveStep::new()],
            simulation_results: None,
            simulation_text: String::new(),
            show_ceiling: true,
            show_depth: true,
            show_pressure: false,
        }
    }
}

impl eframe::App for DivePlannerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Use regular egui layout with improved styling
            ui.horizontal(|ui| {
                // Left column - Control panels with visual blocks
                ui.vertical(|ui| {
                    ui.set_min_width(380.0);
                    ui.set_max_width(400.0);
                    
                    // Parameters block with styling
                    ui.group(|ui| {
                        ui.set_min_width(360.0);
                        ui.spacing_mut().item_spacing.y = 8.0;
                        self.parameters_panel(ui);
                    });
                    
                    ui.add_space(16.0);
                    
                    // Dive profile block with styling
                    ui.group(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        self.dive_profile_panel(ui);
                    });
                    
                    ui.add_space(16.0);
                    
                    // Controls block with styling
                    ui.group(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        self.simulation_controls_panel(ui);
                    });
                });

                ui.separator();
                ui.add_space(16.0);

                // Right column - Results with styling
                ui.vertical(|ui| {
                    ui.set_min_width(600.0);
                    ui.group(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        self.results_panel(ui);
                    });
                });
            });
        });
    }
}

impl DivePlannerApp {
    fn parameters_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔧 Dive Parameters");
        ui.add_space(8.0);
        
        // Grid layout for parameters
        egui::Grid::new("parameters_grid")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .show(ui, |ui| {
                ui.label("GF Low:");
                ui.add(egui::Slider::new(&mut self.gf_low, 0.1..=0.99)
                    .suffix("%")
                    .custom_formatter(|n, _| format!("{:.0}", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|v| v / 100.0)));
                ui.end_row();
                
                ui.label("GF High:");
                ui.add(egui::Slider::new(&mut self.gf_high, 0.1..=0.99)
                    .suffix("%")
                    .custom_formatter(|n, _| format!("{:.0}", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|v| v / 100.0)));
                ui.end_row();
                
                ui.label("Surface Pressure:");
                ui.add(egui::DragValue::new(&mut self.surface_pressure)
                    .speed(0.01)
                    .range(0.8..=1.2)
                    .suffix(" bar"));
                ui.end_row();
            });
        
        // Ensure GF Low <= GF High
        if self.gf_low > self.gf_high {
            self.gf_low = self.gf_high;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 150, 255), "📊 Current GF:");
            ui.colored_label(egui::Color32::LIGHT_GRAY, 
                format!("{:.0}%/{:.0}%", self.gf_low * 100.0, self.gf_high * 100.0));
        });
    }
    
    fn dive_profile_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🏊 Dive Profile");
        ui.add_space(8.0);
        
        // Control buttons with styling
        ui.horizontal(|ui| {
            if ui.button("➕ Add Step").clicked() {
                self.dive_steps.push(DiveStep::new());
            }
            
            if self.dive_steps.len() > 1 {
                if ui.button("➖ Remove Last").clicked() {
                    self.dive_steps.pop();
                }
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(egui::Color32::GRAY, format!("{} steps", self.dive_steps.len()));
            });
        });
        
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        
        // Table with grid layout
        egui::Grid::new("dive_profile_grid")
            .num_columns(4)
            .spacing([20.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                // Header
                ui.strong("Step");
                ui.strong("Depth (m)");
                ui.strong("Duration (min)");
                ui.strong("Actions");
                ui.end_row();
                
                // Dive steps
                let mut to_remove = None;
                let dive_steps_len = self.dive_steps.len();
                for (i, step) in self.dive_steps.iter_mut().enumerate() {
                    ui.label(format!("{}", i + 1));
                    
                    ui.add(egui::DragValue::new(&mut step.depth)
                        .speed(0.5)
                        .range(0.0..=200.0)
                        .suffix(" m"));
                    
                    ui.add(egui::DragValue::new(&mut step.duration)
                        .speed(0.5)
                        .range(0.1..=300.0)
                        .suffix(" min"));
                    
                    if dive_steps_len > 1 && ui.small_button("🗑").clicked() {
                        to_remove = Some(i);
                    }
                    ui.end_row();
                }
                
                if let Some(index) = to_remove {
                    self.dive_steps.remove(index);
                }
            });
        
        // Calculate total dive time
        let total_time: f32 = self.dive_steps.iter().map(|step| step.duration).sum();
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 150, 255), "⏱ Total Bottom Time:");
            ui.colored_label(egui::Color32::LIGHT_GRAY, format!("{:.1} minutes", total_time));
        });
    }
    
    fn simulation_controls_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🚀 Simulation");
        ui.add_space(8.0);
        
        // Large simulation button
        let button_response = ui.add_sized([ui.available_width(), 32.0], 
            egui::Button::new("� Simulate Dive"));
        
        if button_response.clicked() {
            self.run_simulation();
        }
        
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
        
        // Plot options with better layout
        ui.label("📈 Plot Options:");
        ui.add_space(4.0);
        
        egui::Grid::new("plot_options_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.checkbox(&mut self.show_depth, "🌊 Depth Profile");
                ui.checkbox(&mut self.show_ceiling, "🚧 Ceiling");
                ui.end_row();
                ui.checkbox(&mut self.show_pressure, "📊 Pressure");
                ui.label(""); // Empty cell for alignment
                ui.end_row();
            });
    }
    
    fn results_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Simulation Results");
        ui.add_space(8.0);
        
        if self.simulation_results.is_none() {
            ui.colored_label(egui::Color32::GRAY, "Click 'Simulate Dive' to see results");
            return;
        }
        
        // Text results
        ui.collapsing("Dive Summary", |ui| {
            ui.horizontal(|ui| {
                if ui.button("📋 Copy").clicked() {
                    ui.ctx().copy_text(self.simulation_text.clone());
                }
            });
            
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.simulation_text.as_str())
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace));
                });
        });
        
        ui.separator();
        
        // Main dive profile plot
        self.dive_profile_plot(ui);
        
        ui.separator();
        
        // Second plot (placeholder for future enhancements)
        self.secondary_plot(ui);
    }
    
    fn dive_profile_plot(&mut self, ui: &mut egui::Ui) {
        if let Some(ref results) = self.simulation_results {
            ui.label("Dive Profile");
            
            let plot = Plot::new("dive_profile")
                .height(300.0)
                .legend(egui_plot::Legend::default())
                .y_axis_label("Depth (m)")
                .x_axis_label("Time (minutes)")
                .include_y(0.0);
            
            plot.show(ui, |plot_ui| {
                if self.show_depth && !results.depths.is_empty() {
                    let depth_points: PlotPoints = results.depths
                        .iter()
                        .enumerate()
                        .map(|(i, &depth)| {
                            let time_minutes = i as f64 * 10.0 / 60.0; // Assuming 10-second intervals
                            [time_minutes, -depth as f64] // Negative depth for proper visualization
                        })
                        .collect();
                    
                    plot_ui.line(
                        Line::new("Depth", depth_points)
                            .color(egui::Color32::BLUE)
                            .width(2.0)
                    );
                }
                
                if self.show_ceiling && !results.tissues_per_interval.is_empty() {
                    let ceiling_points: PlotPoints = results.tissues_per_interval
                        .iter()
                        .enumerate()
                        .map(|(i, tissues)| {
                            let time_minutes = i as f64 * 10.0 / 60.0;
                            let (ceiling, _) = max_ceiling_with_gf(self.gf_high, tissues);
                            [time_minutes, -(ceiling as f64)] // Negative for proper visualization
                        })
                        .collect();
                    
                    plot_ui.line(
                        Line::new("Ceiling", ceiling_points)
                            .color(egui::Color32::RED)
                            .width(1.5)
                            .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                    );
                }
                
                if self.show_pressure && !results.pressures.is_empty() {
                    let pressure_points: PlotPoints = results.pressures
                        .iter()
                        .enumerate()
                        .map(|(i, &pressure)| {
                            let time_minutes = i as f64 * 10.0 / 60.0;
                            [time_minutes, (pressure as f64 - 1.0) * 10.0] // Convert to depth equivalent
                        })
                        .collect();
                    
                    plot_ui.line(
                        Line::new("Pressure", pressure_points)
                            .color(egui::Color32::GREEN)
                            .width(1.0)
                    );
                }
            });
        }
    }
    
    fn secondary_plot(&mut self, ui: &mut egui::Ui) {
        ui.label("Future Enhancement Plot");
        
        let plot = Plot::new("secondary_plot")
            .height(200.0)
            .legend(egui_plot::Legend::default())
            .y_axis_label("Value")
            .x_axis_label("Time");
        
        plot.show(ui, |plot_ui| {
            // Placeholder for future enhancements
            let placeholder_points: PlotPoints = vec![[0.0, 0.0], [1.0, 0.0]].into();
            plot_ui.line(
                Line::new("Placeholder", placeholder_points)
                    .color(egui::Color32::GRAY)
            );
            
            // Placeholder text for future enhancement
            // plot_ui.text(
            //     egui_plot::Text::new([0.5, 0.0].into(), "Future enhancements here")
            // );
        });
    }
    
    fn run_simulation(&mut self) {
        if self.dive_steps.is_empty() {
            self.simulation_text = "Error: No dive steps defined".to_string();
            return;
        }
        
        // Initialize tissues at surface pressure
        let mut tissues = [Tissue::default(); 16];
        let temperature = 20.0; // Fixed temperature for now
        
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (self.surface_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (self.surface_pressure - water_vapor_pressure(temperature)) * FHE;
        }
        
        // Create dive parameters
        let mut dive_params = DiveParameters::new(self.gf_high, self.gf_low);
        
        let mut all_results = SimulationOutputs::new();
        let mut dive_text = String::new();
        dive_text.push_str(&format!("=== DIVE PLAN ===\n"));
        dive_text.push_str(&format!("GF Low/High: {:.0}%/{:.0}%\n", 
            self.gf_low * 100.0, self.gf_high * 100.0));
        dive_text.push_str(&format!("Surface Pressure: {:.2} bar\n\n", self.surface_pressure));
        
        let mut total_runtime = 0.0;
        
        for (step_num, step) in self.dive_steps.iter().enumerate() {
            dive_text.push_str(&format!("Step {}: {}m for {:.1} minutes\n", 
                step_num + 1, step.depth, step.duration));
            
            // Simulate this step
            let step_results = simulate_with_ascent(
                &mut dive_params,
                &mut tissues,
                self.surface_pressure,
                step.depth,
                temperature,
                10.0, // 10-second intervals
                step.duration * 60.0, // Convert to seconds
                step_num == self.dive_steps.len() - 1 // Only include ascent on last step
            );
            
            // Accumulate results
            for depth in step_results.depths.iter() {
                all_results.depths.push(*depth);
            }
            for pressure in step_results.pressures.iter() {
                all_results.pressures.push(*pressure);
            }
            for tissue_set in step_results.tissues_per_interval.iter() {
                all_results.tissues_per_interval.push(*tissue_set);
            }
            
            total_runtime += step.duration;
        }
        
        // Calculate final ceiling and decompression status
        let (final_ceiling, controlling_tissue) = max_ceiling_with_gf(self.gf_high, &tissues);
        
        dive_text.push_str(&format!("\n=== SIMULATION RESULTS ===\n"));
        dive_text.push_str(&format!("Total Bottom Time: {:.1} minutes\n", total_runtime));
        dive_text.push_str(&format!("Final Ceiling: {}m\n", final_ceiling));
        dive_text.push_str(&format!("Controlling Tissue: {}\n", controlling_tissue));
        
        if final_ceiling > 0 {
            dive_text.push_str(&format!("\n⚠️  DECOMPRESSION REQUIRED ⚠️\n"));
            dive_text.push_str(&format!("Mandatory decompression ceiling: {}m\n", final_ceiling));
            
            // Calculate approximate decompression stops
            dive_text.push_str(&format!("\nApproximate Decompression Schedule:\n"));
            let mut current_depth = ((final_ceiling as f32 + 2.0) / 3.0).ceil() * 3.0;
            while current_depth >= 3.0 {
                dive_text.push_str(&format!("  {}m: variable time\n", current_depth as u32));
                current_depth -= 3.0;
            }
        } else {
            dive_text.push_str(&format!("\n✅ NO DECOMPRESSION REQUIRED\n"));
            dive_text.push_str(&format!("Direct ascent to surface allowed\n"));
        }
        
        // Add tissue loading information
        dive_text.push_str(&format!("\n=== TISSUE LOADING ===\n"));
        for (i, tissue) in tissues.iter().enumerate() {
            let loading_percent = (tissue.load_n2 / 
                dive_computer_deco::m_value::calculate_m_values(self.surface_pressure, i)) * 100.0;
            dive_text.push_str(&format!("Tissue {}: {:.1}%\n", i + 1, loading_percent));
        }
        
        self.simulation_results = Some(all_results);
        self.simulation_text = dive_text;
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Dive Computer Decompression Planner",
        options,
        Box::new(|_cc| Ok(Box::new(DivePlannerApp::default()))),
    )
}