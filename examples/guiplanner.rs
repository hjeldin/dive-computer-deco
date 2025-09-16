use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use dive_computer_deco::{
    DiveParameters,
    tissue::Tissue,
    simulate::SimulationOutputs,
    ceiling::max_ceiling_with_gf,
    m_value::calculate_m_values,
    water_vapor_pressure, FN2, FHE,
};
use std::path::Path;
use fitparser;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum TankType {
    Alu80,
    Steel15L,
    Steel12L,
    Steel10L,
    Alu63,
    Steel8L,
    Custom,
}

impl TankType {
    fn name(&self) -> &'static str {
        match self {
            TankType::Alu80 => "Aluminum 80 cuft",
            TankType::Steel15L => "Steel 15L",
            TankType::Steel12L => "Steel 12L", 
            TankType::Steel10L => "Steel 10L",
            TankType::Alu63 => "Aluminum 63 cuft",
            TankType::Steel8L => "Steel 8L",
            TankType::Custom => "Custom Tank",
        }
    }
    
    fn water_volume_liters(&self) -> f32 {
        match self {
            TankType::Alu80 => 11.1,   // 80 cuft = ~11.1L water volume
            TankType::Steel15L => 15.0,
            TankType::Steel12L => 12.0,
            TankType::Steel10L => 10.0,
            TankType::Alu63 => 8.7,    // 63 cuft = ~8.7L water volume
            TankType::Steel8L => 8.0,
            TankType::Custom => 12.0,  // Default for custom
        }
    }
    
    fn working_pressure_bar(&self) -> f32 {
        match self {
            TankType::Alu80 => 207.0,    // 3000 psi
            TankType::Steel15L => 232.0, // 15L at 232 bar
            TankType::Steel12L => 232.0, // 12L at 232 bar
            TankType::Steel10L => 232.0, // 10L at 232 bar
            TankType::Alu63 => 207.0,    // 3000 psi
            TankType::Steel8L => 200.0,  // 8L at 200 bar
            TankType::Custom => 200.0,   // Default for custom
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct AirConsumption {
    tank_type: TankType,
    custom_volume: f32,      // Water volume in liters for custom tank
    custom_pressure: f32,    // Working pressure in bar for custom tank
    starting_pressure: f32,  // Starting pressure in bar
    ending_pressure: Option<f32>, // Ending pressure in bar (for SAC calculation)
    sac_rate: f32,          // Surface Air Consumption in L/min
    calculated_sac: bool,    // Whether SAC was calculated from dive data
}

impl Default for AirConsumption {
    fn default() -> Self {
        Self {
            tank_type: TankType::Alu80,
            custom_volume: 12.0,
            custom_pressure: 200.0,
            starting_pressure: 200.0,
            ending_pressure: None,
            sac_rate: 20.0,
            calculated_sac: false,
        }
    }
}

impl AirConsumption {
    fn get_tank_volume(&self) -> f32 {
        if self.tank_type == TankType::Custom {
            self.custom_volume
        } else {
            self.tank_type.water_volume_liters()
        }
    }
    
    fn get_working_pressure(&self) -> f32 {
        if self.tank_type == TankType::Custom {
            self.custom_pressure
        } else {
            self.tank_type.working_pressure_bar()
        }
    }
    
    fn air_consumed_liters(&self, depth_m: f32, time_minutes: f32) -> f32 {
        let pressure_factor = (depth_m / 10.0) + 1.0; // Pressure at depth
        self.sac_rate * pressure_factor * time_minutes
    }
    
    fn remaining_pressure(&self, consumed_liters: f32) -> f32 {
        let tank_volume = self.get_tank_volume();
        let consumed_bar = consumed_liters / tank_volume;
        (self.starting_pressure - consumed_bar).max(0.0)
    }
    
    fn calculate_sac_from_dive(&mut self, depths: &[f32], time_intervals: &[f32], ending_pressure: f32) {
        if let Some(_) = self.ending_pressure {
            let pressure_used = self.starting_pressure - ending_pressure;
            let air_used_liters = pressure_used * self.get_tank_volume();
            
            // Calculate average depth weighted by time
            let total_pressure_time: f32 = depths.iter()
                .zip(time_intervals.iter())
                .map(|(&depth, &time)| ((depth / 10.0) + 1.0) * time)
                .sum();
            
            if total_pressure_time > 0.0 {
                self.sac_rate = air_used_liters / total_pressure_time;
                self.calculated_sac = true;
            }
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(serde::Serialize, serde::Deserialize)]
struct DivePlan {
    gf_low: f32,
    gf_high: f32,
    surface_pressure: f32,
    descent_speed: f32,
    ascent_speed: f32,
    dive_steps: Vec<DiveStep>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PlotTab {
    DiveProfile,
    TissuePressure,
    TissueHeatmap,
}

#[derive(Debug, Clone)]
struct FitActivityData {
    timestamps: Vec<f64>, // time in minutes from start
    depths: Vec<f32>,     // depth in meters
}

impl FitActivityData {
    // fn new() -> Self {
    //     Self {
    //         timestamps: Vec::new(),
    //         depths: Vec::new(),
    //     }
    // }
}

struct DivePlannerApp {
    // Dive parameters
    gf_low: f32,
    gf_high: f32,
    surface_pressure: f32,
    descent_speed: f32,
    ascent_speed: f32,
    
    // Air consumption parameters
    air_consumption: AirConsumption,
    
    // Dive profile
    dive_steps: Vec<DiveStep>,
    
    // Simulation results
    simulation_results: Option<SimulationOutputs>,
    simulation_text: String,
    air_remaining: Vec<f32>, // Air remaining at each time interval
    
    // FIT activity data
    fit_activity_data: Option<FitActivityData>,
    
    // UI state
    show_ceiling: bool,
    show_depth: bool,
    show_pressure: bool,
    show_air_remaining: bool,
    show_fit_activity: bool,
    smoothing_window: usize, // Smoothing window size for FIT activity data
    active_tab: PlotTab,
    tissue_visibility: [bool; 16], // Visibility toggle for each tissue compartment
    
    // Velocity tracking
    current_velocity: Option<f32>, // Current velocity in m/min at hover point
    hover_time: Option<f64>, // Current hover time in minutes
    hover_depth: Option<f32>, // Current hover depth in meters
}

impl Default for DivePlannerApp {
    fn default() -> Self {
        Self {
            gf_low: 0.30,
            gf_high: 0.85,
            surface_pressure: 1.0,
            descent_speed: 20.0,  // m/min
            ascent_speed: 10.0,   // m/min
            air_consumption: AirConsumption::default(),
            dive_steps: vec![DiveStep::new()],
            simulation_results: None,
            simulation_text: String::new(),
            air_remaining: Vec::new(),
            fit_activity_data: None,
            show_ceiling: true,
            show_depth: true,
            show_pressure: false,
            show_air_remaining: true,
            show_fit_activity: true,
            smoothing_window: 5, // Default smoothing window size
            active_tab: PlotTab::DiveProfile,
            tissue_visibility: [true; 16], // All tissues visible by default
            current_velocity: None,
            hover_time: None,
            hover_depth: None,
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
                
                ui.label("Descent Speed:");
                ui.add(egui::DragValue::new(&mut self.descent_speed)
                    .speed(0.5)
                    .range(5.0..=60.0)
                    .suffix(" m/min"));
                ui.end_row();
                
                ui.label("Ascent Speed:");
                ui.add(egui::DragValue::new(&mut self.ascent_speed)
                    .speed(0.5)
                    .range(3.0..=30.0)
                    .suffix(" m/min"));
                ui.end_row();
            });
        
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
        
        // Air consumption parameters
        ui.heading("🫁 Air Consumption");
        ui.add_space(4.0);
        
        egui::Grid::new("air_parameters_grid")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .show(ui, |ui| {
                ui.label("Tank Type:");
                let old_tank_type = self.air_consumption.tank_type;
                egui::ComboBox::from_id_salt("tank_type")
                    .selected_text(self.air_consumption.tank_type.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Alu80, TankType::Alu80.name());
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Steel15L, TankType::Steel15L.name());
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Steel12L, TankType::Steel12L.name());
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Steel10L, TankType::Steel10L.name());
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Alu63, TankType::Alu63.name());
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Steel8L, TankType::Steel8L.name());
                        ui.selectable_value(&mut self.air_consumption.tank_type, TankType::Custom, TankType::Custom.name());
                    });
                // Recalculate if tank type changed
                if old_tank_type != self.air_consumption.tank_type {
                    self.recalculate_air_consumption();
                }
                ui.end_row();
                
                // Show custom tank parameters if Custom is selected
                if self.air_consumption.tank_type == TankType::Custom {
                    ui.label("Volume:");
                    let old_volume = self.air_consumption.custom_volume;
                    ui.add(egui::DragValue::new(&mut self.air_consumption.custom_volume)
                        .speed(0.1)
                        .range(1.0..=30.0)
                        .suffix(" L"));
                    if old_volume != self.air_consumption.custom_volume {
                        self.recalculate_air_consumption();
                    }
                    ui.end_row();
                    
                    ui.label("Working Pressure:");
                    let old_pressure = self.air_consumption.custom_pressure;
                    ui.add(egui::DragValue::new(&mut self.air_consumption.custom_pressure)
                        .speed(1.0)
                        .range(100.0..=300.0)
                        .suffix(" bar"));
                    if old_pressure != self.air_consumption.custom_pressure {
                        self.recalculate_air_consumption();
                    }
                    ui.end_row();
                }
                
                ui.label("Starting Pressure:");
                let max_pressure = self.air_consumption.get_working_pressure();
                let old_starting_pressure = self.air_consumption.starting_pressure;
                ui.add(egui::DragValue::new(&mut self.air_consumption.starting_pressure)
                    .speed(1.0)
                    .range(50.0..=max_pressure)
                    .suffix(" bar"));
                if old_starting_pressure != self.air_consumption.starting_pressure {
                    self.recalculate_air_consumption();
                }
                ui.end_row();
                
                ui.label("SAC Rate:");
                ui.horizontal(|ui| {
                    let old_sac_rate = self.air_consumption.sac_rate;
                    ui.add(egui::DragValue::new(&mut self.air_consumption.sac_rate)
                        .speed(0.5)
                        .range(8.0..=50.0)
                        .suffix(" L/min"));
                    if old_sac_rate != self.air_consumption.sac_rate {
                        self.air_consumption.calculated_sac = false; // Mark as manually set
                        self.recalculate_air_consumption();
                    }
                    if self.air_consumption.calculated_sac {
                        ui.colored_label(egui::Color32::GREEN, "(calculated)");
                    }
                });
                ui.end_row();
                
                ui.label("Ending Pressure:");
                ui.horizontal(|ui| {
                    let mut has_ending = self.air_consumption.ending_pressure.is_some();
                    let old_has_ending = has_ending;
                    ui.checkbox(&mut has_ending, "");
                    
                    let mut ending_pressure_changed = false;
                    let mut new_ending_pressure = None;
                    
                    if has_ending && self.air_consumption.ending_pressure.is_none() {
                        self.air_consumption.ending_pressure = Some(50.0);
                        ending_pressure_changed = true;
                    } else if !has_ending {
                        self.air_consumption.ending_pressure = None;
                        self.air_consumption.calculated_sac = false;
                        ending_pressure_changed = true;
                    }
                    
                    if let Some(ref mut ending_pressure) = self.air_consumption.ending_pressure {
                        let old_ending_pressure = *ending_pressure;
                        ui.add(egui::DragValue::new(ending_pressure)
                            .speed(1.0)
                            .range(0.0..=self.air_consumption.starting_pressure)
                            .suffix(" bar"));
                        
                        if old_ending_pressure != *ending_pressure || old_has_ending != has_ending {
                            ending_pressure_changed = true;
                            new_ending_pressure = Some(*ending_pressure);
                        }
                    }
                    
                    // Handle SAC calculation outside the borrow
                    if ending_pressure_changed {
                        if let Some(ending_pressure) = new_ending_pressure {
                            if self.fit_activity_data.is_some() {
                                self.calculate_sac_from_fit_data(ending_pressure);
                            }
                        }
                        self.recalculate_air_consumption();
                    }
                });
                ui.end_row();
            });
        
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        
        // Tank info display
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 150, 255), "🗂️ Tank Info:");
            let volume = self.air_consumption.get_tank_volume();
            let pressure = self.air_consumption.get_working_pressure();
            let total_air = volume * pressure;
            ui.colored_label(egui::Color32::LIGHT_GRAY, 
                format!("{:.1}L @ {:.0}bar = {:.0}L air", volume, pressure, total_air));
        });
        
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 150, 255), "📊 Current GF:");
            let color = egui::Color32::LIGHT_GRAY;
            ui.colored_label(color, 
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
        
        // Check if simulation can be run
        let gf_invalid = self.gf_low > self.gf_high;
        
        // Large simulation button
        let button_text = if gf_invalid {
            "⚠️ Fix GF Values First"
        } else if self.dive_steps.is_empty() {
            "📝 Add Dive Steps First"
        } else {
            "🏊 Simulate Dive"
        };
        
        let button_response = ui.add_sized([ui.available_width(), 32.0], 
            egui::Button::new(button_text));
        
        if button_response.clicked() {
            self.run_simulation();
        }
        
        ui.add_space(8.0);
        
        // File operations
        ui.horizontal(|ui| {
            if ui.button("📁 Load Dive Plan").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .add_filter("All files", &["*"])
                    .set_title("Load Dive Plan")
                    .pick_file()
                {
                    self.load_dive_plan(&path);
                }
            }
            
            if ui.button("💾 Save Dive Plan").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .add_filter("All files", &["*"])
                    .set_title("Save Dive Plan")
                    .save_file()
                {
                    self.save_dive_plan(&path);
                }
            }
        });
        
        ui.add_space(8.0);
        
        // FIT file loading
        ui.horizontal(|ui| {
            if ui.button("🏊‍♀️ Load .fit Activity").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("FIT files", &["fit"])
                    .add_filter("All files", &["*"])
                    .set_title("Load FIT Activity File")
                    .pick_file()
                {
                    self.load_fit_activity(&path);
                }
            }
            
            if self.fit_activity_data.is_some() {
                if ui.button("🗑 Clear Activity").clicked() {
                    self.fit_activity_data = None;
                }
            }
        });
        
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
                ui.checkbox(&mut self.show_air_remaining, "🫁 Air Remaining");
                ui.end_row();
                if self.fit_activity_data.is_some() {
                    ui.checkbox(&mut self.show_fit_activity, "🏊‍♀️ FIT Activity");
                    ui.label(""); // Empty cell for alignment
                    ui.end_row();
                }
            });
        
        // Smoothing control for FIT data
        if self.fit_activity_data.is_some() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label("🔧 FIT Data Smoothing:");
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("Window Size:");
                ui.add(egui::Slider::new(&mut self.smoothing_window, 1..=200)
                    .suffix(" samples")
                    .text("smoothing"));
            });
            
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if ui.small_button("No Smoothing").clicked() {
                    self.smoothing_window = 1;
                }
                if ui.small_button("Light").clicked() {
                    self.smoothing_window = 3;
                }
                if ui.small_button("Medium").clicked() {
                    self.smoothing_window = 5;
                }
                if ui.small_button("Heavy").clicked() {
                    self.smoothing_window = 10;
                }
            });
        }
    }
    
    fn results_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Simulation Results");
        ui.add_space(8.0);
        
        // Show text results only if simulation has been run
        if let Some(_) = self.simulation_results {
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
        } else if self.fit_activity_data.is_none() {
            ui.colored_label(egui::Color32::GRAY, "Click 'Simulate Dive' to see results or load a FIT file to view dive profile");
            ui.add_space(8.0);
        }
        
        // Always show tabbed plots (they will handle empty states internally)
        egui::TopBottomPanel::top("plot_tabs").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, PlotTab::DiveProfile, "🌊 Dive Profile");
                ui.selectable_value(&mut self.active_tab, PlotTab::TissuePressure, "🧬 Tissue Pressure");
                ui.selectable_value(&mut self.active_tab, PlotTab::TissueHeatmap, "🔥 Tissue Heatmap");
            });
        });
        
        ui.add_space(8.0);
        
        // Show the selected tab content
        match self.active_tab {
            PlotTab::DiveProfile => self.dive_profile_plot(ui),
            PlotTab::TissuePressure => self.secondary_plot(ui),
            PlotTab::TissueHeatmap => self.tissue_heatmap_plot(ui),
        }
    }
    
    fn dive_profile_plot(&mut self, ui: &mut egui::Ui) {
        // Always show the dive profile plot
        ui.label("Dive Profile");
        
        let velocity = self.current_velocity.unwrap_or(0.0);
        let time = self.hover_time.unwrap_or(0.0);
        let depth = self.hover_depth.unwrap_or(0.0);
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 200, 255), "📊 Hover Info:");
            ui.colored_label(egui::Color32::LIGHT_GRAY, 
                format!("Time: {:.1}min, Depth: {:.1}m, Velocity: {:.2} m/min", 
                        time, depth, velocity));
            
            // Add velocity direction indicator
            let direction_text = if velocity > 0.1 {
                "⬇️ Descending"
            } else if velocity < -0.1 {
                "⬆️ Ascending"
            } else {
                "➡️ Stable"
            };
            ui.colored_label(
                if velocity.abs() > 0.1 { egui::Color32::YELLOW } else { egui::Color32::GREEN },
                direction_text
            );
        });
        
        // Calculate available height for the plot
        let available_height = ui.available_height() - 40.0; // Leave some margin
        let plot_height = available_height.max(300.0); // Minimum height of 300
        
        let plot = Plot::new("dive_profile")
            .height(plot_height)
            .legend(egui_plot::Legend::default())
            .y_axis_label("Depth (m) / Pressure (bar)")
            .x_axis_label("Time (minutes)")
            .include_y(0.0);
        
        let plot_response = plot.show(ui, |plot_ui| {
            // Show simulation results if available
            if let Some(ref results) = self.simulation_results {
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
                        Line::new("Simulated Depth", depth_points)
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
                            let (ceiling, _) = max_ceiling_with_gf(self.gf_low, self.gf_high, tissues);
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
            }
            
            // Show air remaining if available (works for both simulation and FIT data)
            if self.show_air_remaining && !self.air_remaining.is_empty() {
                // Calculate depth range for scaling air pressure
                let mut max_depth = 0.0_f32;
                let mut min_depth = 0.0_f32;
                
                // Get depth range from appropriate data source
                if let Some(ref results) = self.simulation_results {
                    if !results.depths.is_empty() {
                        max_depth = results.depths.iter().fold(0.0, |a, &b| a.max(b));
                        min_depth = results.depths.iter().fold(max_depth, |a, &b| a.min(b));
                    }
                } else if let Some(ref fit_data) = self.fit_activity_data {
                    if !fit_data.depths.is_empty() {
                        max_depth = fit_data.depths.iter().fold(0.0, |a, &b| a.max(b));
                        min_depth = fit_data.depths.iter().fold(max_depth, |a, &b| a.min(b));
                    }
                }
                
                // Calculate air pressure range for scaling
                let max_air_pressure = self.air_remaining.iter().fold(0.0_f32, |a, &b| a.max(b));
                let min_air_pressure = self.air_remaining.iter().fold(max_air_pressure, |a, &b| a.min(b));
                let air_pressure_range = max_air_pressure - min_air_pressure;
                
                // Scale air pressure to depth range (but invert it since depth is negative in the plot)
                let depth_range = max_depth - min_depth;
                let scale_factor = if air_pressure_range > 0.0 && depth_range > 0.0 {
                    depth_range / air_pressure_range
                } else {
                    1.0
                };
                
                // Create time points based on data source
                let air_points: PlotPoints = if let Some(ref _results) = self.simulation_results {
                    // For simulation data, use 10-second intervals
                    self.air_remaining
                        .iter()
                        .enumerate()
                        .map(|(i, &pressure)| {
                            let time_minutes = i as f64 * 10.0 / 60.0;
                            // Scale pressure to depth range and offset to be visible above surface
                            let scaled_pressure = (pressure - min_air_pressure) * scale_factor;
                            [time_minutes, scaled_pressure as f64] // Scaled pressure to match depth range
                        })
                        .collect()
                } else if let Some(ref fit_data) = self.fit_activity_data {
                    // For FIT data, use the actual timestamps
                    self.air_remaining
                        .iter()
                        .zip(fit_data.timestamps.iter())
                        .map(|(&pressure, &time)| {
                            // Scale pressure to depth range and offset to be visible above surface
                            let scaled_pressure = (pressure - min_air_pressure) * scale_factor;
                            [time, scaled_pressure as f64] // Scaled pressure to match depth range
                        })
                        .collect()
                } else {
                    PlotPoints::new(vec![])
                };
                
                plot_ui.line(
                    Line::new(format!("Air Remaining ({:.0}-{:.0} bar)", min_air_pressure, max_air_pressure), air_points)
                        .color(egui::Color32::LIGHT_BLUE)
                        .width(2.0)
                );
            }
            
            // Always show FIT activity data if available and enabled
            if self.show_fit_activity {
                if let Some(ref fit_data) = self.fit_activity_data {
                    // Raw FIT activity data
                    let fit_points: PlotPoints = fit_data.timestamps
                        .iter()
                        .zip(fit_data.depths.iter())
                        .map(|(&time, &depth)| {
                            [time as f64, -depth as f64] // Negative depth for proper visualization
                        })
                        .collect();
                    
                    let raw_line_name = if self.smoothing_window > 1 {
                        "FIT Activity (Raw)"
                    } else {
                        "FIT Activity"
                    };
                    
                    plot_ui.line(
                        Line::new(raw_line_name, fit_points)
                            .color(egui::Color32::from_rgb(255, 165, 0)) // Orange
                            .width(if self.smoothing_window > 1 { 1.5 } else { 2.5 })
                    );
                    
                    // Smoothed FIT activity data (only show if smoothing is enabled)
                    if self.smoothing_window > 1 {
                        let smoothed_depths = self.smooth_depth_data(&fit_data.depths, self.smoothing_window);
                        let smoothed_points: PlotPoints = fit_data.timestamps
                            .iter()
                            .zip(smoothed_depths.iter())
                            .map(|(&time, &depth)| {
                                [time as f64, -depth as f64] // Negative depth for proper visualization
                            })
                            .collect();
                        
                        plot_ui.line(
                            Line::new(format!("FIT Activity (Smoothed, {}pt)", self.smoothing_window), smoothed_points)
                                .color(egui::Color32::from_rgb(255, 69, 0)) // Red-orange, more prominent
                                .width(3.0)
                        );
                    }
                }
            }
            
            // Show a placeholder message if no data is available
            if self.simulation_results.is_none() && self.fit_activity_data.is_none() {
                let placeholder_points: PlotPoints = vec![[0.0, 0.0], [30.0, 0.0]].into();
                plot_ui.line(
                    Line::new("No data - load FIT file or run simulation", placeholder_points)
                        .color(egui::Color32::GRAY)
                );
            }
        });

        // Handle hover events to calculate and display velocity
        if let Some(pointer_pos) = plot_response.response.hover_pos() {
            let plot_pos = plot_response.transform.value_from_position(pointer_pos);
            let hover_time = plot_pos.x;
            let hover_depth = -plot_pos.y; // Convert back from negative visualization
            
            // Update hover position
            self.hover_time = Some(hover_time);
            self.hover_depth = Some(hover_depth as f32);
            
            // Calculate velocity from appropriate data source
            let mut velocity = None;
            
            // Try simulation data first if visible
            if self.show_depth && self.simulation_results.is_some() {
                velocity = self.calculate_simulation_velocity_at_time(hover_time);
            }
            
            // Try FIT data if simulation velocity not available and FIT data is visible
            if velocity.is_none() && self.show_fit_activity && self.fit_activity_data.is_some() {
                // Prefer smoothed data if available
                velocity = self.calculate_fit_velocity_at_time(hover_time, self.smoothing_window > 1);
                
                // Fall back to raw FIT data if smoothed not available
                if velocity.is_none() {
                    velocity = self.calculate_fit_velocity_at_time(hover_time, false);
                }
            }
            
            self.current_velocity = velocity;
        } else {
            // Clear hover information when not hovering
            self.current_velocity = None;
            self.hover_time = None;
            self.hover_depth = None;
        }
    }
    
    fn secondary_plot(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Tissue Pressure vs Ambient Pressure");
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
            .x_axis_label("Ambient Pressure (bar)")
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
                                let ambient_pressure = self.surface_pressure + (depth / 10.0); // Convert depth to pressure
                                let tissue_pressure = tissues[tissue_idx].load_n2;
                                [ambient_pressure as f64, tissue_pressure as f64]
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
                                [ambient_pressure as f64, m_value as f64]
                            })
                            .collect();
                        
                        plot_ui.line(
                            Line::new(format!("M-Value {}", tissue_idx + 1), m_value_points)
                                .color(colors[tissue_idx])
                                .width(1.0)
                                .style(egui_plot::LineStyle::Dashed { length: 3.0 })
                        );
                    }

                    // Add bisector line (x = y) for first quadrant reference
                    let max_pressure = results.depths.iter()
                        .map(|&depth| self.surface_pressure + (depth / 10.0))
                        .fold(self.surface_pressure, f32::max);
                    let bisector_points: PlotPoints = vec![
                        [self.surface_pressure as f64, self.surface_pressure as f64],
                        [max_pressure as f64, max_pressure as f64]
                    ].into();
                    
                    plot_ui.line(
                        Line::new("Bisector (Ambient = Tissue)", bisector_points)
                            .color(egui::Color32::WHITE)
                            .width(1.5)
                            .style(egui_plot::LineStyle::Dashed { length: 2.0 })
                    );

                    // Plot gradient factor lines for each tissue compartment
                    for tissue_idx in 0..16 {
                        if !self.tissue_visibility[tissue_idx] {
                            continue; // Skip if tissue is not visible
                        }
                        
                        let gf_low_points: PlotPoints = results.depths
                            .iter()
                            .map(|&depth| {
                                let ambient_pressure = self.surface_pressure + (depth / 10.0);
                                let m_value = calculate_m_values(ambient_pressure, tissue_idx);
                                
                                // GF low line: interpolate between bisector and M-value
                                // Start point: bisector (ambient = tissue)
                                // End point: M-value 
                                // Interpolation: bisector + gf_low * (m_value - bisector)
                                let bisector_value = ambient_pressure;
                                let gf_low_value = bisector_value + self.gf_low * (m_value - bisector_value);
                                
                                [ambient_pressure as f64, gf_low_value as f64]
                            })
                            .collect();
                        
                        plot_ui.line(
                            Line::new(format!("GF Low {} ({}%)", tissue_idx + 1, (self.gf_low * 100.0) as u8), gf_low_points)
                                .color(colors[tissue_idx])
                                .width(1.0)
                                .style(egui_plot::LineStyle::Dotted { spacing: 2.0 })
                        );

                        let gf_high_points: PlotPoints = results.depths
                            .iter()
                            .map(|&depth| {
                                let ambient_pressure = self.surface_pressure + (depth / 10.0);
                                let m_value = calculate_m_values(ambient_pressure, tissue_idx);
                                
                                // GF high line: interpolate between bisector and M-value
                                // Start point: bisector (ambient = tissue)
                                // End point: M-value
                                // Interpolation: bisector + gf_high * (m_value - bisector)
                                let bisector_value = ambient_pressure;
                                let gf_high_value = bisector_value + self.gf_high * (m_value - bisector_value);
                                
                                [ambient_pressure as f64, gf_high_value as f64]
                            })
                            .collect();
                        
                        plot_ui.line(
                            Line::new(format!("GF High {} ({}%)", tissue_idx + 1, (self.gf_high * 100.0) as u8), gf_high_points)
                                .color(colors[tissue_idx])
                                .width(1.2)
                                .style(egui_plot::LineStyle::Dotted { spacing: 4.0 })
                        );
                    }

                    // Add gradient factor profile line that connects GF High at shallow to GF Low at deep depths
                    if self.tissue_visibility.iter().any(|&visible| visible) {
                        // Find the pressure range
                        let min_pressure = self.surface_pressure;
                        let max_pressure = results.depths.iter()
                            .map(|&depth| self.surface_pressure + (depth / 10.0))
                            .fold(self.surface_pressure, f32::max);
                        
                        // Use the first visible tissue's M-value for calculating start and end points
                        let first_visible_tissue = self.tissue_visibility.iter()
                            .position(|&visible| visible)
                            .unwrap_or(0);
                        
                        // Calculate start point (shallow, GF High)
                        let start_m_value = calculate_m_values(min_pressure, first_visible_tissue);
                        let start_bisector = min_pressure;
                        let start_y = start_bisector + self.gf_high * (start_m_value - start_bisector);
                        
                        // Calculate end point (deep, GF Low)
                        let end_m_value = calculate_m_values(max_pressure, first_visible_tissue);
                        let end_bisector = max_pressure;
                        let end_y = end_bisector + self.gf_low * (end_m_value - end_bisector);
                        
                        // Create a straight line from start to end
                        let gf_profile_points: PlotPoints = vec![
                            [min_pressure as f64, start_y as f64],
                            [max_pressure as f64, end_y as f64]
                        ].into();
                        
                        plot_ui.line(
                            Line::new(format!("GF Profile ({}% → {}%)", (self.gf_high * 100.0) as u8, (self.gf_low * 100.0) as u8), gf_profile_points)
                                .color(egui::Color32::YELLOW)
                                .width(2.0)
                        );
                    }
                }
            } else {
                // Show placeholder when no simulation results
                let placeholder_points: PlotPoints = vec![[1.0, 1.0], [4.0, 1.0]].into();
                plot_ui.line(
                    Line::new("No data - run simulation", placeholder_points)
                        .color(egui::Color32::GRAY)
                );
            }
        });
    }
    
    fn tissue_heatmap_plot(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Tissue Loading Heatmap (% of M-Value)");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(egui::Color32::GRAY, "🔥 Hot = High Loading");
            });
        });
        
        ui.add_space(8.0);
        
        // Calculate available height for the plot
        let available_height = ui.available_height() - 40.0;
        let plot_height = available_height.max(400.0); // Minimum height for heatmap
        
        if let Some(ref results) = self.simulation_results {
            if !results.tissues_per_interval.is_empty() {
                self.render_tissue_heatmap(ui, results, plot_height);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::GRAY, "No tissue data available - run simulation");
                });
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.colored_label(egui::Color32::GRAY, "No simulation data - click 'Simulate Dive' to generate heatmap");
            });
        }
    }
    
    fn render_tissue_heatmap(&self, ui: &mut egui::Ui, results: &SimulationOutputs, plot_height: f32) {
        use egui::*;
        
        // Calculate time points (x-axis)
        let time_points: Vec<f64> = (0..results.tissues_per_interval.len())
            .map(|i| i as f64 * 10.0 / 60.0) // Convert 10-second intervals to minutes
            .collect();
        
        if time_points.is_empty() {
            return;
        }
        
        let _max_time = time_points.last().unwrap_or(&0.0);
        let num_tissues = 16;
        
        // Calculate tissue loading percentages for each time point
        let heatmap_data: Vec<Vec<f32>> = Vec::new();
        
        // for (time_idx, tissues) in results.tissues_per_interval.iter().enumerate() {
        //     let mut tissue_loadings = Vec::new();
            
        //     // Get current depth for this time point to calculate proper gradient factor
        //     let current_depth = results.depths.get(time_idx).unwrap_or(&0.0);
        //     let ambient_pressure = self.surface_pressure + (current_depth / 10.0);
            
        //     for tissue_idx in 0..num_tissues {
        //         let tissue = &tissues[tissue_idx];
        //         let m_value = calculate_m_values(ambient_pressure, tissue_idx);
        //         let tissue_pressure = tissue.load_n2 + tissue.load_he;
                
        //         // Calculate the first stop pressure for gradient factor interpolation
        //         let first_stop_pressure = dive_computer_deco::ceiling::first_stop_pressure(tissues, self.surface_pressure);
                
        //         // Interpolate gradient factor for current conditions
        //         let current_gf = if (first_stop_pressure - self.surface_pressure).abs() < 1e-6 {
        //             self.gf_high // At surface or no decompression needed
        //         } else {
        //             let fraction = (tissue_pressure - self.surface_pressure) 
        //                          / (first_stop_pressure - self.surface_pressure);
        //             let fraction = fraction.clamp(0.0, 1.0);
        //             self.gf_low + (self.gf_high - self.gf_low) * fraction
        //         };
                
        //         // Calculate loading percentage using gradient factor
        //         let loading_percent = if m_value > 0.0 {
        //             // Calculate the allowed pressure at current depth with current GF
        //             let allowed_overpressure = current_gf * (m_value - ambient_pressure);
        //             let allowed_pressure = ambient_pressure + allowed_overpressure;
                    
        //             // Calculate percentage of allowed pressure
        //             (tissue_pressure / allowed_pressure * 100.0).min(100.0).max(0.0)
        //         } else {
        //             0.0
        //         };
        //         tissue_loadings.push(loading_percent);
        //     }
        //     heatmap_data.push(tissue_loadings);
        // }
        
        // Create a custom widget for the heatmap
        let heatmap_response = ui.allocate_response(
            Vec2::new(ui.available_width(), plot_height),
            Sense::hover()
        );
        
        if heatmap_response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        
        let painter = ui.painter_at(heatmap_response.rect);
        
        // Draw the heatmap
        let rect = heatmap_response.rect;
        let cell_width = rect.width() / time_points.len() as f32;
        let cell_height = rect.height() / num_tissues as f32;
        
        // Draw heatmap cells
        for (time_idx, tissue_loadings) in heatmap_data.iter().enumerate() {
            for (tissue_idx, &loading) in tissue_loadings.iter().enumerate() {
                let x = rect.min.x + time_idx as f32 * cell_width;
                let y = rect.min.y + tissue_idx as f32 * cell_height;
                
                let cell_rect = Rect::from_min_size(
                    Pos2::new(x, y),
                    Vec2::new(cell_width, cell_height)
                );
                
                // Color mapping: blue (low) -> green -> yellow -> red (high)
                let color = self.loading_to_color(loading);
                painter.rect_filled(cell_rect, 0.0, color);
            }
        }
        
        // Draw grid lines
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GRAY), egui::StrokeKind::Middle);
        
        // Draw tissue compartment labels (y-axis)
        for tissue_idx in 0..num_tissues {
            let y = rect.min.y + (tissue_idx as f32 + 0.5) * cell_height;
            let label_pos = Pos2::new(rect.min.x - 5.0, y);
            
            painter.text(
                label_pos,
                Align2::RIGHT_CENTER,
                format!("T{}", tissue_idx + 1),
                FontId::proportional(10.0),
                Color32::WHITE
            );
        }
        
        // Draw time labels (x-axis) - show every 10th point to avoid crowding
        let time_step = (time_points.len() / 10).max(1);
        for (i, &time) in time_points.iter().enumerate() {
            if i % time_step == 0 {
                let x = rect.min.x + i as f32 * cell_width + cell_width * 0.5;
                let label_pos = Pos2::new(x, rect.max.y + 15.0);
                
                painter.text(
                    label_pos,
                    Align2::CENTER_TOP,
                    format!("{:.0}", time),
                    FontId::proportional(10.0),
                    Color32::WHITE
                );
            }
        }
        
        // Add axis labels
        painter.text(
            Pos2::new(rect.center().x, rect.max.y + 35.0),
            Align2::CENTER_TOP,
            "Time (minutes)",
            FontId::proportional(12.0),
            Color32::WHITE
        );
        
        // Rotate tissue label for y-axis
        painter.text(
            Pos2::new(rect.min.x - 40.0, rect.center().y),
            Align2::CENTER_CENTER,
            "Tissue Compartment",
            FontId::proportional(12.0),
            Color32::WHITE
        );
        
        // Draw color scale legend
        self.draw_color_legend(ui, &painter, rect);
        
        // Handle immediate mouse tracking for real-time info display
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if heatmap_response.rect.contains(pointer_pos) {
                let relative_pos = pointer_pos - rect.min;
                let time_idx = ((relative_pos.x / cell_width) as usize).min(time_points.len() - 1);
                let tissue_idx = ((relative_pos.y / cell_height) as usize).min(num_tissues - 1);
                
                if time_idx < heatmap_data.len() && tissue_idx < heatmap_data[time_idx].len() {
                    let loading = heatmap_data[time_idx][tissue_idx];
                    let time = time_points[time_idx];
                    
                    // Show immediate info overlay
                    let info_text = format!(
                        "{:.1}mins, T{}, GF: {:.1}%",
                        time, tissue_idx + 1, loading
                    );
                    
                    // Draw info box near cursor
                    let info_pos = pointer_pos + egui::Vec2::new(10.0, -20.0);
                    let info_rect = egui::Rect::from_min_size(info_pos, egui::Vec2::new(120.0, 20.0));
                    
                    // Draw background
                    painter.rect_filled(
                        info_rect,
                        4.0,
                        egui::Color32::from_black_alpha(200)
                    );
                    painter.rect_stroke(
                        info_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                        egui::StrokeKind::Middle
                    );
                    
                    // Draw text
                    painter.text(
                        info_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        info_text,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE
                    );
                }
            }
        }
    }
    
    fn loading_to_color(&self, loading_percent: f32) -> egui::Color32 {
        // Clamp loading between 0 and 100
        let loading = loading_percent.clamp(0.0, 100.0);
        
        // Create a color gradient from blue (0%) to red (100%)
        if loading < 25.0 {
            // Blue to cyan (0-25%)
            let t = loading / 25.0;
            egui::Color32::from_rgb(
                (0.0 * (1.0 - t) + 0.0 * t) as u8,
                (0.0 * (1.0 - t) + 255.0 * t) as u8,
                255
            )
        } else if loading < 50.0 {
            // Cyan to green (25-50%)
            let t = (loading - 25.0) / 25.0;
            egui::Color32::from_rgb(
                0,
                255,
                (255.0 * (1.0 - t) + 0.0 * t) as u8
            )
        } else if loading < 75.0 {
            // Green to yellow (50-75%)
            let t = (loading - 50.0) / 25.0;
            egui::Color32::from_rgb(
                (0.0 * (1.0 - t) + 255.0 * t) as u8,
                255,
                0
            )
        } else {
            // Yellow to red (75-100%)
            let t = (loading - 75.0) / 25.0;
            egui::Color32::from_rgb(
                255,
                (255.0 * (1.0 - t) + 0.0 * t) as u8,
                0
            )
        }
    }
    
    fn draw_color_legend(&self, _ui: &mut egui::Ui, painter: &egui::Painter, heatmap_rect: egui::Rect) {
        // Draw color scale legend on the right side
        let legend_width = 20.0;
        let legend_height = 200.0;
        let legend_x = heatmap_rect.max.x + 20.0;
        let legend_y = heatmap_rect.min.y + (heatmap_rect.height() - legend_height) * 0.5;
        
        let legend_rect = egui::Rect::from_min_size(
            egui::Pos2::new(legend_x, legend_y),
            egui::Vec2::new(legend_width, legend_height)
        );
        
        // Draw gradient bars
        let num_segments = 100;
        let segment_height = legend_height / num_segments as f32;
        
        for i in 0..num_segments {
            let loading = (i as f32 / num_segments as f32) * 100.0;
            let color = self.loading_to_color(loading);
            
            let y = legend_rect.max.y - (i as f32 + 1.0) * segment_height;
            let segment_rect = egui::Rect::from_min_size(
                egui::Pos2::new(legend_rect.min.x, y),
                egui::Vec2::new(legend_width, segment_height)
            );
            
            painter.rect_filled(segment_rect, 0.0, color);
        }
        
        // Draw legend border
        painter.rect_stroke(legend_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);

        // Draw legend labels
        let legend_labels = [
            (0.0, "0%"),
            (25.0, "25%"),
            (50.0, "50%"),
            (75.0, "75%"),
            (100.0, "100%"),
        ];
        
        for (percent, label) in legend_labels.iter() {
            let y = legend_rect.max.y - (percent / 100.0) * legend_height;
            let label_pos = egui::Pos2::new(legend_rect.max.x + 5.0, y);
            
            painter.text(
                label_pos,
                egui::Align2::LEFT_CENTER,
                *label,
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE
            );
        }
        
        // Legend title
        painter.text(
            egui::Pos2::new(legend_rect.center().x, legend_rect.min.y - 15.0),
            egui::Align2::CENTER_BOTTOM,
            "% M-Value",
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE
        );
    }
    
    fn calculate_air_consumption(&mut self, results: &SimulationOutputs) {
        self.air_remaining.clear();
        
        if results.depths.is_empty() {
            return;
        }
        
        let mut total_consumed_liters = 0.0;
        
        for &depth in results.depths.iter() {
            // Calculate time interval (10 seconds = 1/6 minute)
            let time_interval_minutes = 10.0 / 60.0;
            
            // Calculate air consumed during this interval
            let air_consumed_this_interval = self.air_consumption.air_consumed_liters(depth, time_interval_minutes);
            total_consumed_liters += air_consumed_this_interval;
            
            // Calculate remaining pressure
            let remaining_pressure = self.air_consumption.remaining_pressure(total_consumed_liters);
            self.air_remaining.push(remaining_pressure);
        }
    }
    
    fn calculate_air_consumption_for_fit(&mut self) {
        self.air_remaining.clear();
        
        if let Some(ref fit_data) = self.fit_activity_data {
            if fit_data.depths.is_empty() || fit_data.timestamps.is_empty() {
                return;
            }
            
            let mut total_consumed_liters = 0.0;
            let mut prev_time = 0.0_f64;
            
            for (i, (&depth, &time)) in fit_data.depths.iter().zip(fit_data.timestamps.iter()).enumerate() {
                // Calculate time interval
                let time_interval_minutes = if i == 0 {
                    0.0 // No consumption for the first data point
                } else {
                    (time - prev_time) as f32
                };
                
                // Calculate air consumed during this interval
                let air_consumed_this_interval = self.air_consumption.air_consumed_liters(depth, time_interval_minutes);
                total_consumed_liters += air_consumed_this_interval;
                
                // Calculate remaining pressure
                let remaining_pressure = self.air_consumption.remaining_pressure(total_consumed_liters);
                self.air_remaining.push(remaining_pressure);
                
                prev_time = time;
            }
        }
    }
    
    fn calculate_sac_from_fit_data(&mut self, ending_pressure: f32) {
        if let Some(ref fit_data) = self.fit_activity_data {
            if fit_data.depths.is_empty() || fit_data.timestamps.is_empty() {
                return;
            }
            
            let pressure_used = self.air_consumption.starting_pressure - ending_pressure;
            let air_used_liters = pressure_used * self.air_consumption.get_tank_volume();
            
            // Calculate total pressure-time (depth-adjusted time)
            let mut total_pressure_time = 0.0_f32;
            let mut prev_time = 0.0_f64;
            
            for (i, (&depth, &time)) in fit_data.depths.iter().zip(fit_data.timestamps.iter()).enumerate() {
                if i > 0 {
                    let time_interval = (time - prev_time) as f32;
                    let pressure_factor = (depth / 10.0) + 1.0;
                    total_pressure_time += pressure_factor * time_interval;
                }
                prev_time = time;
            }
            
            if total_pressure_time > 0.0 {
                self.air_consumption.sac_rate = air_used_liters / total_pressure_time;
                self.air_consumption.calculated_sac = true;
                
                // Recalculate air consumption with new SAC rate
                self.calculate_air_consumption_for_fit();
            }
        }
    }
    
    fn recalculate_air_consumption(&mut self) {
        // Update text results if we have either simulation results or FIT data
        if self.simulation_results.is_some() {
            // Recalculate for simulation data
            if let Some(ref results) = self.simulation_results.clone() {
                self.calculate_air_consumption(&results);
            }
        } else if self.fit_activity_data.is_some() {
            // Recalculate for FIT data
            self.calculate_air_consumption_for_fit();
            
            // Update the simulation text with new air consumption results
            if let Some(ref fit_data) = self.fit_activity_data {
                if !self.air_remaining.is_empty() {
                    let total_dive_time = *fit_data.timestamps.last().unwrap_or(&0.0);
                    let final_air_pressure = self.air_remaining.last().unwrap_or(&0.0);
                    let total_air_consumed = self.air_consumption.starting_pressure - final_air_pressure;
                    let total_air_consumed_liters = total_air_consumed * self.air_consumption.get_tank_volume();
                    let average_consumption = if total_dive_time > 0.0 { total_air_consumed_liters / (total_dive_time as f32) } else { 0.0 };
                    
                    let mut result_text = format!("Loaded FIT activity with {} data points\n", fit_data.depths.len());
                    result_text.push_str(&format!("\n=== AIR CONSUMPTION (FIT Activity) ===\n"));
                    result_text.push_str(&format!("Tank: {} ({:.1}L @ {:.0} bar)\n", 
                        self.air_consumption.tank_type.name(),
                        self.air_consumption.get_tank_volume(),
                        self.air_consumption.get_working_pressure()));
                    result_text.push_str(&format!("Starting Pressure: {:.0} bar\n", self.air_consumption.starting_pressure));
                    result_text.push_str(&format!("Final Tank Pressure: {:.0} bar\n", final_air_pressure));
                    result_text.push_str(&format!("Total Air Consumed: {:.0} bar ({:.0} L)\n", total_air_consumed, total_air_consumed_liters));
                    result_text.push_str(&format!("Average Consumption Rate: {:.1} L/min\n", average_consumption));
                    result_text.push_str(&format!("SAC Rate: {:.1} L/min{}\n", 
                        self.air_consumption.sac_rate,
                        if self.air_consumption.calculated_sac { " (calculated)" } else { "" }));
                    result_text.push_str(&format!("Total Dive Time: {:.1} minutes\n", total_dive_time));
                    
                    self.simulation_text = result_text;
                }
            }
        }
    }
    
    fn get_responsible_tissues(&self, tissues: &[Tissue; 16]) -> Vec<(usize, u32, f32)> {
        let mut responsible_tissues = Vec::new();
        
        for i in 0..16 {
            let tissue_ceiling = dive_computer_deco::ceiling::ceiling_with_gf(
                self.gf_low,
                self.gf_high, 
                &tissues[i], 
                i, 
                self.surface_pressure,
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
        dive_params.descent_speed = self.descent_speed / 60.0; // Convert m/min to m/s
        dive_params.ascent_speed = self.ascent_speed / 60.0;   // Convert m/min to m/s
        
        let mut dive_text = String::new();
        dive_text.push_str(&format!("=== DIVE PLAN ===\n"));
        dive_text.push_str(&format!("GF Low/High: {:.0}%/{:.0}%\n", 
            self.gf_low * 100.0, self.gf_high * 100.0));
        dive_text.push_str(&format!("Surface Pressure: {:.2} bar\n", self.surface_pressure));
        dive_text.push_str(&format!("Descent Speed: {:.1} m/min\n", self.descent_speed));
        dive_text.push_str(&format!("Ascent Speed: {:.1} m/min\n\n", self.ascent_speed));
        
        // Add air consumption info
        dive_text.push_str(&format!("=== AIR CONSUMPTION ===\n"));
        dive_text.push_str(&format!("Tank: {} ({:.1}L @ {:.0} bar)\n", 
            self.air_consumption.tank_type.name(),
            self.air_consumption.get_tank_volume(),
            self.air_consumption.get_working_pressure()));
        dive_text.push_str(&format!("Starting Pressure: {:.0} bar\n", self.air_consumption.starting_pressure));
        dive_text.push_str(&format!("SAC Rate: {:.1} L/min{}\n", 
            self.air_consumption.sac_rate,
            if self.air_consumption.calculated_sac { " (calculated)" } else { "" }));
        if let Some(ending_pressure) = self.air_consumption.ending_pressure {
            dive_text.push_str(&format!("Target Ending Pressure: {:.0} bar\n", ending_pressure));
        }
        dive_text.push_str("\n");
        
        let mut total_runtime = 0.0;

        // Create a continuous simulation for all dive steps
        let all_results = self.simulate_dive_steps(&mut dive_params, &mut tissues, temperature);
        
        // Calculate air consumption for each time interval
        self.calculate_air_consumption(&all_results);
        
        // Calculate SAC rate if ending pressure is provided
        if let Some(ending_pressure) = self.air_consumption.ending_pressure {
            let time_intervals: Vec<f32> = vec![10.0 / 60.0; all_results.depths.len()]; // 10-second intervals in minutes
            self.air_consumption.calculate_sac_from_dive(&all_results.depths, &time_intervals, ending_pressure);
            
            // Recalculate air consumption with new SAC rate
            self.calculate_air_consumption(&all_results);
        }
        
        for (step_num, step) in self.dive_steps.iter().enumerate() {
            dive_text.push_str(&format!("Step {}: {}m for {:.1} minutes\n", 
                step_num + 1, step.depth, step.duration));
            total_runtime += step.duration;
        }
        
        // Calculate final ceiling and decompression status
        let (final_ceiling, controlling_tissue) = max_ceiling_with_gf(self.gf_low, self.gf_high, &tissues);
        
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
        
        // Add air consumption results
        if !self.air_remaining.is_empty() {
            let final_air_pressure = self.air_remaining.last().unwrap_or(&0.0);
            let total_air_consumed = self.air_consumption.starting_pressure - final_air_pressure;
            let total_air_consumed_liters = total_air_consumed * self.air_consumption.get_tank_volume();
            let average_consumption = if total_dive_time > 0.0 { total_air_consumed_liters / total_dive_time } else { 0.0 };
            
            dive_text.push_str(&format!("\n=== AIR CONSUMPTION RESULTS ===\n"));
            dive_text.push_str(&format!("Final Tank Pressure: {:.0} bar\n", final_air_pressure));
            dive_text.push_str(&format!("Total Air Consumed: {:.0} bar ({:.0} L)\n", total_air_consumed, total_air_consumed_liters));
            dive_text.push_str(&format!("Average Consumption Rate: {:.1} L/min\n", average_consumption));
            
            if self.air_consumption.calculated_sac {
                dive_text.push_str(&format!("Calculated SAC Rate: {:.1} L/min\n", self.air_consumption.sac_rate));
            }
        }
        
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
    
    fn load_dive_plan(&mut self, path: &Path) {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                match serde_json::from_str::<DivePlan>(&contents) {
                    Ok(plan) => {
                        self.gf_low = plan.gf_low;
                        self.gf_high = plan.gf_high;
                        self.surface_pressure = plan.surface_pressure;
                        self.descent_speed = plan.descent_speed;
                        self.ascent_speed = plan.ascent_speed;
                        self.dive_steps = plan.dive_steps;
                        // Clear simulation results when loading a new plan
                        self.simulation_results = None;
                        self.simulation_text = String::new();
                    }
                    Err(e) => {
                        self.simulation_text = format!("Error parsing dive plan: {}", e);
                    }
                }
            }
            Err(e) => {
                self.simulation_text = format!("Error reading file: {}", e);
            }
        }
    }
    
    fn save_dive_plan(&self, path: &Path) {
        let plan = DivePlan {
            gf_low: self.gf_low,
            gf_high: self.gf_high,
            surface_pressure: self.surface_pressure,
            descent_speed: self.descent_speed,
            ascent_speed: self.ascent_speed,
            dive_steps: self.dive_steps.clone(),
        };
        
        match serde_json::to_string_pretty(&plan) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    eprintln!("Error saving dive plan: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error serializing dive plan: {}", e);
            }
        }
    }
    
    fn load_fit_activity(&mut self, path: &Path) {
        use std::fs::File;
        use std::io::BufReader;
        
        match File::open(path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                match fitparser::from_reader(&mut reader) {
                    Ok(fit_file) => {
                        self.process_fit_file(fit_file);
                    }
                    Err(e) => {
                        self.simulation_text = format!("Error parsing FIT file: {}", e);
                    }
                }
            }
            Err(e) => {
                self.simulation_text = format!("Error reading FIT file: {}", e);
            }
        }
    }
    
    fn process_fit_file(&mut self, fit_file: Vec<fitparser::FitDataRecord>) {
        let mut timestamps = Vec::new();
        let mut depths = Vec::new();
        let mut start_timestamp: Option<i64> = None;
        
        // Extract record messages which contain the depth and timestamp data
        for data in fit_file.into_iter() {
            let fields = data.fields();
            let mut time_minutes: Option<f64> = None;
            let mut depth_value: Option<f32> = None;
            
            // Extract timestamp and depth from fields
            for field in fields {
                match field.name() {
                    "timestamp" => {
                        if let fitparser::Value::Timestamp(ts) = field.value() {
                            let timestamp_secs = ts.timestamp();
                            if start_timestamp.is_none() {
                                start_timestamp = Some(timestamp_secs);
                                time_minutes = Some(0.0);
                            } else if let Some(start) = start_timestamp {
                                let duration_secs = timestamp_secs - start;
                                time_minutes = Some(duration_secs as f64 / 60.0);
                            }
                        }
                    }
                    "depth" | "enhanced_depth" | "depth_m" | "water_depth" => {
                        if let Some(depth) = self.extract_depth_value(field.value()) {
                            depth_value = Some(depth);
                        }
                    }
                    _ => {}
                }
            }
            
            // Add the data point if we have both time and depth
            if let (Some(time), Some(depth)) = (time_minutes, depth_value) {
                timestamps.push(time);
                depths.push(depth);
            }
        }
        
        if !timestamps.is_empty() && !depths.is_empty() {
            let data_points = timestamps.len();
            let total_dive_time = *timestamps.last().unwrap_or(&0.0);
            
            self.fit_activity_data = Some(FitActivityData {
                timestamps,
                depths,
            });
            
            // Calculate air consumption for FIT activity
            self.calculate_air_consumption_for_fit();
            
            let mut result_text = format!("Loaded FIT activity with {} data points\n", data_points);
            
            // Add air consumption results if available
            if !self.air_remaining.is_empty() {
                let final_air_pressure = self.air_remaining.last().unwrap_or(&0.0);
                let total_air_consumed = self.air_consumption.starting_pressure - final_air_pressure;
                let total_air_consumed_liters = total_air_consumed * self.air_consumption.get_tank_volume();
                let average_consumption = if total_dive_time > 0.0 { total_air_consumed_liters / (total_dive_time as f32) } else { 0.0 };
                
                result_text.push_str(&format!("\n=== AIR CONSUMPTION (FIT Activity) ===\n"));
                result_text.push_str(&format!("Tank: {} ({:.1}L @ {:.0} bar)\n", 
                    self.air_consumption.tank_type.name(),
                    self.air_consumption.get_tank_volume(),
                    self.air_consumption.get_working_pressure()));
                result_text.push_str(&format!("Starting Pressure: {:.0} bar\n", self.air_consumption.starting_pressure));
                result_text.push_str(&format!("Final Tank Pressure: {:.0} bar\n", final_air_pressure));
                result_text.push_str(&format!("Total Air Consumed: {:.0} bar ({:.0} L)\n", total_air_consumed, total_air_consumed_liters));
                result_text.push_str(&format!("Average Consumption Rate: {:.1} L/min\n", average_consumption));
                result_text.push_str(&format!("SAC Rate: {:.1} L/min{}\n", 
                    self.air_consumption.sac_rate,
                    if self.air_consumption.calculated_sac { " (calculated)" } else { "" }));
                result_text.push_str(&format!("Total Dive Time: {:.1} minutes\n", total_dive_time));
                
                // Calculate SAC rate if ending pressure is provided
                if let Some(ending_pressure) = self.air_consumption.ending_pressure {
                    self.calculate_sac_from_fit_data(ending_pressure);
                    result_text.push_str(&format!("Calculated SAC from ending pressure: {:.1} L/min\n", self.air_consumption.sac_rate));
                }
            }
            
            self.simulation_text = result_text;
        } else {
            self.simulation_text = "No valid depth data found in FIT file".to_string();
        }
    }
    
    fn extract_depth_value(&self, value: &fitparser::Value) -> Option<f32> {
        match value {
            fitparser::Value::Float32(v) => Some(*v),
            fitparser::Value::Float64(v) => Some(*v as f32),
            fitparser::Value::SInt8(v) => Some(*v as f32),
            fitparser::Value::UInt8(v) => Some(*v as f32),
            fitparser::Value::SInt16(v) => Some(*v as f32),
            fitparser::Value::UInt16(v) => Some(*v as f32),
            fitparser::Value::SInt32(v) => Some(*v as f32),
            fitparser::Value::UInt32(v) => Some(*v as f32),
            fitparser::Value::SInt64(v) => Some(*v as f32),
            fitparser::Value::UInt64(v) => Some(*v as f32),
            _ => None,
        }
    }
    
    fn smooth_depth_data(&self, depths: &[f32], window_size: usize) -> Vec<f32> {
        if depths.is_empty() || window_size <= 1 {
            return depths.to_vec();
        }
        
        let mut smoothed = Vec::with_capacity(depths.len());
        let half_window = window_size / 2;
        
        for i in 0..depths.len() {
            // Preserve the exact start and end values to match the raw curve
            if i == 0 || i == depths.len() - 1 {
                smoothed.push(depths[i]);
            } else {
                let start = if i >= half_window { i - half_window } else { 0 };
                let end = (i + half_window + 1).min(depths.len());
                
                let sum: f32 = depths[start..end].iter().sum();
                let count = end - start;
                let average = sum / count as f32;
                
                smoothed.push(average);
            }
        }
        
        smoothed
    }

    fn calculate_velocity_at_time(&self, time_minutes: f64, depths: &[f32], timestamps: &[f64]) -> Option<f32> {
        if depths.len() < 2 || timestamps.len() != depths.len() {
            return None;
        }

        // Find the closest data points
        let mut closest_idx = 0;
        let mut min_diff = f64::INFINITY;
        
        for (i, &timestamp) in timestamps.iter().enumerate() {
            let diff = (timestamp - time_minutes).abs();
            if diff < min_diff {
                min_diff = diff;
                closest_idx = i;
            }
        }

        // Calculate velocity using central difference when possible
        let velocity = if closest_idx == 0 {
            // Forward difference at the start
            if depths.len() > 1 {
                let dt = timestamps[1] - timestamps[0];
                let dd = depths[1] - depths[0];
                if dt > 0.0 { Some(dd / dt as f32) } else { None }
            } else {
                None
            }
        } else if closest_idx == depths.len() - 1 {
            // Backward difference at the end
            let dt = timestamps[closest_idx] - timestamps[closest_idx - 1];
            let dd = depths[closest_idx] - depths[closest_idx - 1];
            if dt > 0.0 { Some(dd / dt as f32) } else { None }
        } else {
            // Central difference in the middle
            let dt = timestamps[closest_idx + 1] - timestamps[closest_idx - 1];
            let dd = depths[closest_idx + 1] - depths[closest_idx - 1];
            if dt > 0.0 { Some(dd / dt as f32) } else { None }
        };

        velocity
    }

    fn calculate_simulation_velocity_at_time(&self, time_minutes: f64) -> Option<f32> {
        if let Some(ref results) = self.simulation_results {
            if !results.depths.is_empty() {
                let timestamps: Vec<f64> = (0..results.depths.len())
                    .map(|i| i as f64 * 10.0 / 60.0) // 10-second intervals converted to minutes
                    .collect();
                return self.calculate_velocity_at_time(time_minutes, &results.depths, &timestamps);
            }
        }
        None
    }

    fn calculate_fit_velocity_at_time(&self, time_minutes: f64, use_smoothed: bool) -> Option<f32> {
        if let Some(ref fit_data) = self.fit_activity_data {
            let depths = if use_smoothed && self.smoothing_window > 1 {
                self.smooth_depth_data(&fit_data.depths, self.smoothing_window)
            } else {
                fit_data.depths.clone()
            };
            return self.calculate_velocity_at_time(time_minutes, &depths, &fit_data.timestamps);
        }
        None
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