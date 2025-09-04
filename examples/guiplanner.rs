use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use dive_computer_deco::{
    DiveParameters,
    tissue::{Tissue, calculate_tissue},
    simulate::SimulationOutputs,
    ceiling::max_ceiling_with_gf,
    m_value::calculate_m_values,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum PlotTab {
    DiveProfile,
    TissuePressure,
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
    active_tab: PlotTab,
    tissue_visibility: [bool; 16], // Visibility toggle for each tissue compartment
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
            active_tab: PlotTab::DiveProfile,
            tissue_visibility: [true; 16], // All tissues visible by default
        }
    }
}

impl eframe::App for DivePlannerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Use regular egui layout with improved styling - take full height
            let available_rect = ui.available_rect_before_wrap();
            ui.allocate_ui_with_layout(
                available_rect.size(),
                egui::Layout::left_to_right(egui::Align::TOP),
                |ui| {
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
                    // Remove the group wrapper to allow full height usage
                    ui.spacing_mut().item_spacing.y = 8.0;
                    self.results_panel(ui);
                });
            }
        );
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
                .max_height(150.0) // Reduced from 200.0 to leave more space for plots
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.simulation_text.as_str())
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace));
                });
        });
        
        ui.separator();
        ui.add_space(8.0);
        
        // Tabbed plots
        egui::TopBottomPanel::top("plot_tabs").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, PlotTab::DiveProfile, "🌊 Dive Profile");
                ui.selectable_value(&mut self.active_tab, PlotTab::TissuePressure, "🧬 Tissue Pressure");
            });
        });
        
        ui.add_space(8.0);
        
        // Show the selected tab content
        match self.active_tab {
            PlotTab::DiveProfile => self.dive_profile_plot(ui),
            PlotTab::TissuePressure => self.secondary_plot(ui),
        }
    }
    
    fn dive_profile_plot(&mut self, ui: &mut egui::Ui) {
        if let Some(ref results) = self.simulation_results {
            ui.label("Dive Profile");
            
            // Calculate available height for the plot
            let available_height = ui.available_height() - 40.0; // Leave some margin
            let plot_height = available_height.max(300.0); // Minimum height of 300
            
            let plot = Plot::new("dive_profile")
                .height(plot_height)
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
        ui.horizontal(|ui| {
            ui.label("Tissue Pressure vs Depth");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Show All").clicked() {
                    self.tissue_visibility = [true; 16];
                }
                if ui.button("Hide All").clicked() {
                    self.tissue_visibility = [false; 16];
                }
            });
        });
        
        // Tissue visibility controls
        ui.collapsing("🧬 Tissue Visibility", |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            
            // Create a grid for tissue toggles (4 columns)
            egui::Grid::new("tissue_visibility_grid")
                .num_columns(4)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    for i in 0..16 {
                        ui.checkbox(&mut self.tissue_visibility[i], format!("T{}", i + 1));
                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
        
        ui.add_space(8.0);
        
        // Calculate available height for the plot
        let available_height = ui.available_height() - 40.0; // Leave some margin
        let plot_height = available_height.max(300.0); // Minimum height of 300
        
        let plot = Plot::new("secondary_plot")
            .height(plot_height)
            .legend(egui_plot::Legend::default())
            .y_axis_label("Absolute Tissue Pressure (bar)")
            .x_axis_label("Depth (m)")
            .include_y(1.0);
        
        plot.show(ui, |plot_ui| {
            if let Some(ref results) = self.simulation_results {
                if !results.depths.is_empty() && !results.tissues_per_interval.is_empty() {
                    // Generate colors for each tissue compartment
                    let colors = [
                        egui::Color32::from_rgb(255, 100, 100), // Red
                        egui::Color32::from_rgb(255, 150, 100), // Orange-red
                        egui::Color32::from_rgb(255, 200, 100), // Orange
                        egui::Color32::from_rgb(255, 255, 100), // Yellow
                        egui::Color32::from_rgb(200, 255, 100), // Yellow-green
                        egui::Color32::from_rgb(150, 255, 100), // Light green
                        egui::Color32::from_rgb(100, 255, 100), // Green
                        egui::Color32::from_rgb(100, 255, 150), // Green-cyan
                        egui::Color32::from_rgb(100, 255, 200), // Cyan-green
                        egui::Color32::from_rgb(100, 255, 255), // Cyan
                        egui::Color32::from_rgb(100, 200, 255), // Light blue
                        egui::Color32::from_rgb(100, 150, 255), // Blue
                        egui::Color32::from_rgb(100, 100, 255), // Dark blue
                        egui::Color32::from_rgb(150, 100, 255), // Blue-purple
                        egui::Color32::from_rgb(200, 100, 255), // Purple
                        egui::Color32::from_rgb(255, 100, 255), // Magenta
                    ];

                    // Plot each tissue compartment
                    for tissue_idx in 0..16 {
                        if !self.tissue_visibility[tissue_idx] {
                            continue; // Skip if tissue is not visible
                        }
                        
                        let tissue_points: PlotPoints = results.tissues_per_interval
                            .iter()
                            .enumerate()
                            .map(|(i, tissues)| {
                                let depth = results.depths.get(i).unwrap_or(&0.0);
                                let tissue_pressure = tissues[tissue_idx].load_n2;
                                [*depth as f64, tissue_pressure as f64]
                            })
                            .collect();
                        
                        plot_ui.line(
                            Line::new(format!("Tissue {}", tissue_idx + 1), tissue_points)
                                .color(colors[tissue_idx])
                                .width(1.5)
                        );
                    }

                    // Plot M-values for each tissue compartment
                    for tissue_idx in 0..16 {
                        if !self.tissue_visibility[tissue_idx] {
                            continue; // Skip if tissue is not visible
                        }
                        
                        let m_value_points: PlotPoints = results.depths
                            .iter()
                            .map(|&depth| {
                                let ambient_pressure = self.surface_pressure + (depth / 10.0); // Convert depth to pressure
                                let m_value = calculate_m_values(ambient_pressure, tissue_idx);
                                [depth as f64, m_value as f64]
                            })
                            .collect();
                        
                        plot_ui.line(
                            Line::new(format!("M-Value {}", tissue_idx + 1), m_value_points)
                                .color(colors[tissue_idx])
                                .width(1.0)
                                .style(egui_plot::LineStyle::Dashed { length: 3.0 })
                        );
                    }
                }
            } else {
                // Show placeholder when no simulation results
                let placeholder_points: PlotPoints = vec![[0.0, 1.0], [30.0, 1.0]].into();
                plot_ui.line(
                    Line::new("No data - run simulation", placeholder_points)
                        .color(egui::Color32::GRAY)
                );
            }
        });
    }
    
    fn get_responsible_tissues(&self, tissues: &[Tissue; 16]) -> Vec<(usize, u32, f32)> {
        let mut responsible_tissues = Vec::new();
        
        for i in 0..16 {
            let tissue_ceiling = dive_computer_deco::ceiling::ceiling_with_gf(
                self.gf_high, 
                tissues[i], 
                i, 
                true
            );
            
            if tissue_ceiling > 0 {
                // Calculate tissue loading percentage
                let m_value = dive_computer_deco::m_value::calculate_m_values(self.surface_pressure, i);
                let loading_percent = (tissues[i].load_n2 / m_value) * 100.0;
                responsible_tissues.push((i, tissue_ceiling, loading_percent));
            }
        }
        
        // Sort by ceiling depth (deepest first)
        responsible_tissues.sort_by(|a, b| b.1.cmp(&a.1));
        responsible_tissues
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

        // Create a continuous simulation for all dive steps
        all_results = self.simulate_dive_steps(&mut dive_params, &mut tissues, temperature);
        
        for (step_num, step) in self.dive_steps.iter().enumerate() {
            dive_text.push_str(&format!("Step {}: {}m for {:.1} minutes\n", 
                step_num + 1, step.depth, step.duration));
            total_runtime += step.duration;
        }
        
        // Calculate final ceiling and decompression status
        let (final_ceiling, controlling_tissue) = max_ceiling_with_gf(self.gf_high, &tissues);
        
        // Get all tissues requiring decompression
        let responsible_tissues = self.get_responsible_tissues(&tissues);
        
        // Calculate total dive time from simulation results
        let total_dive_time = if !all_results.depths.is_empty() {
            (all_results.depths.len() as f32 * 10.0) / 60.0 // Convert from 10-second intervals to minutes
        } else {
            total_runtime // Fallback to bottom time only
        };
        
        // Calculate decompression stops from actual simulation data
        let deco_stops = self.calculate_deco_stops_from_results(&all_results);
        let total_deco_time: f32 = deco_stops.iter().map(|(_, time)| time).sum();
        let ascent_time = total_dive_time - total_runtime; // Total time minus bottom time
        
        dive_text.push_str(&format!("\n=== SIMULATION RESULTS ===\n"));
        dive_text.push_str(&format!("Total Bottom Time: {:.1} minutes\n", total_runtime));
        dive_text.push_str(&format!("Total Dive Time: {:.1} minutes\n", total_dive_time));
        dive_text.push_str(&format!("Ascent Time: {:.1} minutes\n", ascent_time));
        dive_text.push_str(&format!("Total Decompression Time: {:.1} minutes\n", total_deco_time));
        dive_text.push_str(&format!("Final Ceiling: {}m\n", final_ceiling));
        dive_text.push_str(&format!("Controlling Tissue: {}\n", controlling_tissue + 1));
        
        if final_ceiling > 0 || !deco_stops.is_empty() {
            dive_text.push_str(&format!("\n⚠️  DECOMPRESSION REQUIRED ⚠️\n"));
            dive_text.push_str(&format!("Mandatory decompression ceiling: {}m\n", final_ceiling));
            
            // Show responsible tissues
            if !responsible_tissues.is_empty() {
                dive_text.push_str(&format!("\nResponsible Tissue(s):\n"));
                for (tissue_idx, ceiling, loading_pct) in &responsible_tissues {
                    dive_text.push_str(&format!("  Tissue {}: {}m ceiling ({:.1}% loaded)\n", 
                        tissue_idx + 1, ceiling, loading_pct));
                }
            }
            
            if !deco_stops.is_empty() {
                dive_text.push_str(&format!("\nDecompression Schedule:\n"));
                for (depth, time) in deco_stops {
                    dive_text.push_str(&format!("  {}m: {:.1} minutes\n", depth as u32, time));
                }
                dive_text.push_str(&format!("\nTotal decompression time: {:.1} minutes\n", total_deco_time));
            } else {
                dive_text.push_str(&format!("\nNo decompression stops detected in simulation\n"));
                dive_text.push_str(&format!("(Final ceiling suggests decompression may be required)\n"));
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
    
    fn simulate_dive_steps(&self, dive_params: &mut DiveParameters, tissues: &mut [Tissue; 16], temperature: f32) -> SimulationOutputs {
        use dive_computer_deco::simulate::simulate_with_ascent_from_depth;
        
        let mut combined_results = SimulationOutputs::new();
        let mut current_depth = 0.0;
        
        for (step_num, step) in self.dive_steps.iter().enumerate() {
            let is_last_step = step_num == self.dive_steps.len() - 1;
            
            // For each step, simulate from current depth to target depth
            let step_results = simulate_with_ascent_from_depth(
                dive_params,
                tissues,
                self.surface_pressure,
                current_depth,
                step.depth,
                temperature,
                10.0, // 10-second intervals
                step.duration * 60.0, // Convert minutes to seconds
                is_last_step, // Only include ascent on the last step
            );
            
            // Append results to combined results
            combined_results.depths.extend(step_results.depths);
            combined_results.pressures.extend(step_results.pressures);
            combined_results.tissues_per_interval.extend(step_results.tissues_per_interval);
            
            // Update current depth for next step
            current_depth = step.depth;
        }
        
        combined_results
    }

    fn calculate_deco_stops_from_results(&self, results: &SimulationOutputs) -> Vec<(f32, f32)> {
        let mut deco_stops: Vec<(f32, f32)> = Vec::new();
        
        if results.depths.is_empty() {
            return deco_stops;
        }
        
        let mut current_stop_depth: Option<f32> = None;
        let mut stop_start_time: f32 = 0.0;
        let mut in_ascent_phase: bool = false;
        let mut max_depth_reached: f32 = 0.0;
        let mut ascending_from_max = false;
        
        // First pass: find maximum depth to determine when ascent starts
        for &depth in &results.depths {
            if depth > max_depth_reached {
                max_depth_reached = depth;
            }
        }
        
        // Calculate total bottom time to help identify when ascent truly begins
        let total_bottom_time_seconds: f32 = self.dive_steps.iter().map(|step| step.duration * 60.0).sum();
        let estimated_bottom_intervals = (total_bottom_time_seconds / 10.0) as usize;
        
        for (i, &depth) in results.depths.iter().enumerate() {
            let time_minutes = i as f32 * 10.0 / 60.0; // Convert from 10-second intervals
            
            // More sophisticated ascent detection:
            // 1. We must be past the estimated bottom time
            // 2. We must be ascending from the maximum depth
            // 3. We must be at a depth that could be a decompression stop (multiple of 3m, >= 3m)
            if !in_ascent_phase {
                if i > estimated_bottom_intervals && 
                   depth < max_depth_reached - 1.0_f32 && 
                   !ascending_from_max {
                    ascending_from_max = true;
                }
                
                if ascending_from_max && depth < max_depth_reached - 2.0_f32 {
                    in_ascent_phase = true;
                }
            }
            
            if in_ascent_phase && depth > 0.0 {
                // Only consider depths that are likely decompression stops:
                // - Multiple of 3 meters (standard deco stop depths)
                // - Between 3m and 50m
                // - Not the original dive step depths
                let is_deco_stop_depth = depth >= 3.0_f32 && 
                                       depth <= 50.0_f32 && 
                                       (depth % 3.0_f32).abs() < 0.5_f32 &&
                                       !self.dive_steps.iter().any(|step| (step.depth - depth).abs() < 1.0_f32);
                
                if !is_deco_stop_depth {
                    // Reset current stop if we're not at a valid deco depth
                    if let Some(stop_depth) = current_stop_depth {
                        let stop_duration = time_minutes - stop_start_time;
                        if stop_duration >= 1.0_f32 { // Minimum 1 minute for a deco stop
                            deco_stops.push((stop_depth, stop_duration));
                        }
                        current_stop_depth = None;
                    }
                    continue;
                }
                
                // Check if we're at a constant depth (potential deco stop)
                if let Some(stop_depth) = current_stop_depth {
                    if (depth - stop_depth).abs() < 0.5_f32 {
                        // Still at the same stop depth
                        continue;
                    } else {
                        // We've moved from the stop depth
                        let stop_duration = time_minutes - stop_start_time;
                        if stop_duration >= 1.0_f32 { // Minimum 1 minute for a deco stop
                            deco_stops.push((stop_depth, stop_duration));
                        }
                        current_stop_depth = None;
                    }
                }
                
                // Check if we're starting a new deco stop
                // Look ahead to see if we stay at this depth
                if current_stop_depth.is_none() {
                    let mut same_depth_count = 0;
                    let check_ahead = 12; // Check next 12 intervals (2 minutes)
                    
                    for j in (i + 1)..(i + 1 + check_ahead).min(results.depths.len()) {
                        if (results.depths[j] - depth).abs() < 0.5_f32 {
                            same_depth_count += 1;
                        } else {
                            break;
                        }
                    }
                    
                    // If we stay at the same depth for at least 1 minute, it's likely a deco stop
                    if same_depth_count >= 6 {
                        current_stop_depth = Some(depth);
                        stop_start_time = time_minutes;
                    }
                }
            }
        }
        
        // Handle any ongoing stop at the end
        if let Some(stop_depth) = current_stop_depth {
            let final_time = results.depths.len() as f32 * 10.0 / 60.0;
            let stop_duration = final_time - stop_start_time;
            if stop_duration >= 1.0_f32 {
                deco_stops.push((stop_depth, stop_duration));
            }
        }
        
        // Sort by depth (deepest first)
        deco_stops.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        deco_stops
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