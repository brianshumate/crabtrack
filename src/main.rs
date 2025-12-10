mod config;
mod database;
mod observer;
mod pass_prediction;
mod radio;
mod satellite;
mod ui;

use database::{Database, SatelliteDetails};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use sgp4::{Constants, Elements, MinutesSinceEpoch};
use std::fs;

use config::Config;
use observer::Observer;
use pass_prediction::{calculate_gmst, calculate_look_angles, SatellitePass};
use radio::{calculate_doppler_shift, evaluate_communication_window};
use satellite::{Satellite, SatellitePosition};

/// Application view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    SatelliteConfig,
}

/// Editing mode for satellite configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigEditMode {
    List,
    Edit,
    Add,
}

/// Field being edited in satellite config
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    Name,
    TleLine1,
    TleLine2,
    LaunchDate,
    LaunchSite,
    CountryOfOrigin,
    Operator,
    SatelliteType,
    DownlinkFrequency,
    UplinkFrequency,
    Notes,
}

impl ConfigField {
    fn next(&self) -> Self {
        match self {
            ConfigField::Name => ConfigField::TleLine1,
            ConfigField::TleLine1 => ConfigField::TleLine2,
            ConfigField::TleLine2 => ConfigField::LaunchDate,
            ConfigField::LaunchDate => ConfigField::LaunchSite,
            ConfigField::LaunchSite => ConfigField::CountryOfOrigin,
            ConfigField::CountryOfOrigin => ConfigField::Operator,
            ConfigField::Operator => ConfigField::SatelliteType,
            ConfigField::SatelliteType => ConfigField::DownlinkFrequency,
            ConfigField::DownlinkFrequency => ConfigField::UplinkFrequency,
            ConfigField::UplinkFrequency => ConfigField::Notes,
            ConfigField::Notes => ConfigField::Name,
        }
    }

    fn prev(&self) -> Self {
        match self {
            ConfigField::Name => ConfigField::Notes,
            ConfigField::TleLine1 => ConfigField::Name,
            ConfigField::TleLine2 => ConfigField::TleLine1,
            ConfigField::LaunchDate => ConfigField::TleLine2,
            ConfigField::LaunchSite => ConfigField::LaunchDate,
            ConfigField::CountryOfOrigin => ConfigField::LaunchSite,
            ConfigField::Operator => ConfigField::CountryOfOrigin,
            ConfigField::SatelliteType => ConfigField::Operator,
            ConfigField::DownlinkFrequency => ConfigField::SatelliteType,
            ConfigField::UplinkFrequency => ConfigField::DownlinkFrequency,
            ConfigField::Notes => ConfigField::UplinkFrequency,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ConfigField::Name => "Name",
            ConfigField::TleLine1 => "TLE Line 1",
            ConfigField::TleLine2 => "TLE Line 2",
            ConfigField::LaunchDate => "Launch Date",
            ConfigField::LaunchSite => "Launch Site",
            ConfigField::CountryOfOrigin => "Country",
            ConfigField::Operator => "Operator",
            ConfigField::SatelliteType => "Type",
            ConfigField::DownlinkFrequency => "Downlink (MHz)",
            ConfigField::UplinkFrequency => "Uplink (MHz)",
            ConfigField::Notes => "Notes",
        }
    }
}

/// State for satellite configuration screen
pub struct SatelliteConfigState {
    pub satellites: Vec<SatelliteDetails>,
    pub selected_index: usize,
    pub edit_mode: ConfigEditMode,
    pub current_field: ConfigField,
    pub editing_satellite: SatelliteDetails,
    pub input_buffer: String,
    pub status_message: Option<String>,
}

impl SatelliteConfigState {
    fn new() -> Self {
        Self {
            satellites: Vec::new(),
            selected_index: 0,
            edit_mode: ConfigEditMode::List,
            current_field: ConfigField::Name,
            editing_satellite: SatelliteDetails::default(),
            input_buffer: String::new(),
            status_message: None,
        }
    }

    fn load_from_database(&mut self, db: &Database) -> Result<()> {
        self.satellites = db.read_all()?;
        if self.selected_index >= self.satellites.len() && !self.satellites.is_empty() {
            self.selected_index = self.satellites.len() - 1;
        }
        Ok(())
    }

    fn get_field_value(&self, field: ConfigField) -> String {
        match field {
            ConfigField::Name => self.editing_satellite.name.clone(),
            ConfigField::TleLine1 => self.editing_satellite.tle_line1.clone(),
            ConfigField::TleLine2 => self.editing_satellite.tle_line2.clone(),
            ConfigField::LaunchDate => self.editing_satellite.launch_date.clone().unwrap_or_default(),
            ConfigField::LaunchSite => self.editing_satellite.launch_site.clone().unwrap_or_default(),
            ConfigField::CountryOfOrigin => self.editing_satellite.country_of_origin.clone().unwrap_or_default(),
            ConfigField::Operator => self.editing_satellite.operator.clone().unwrap_or_default(),
            ConfigField::SatelliteType => self.editing_satellite.satellite_type.clone().unwrap_or_default(),
            ConfigField::DownlinkFrequency => self.editing_satellite.downlink_frequency_mhz
                .map(|f| format!("{:.3}", f))
                .unwrap_or_default(),
            ConfigField::UplinkFrequency => self.editing_satellite.uplink_frequency_mhz
                .map(|f| format!("{:.3}", f))
                .unwrap_or_default(),
            ConfigField::Notes => self.editing_satellite.notes.clone().unwrap_or_default(),
        }
    }

    fn set_field_value(&mut self, field: ConfigField, value: String) {
        match field {
            ConfigField::Name => self.editing_satellite.name = value,
            ConfigField::TleLine1 => self.editing_satellite.tle_line1 = value,
            ConfigField::TleLine2 => self.editing_satellite.tle_line2 = value,
            ConfigField::LaunchDate => {
                self.editing_satellite.launch_date = if value.is_empty() { None } else { Some(value) }
            }
            ConfigField::LaunchSite => {
                self.editing_satellite.launch_site = if value.is_empty() { None } else { Some(value) }
            }
            ConfigField::CountryOfOrigin => {
                self.editing_satellite.country_of_origin = if value.is_empty() { None } else { Some(value) }
            }
            ConfigField::Operator => {
                self.editing_satellite.operator = if value.is_empty() { None } else { Some(value) }
            }
            ConfigField::SatelliteType => {
                self.editing_satellite.satellite_type = if value.is_empty() { None } else { Some(value) }
            }
            ConfigField::DownlinkFrequency => {
                self.editing_satellite.downlink_frequency_mhz = value.parse().ok()
            }
            ConfigField::UplinkFrequency => {
                self.editing_satellite.uplink_frequency_mhz = value.parse().ok()
            }
            ConfigField::Notes => {
                self.editing_satellite.notes = if value.is_empty() { None } else { Some(value) }
            }
        }
    }
}

pub struct AppState {
    pub satellites: Vec<Satellite>,
    pub current_positions: Vec<SatellitePosition>,
    pub selected_satellite: usize,
    pub observer: Observer,
    pub config: Config,
    pub alerts: Vec<Alert>,
    pub mode: AppMode,
    pub sat_config_state: SatelliteConfigState,
    pub database: Database,
}

#[derive(Clone, Debug)]
pub struct Alert {
    pub satellite_name: String,
    pub pass: SatellitePass,
    pub time_until_minutes: i64,
    #[allow(dead_code)]
    pub shown: bool,
}

fn main() -> Result<()> {
    // Load configuration
    let config = match Config::load("config.toml") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("\nError: Could not load configuration file 'config.toml'");
            eprintln!("   Reason: {}\n", e);
            eprintln!("   To fix this issue:");
            eprintln!("   1. Copy the example configuration file:");
            eprintln!("      cp example.config.toml config.toml");
            eprintln!("   2. Edit config.toml with your location and preferences");
            eprintln!("   3. Run the program again\n");
            std::process::exit(1);
        }
    };

    // Create observer
    let observer = Observer::new(
        config.observer.name.clone(),
        config.observer.latitude,
        config.observer.longitude,
        config.observer.altitude,
    );

    // Load TLE data and create satellites
    let tle_data = fs::read_to_string(&config.satellites.tle_file)?;
    let mut satellites = parse_multiple_tles(&tle_data, &config)?;

    // Predict passes for all satellites
    println!("Predicting passes for {} satellites...", satellites.len());
    for satellite in satellites.iter_mut() {
        match predict_passes(
            &satellite.elements,
            &satellite.epoch,
            &observer,
            &config.prediction,
        ) {
            Ok(passes) => {
                satellite.passes = passes;
                println!(
                    "  {} - Found {} passes",
                    satellite.name,
                    satellite.passes.len()
                );
            }
            Err(e) => {
                eprintln!("  {} - Error: {}", satellite.name, e);
                satellite.passes = Vec::new();
            }
        }
    }

    // Calculate initial positions
    let mut current_positions = satellites
        .iter()
        .filter_map(|sat| sat.calculate_position(Utc::now(), &observer).ok())
        .collect::<Vec<_>>();

    // Add radio calculations if enabled
    if config.radio.enabled {
        for pos in current_positions.iter_mut() {
            pos.doppler = Some(calculate_doppler_shift(
                pos,
                config.radio.downlink_frequency_mhz,
                config.radio.uplink_frequency_mhz,
            ));
            pos.comm_window = Some(evaluate_communication_window(pos));
        }
    }

    // Initialize database
    let db_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("crabtrack")
        .join("satellites.db");

    // Ensure the directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let database = Database::open(&db_path)?;
    println!("Database initialized at: {}", db_path.display());

    // Load satellite config state from database
    let mut sat_config_state = SatelliteConfigState::new();
    if let Err(e) = sat_config_state.load_from_database(&database) {
        eprintln!("Warning: Could not load satellite details from database: {}", e);
    }

    let mut app_state = AppState {
        satellites,
        current_positions,
        selected_satellite: 0,
        observer,
        config,
        alerts: Vec::new(),
        mode: AppMode::Normal,
        sat_config_state,
        database,
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run TUI
    let res = run_app(&mut terminal, &mut app_state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn parse_multiple_tles(tle_data: &str, config: &Config) -> Result<Vec<Satellite>> {
    let lines: Vec<&str> = tle_data.lines().collect();
    let mut satellites = Vec::new();

    let mut i = 0;
    while i < lines.len() - 2 {
        if !lines[i].is_empty() && lines[i + 1].starts_with('1') && lines[i + 2].starts_with('2') {
            let name = lines[i].trim().to_string();
            let tle_line1 = lines[i + 1];

            // Check if we should track this satellite
            let should_track = if config.satellites.tracked_satellites.is_empty() {
                satellites.len() < config.satellites.max_satellites
            } else {
                config
                    .satellites
                    .tracked_satellites
                    .iter()
                    .any(|tracked| name.contains(tracked))
            };

            if should_track {
                // Parse epoch from TLE line 1, columns 18-32
                let epoch_datetime = if tle_line1.len() >= 32 {
                    let epoch_str = &tle_line1[18..32];

                    if let Ok(epoch_val) = epoch_str.trim().parse::<f64>() {
                        let year_2digit = (epoch_val / 1000.0).floor() as i32;
                        let day_of_year = epoch_val % 1000.0;

                        let full_year = if year_2digit >= 57 {
                            1900 + year_2digit
                        } else {
                            2000 + year_2digit
                        };

                        year_day_to_datetime(full_year, day_of_year)
                    } else {
                        Utc::now() // Fallback
                    }
                } else {
                    Utc::now() // Fallback
                };

                match Elements::from_tle(
                    Some(name.clone()),
                    lines[i + 1].as_bytes(),
                    lines[i + 2].as_bytes(),
                ) {
                    Ok(elements) => {
                        satellites.push(Satellite::new(name, elements, epoch_datetime));
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse TLE for {}: {:?}", name, e);
                    }
                }
            }

            i += 3;
        } else {
            i += 1;
        }
    }

    if satellites.is_empty() {
        return Err(anyhow::anyhow!("No valid satellites found in TLE file"));
    }

    Ok(satellites)
}

fn predict_passes(
    elements: &Elements,
    tle_epoch: &DateTime<Utc>,
    observer: &Observer,
    config: &config::PredictionConfig,
) -> Result<Vec<SatellitePass>> {
    let mut passes = Vec::new();
    let start_time = Utc::now();
    let end_time = start_time + Duration::days(config.search_days as i64);
    let observer_ecef = observer.to_ecef();

    // Check if TLE is too old
    let tle_age_seconds = (start_time.timestamp() - tle_epoch.timestamp()).abs();
    let tle_age_days = tle_age_seconds / 86400;

    if tle_age_days > 30 {
        eprintln!(
            "Warning: TLE data is {} days old. Predictions may be inaccurate.",
            tle_age_days
        );
        if tle_age_days > 90 {
            return Err(anyhow::anyhow!(
                "TLE data is too old ({} days). Please download fresh TLE data from https://celestrak.org",
                tle_age_days
            ));
        }
    }

    let mut current_time = start_time;
    let time_step = Duration::seconds(config.time_step as i64);

    let mut in_pass = false;
    let mut pass_start = start_time;
    let mut max_elevation = 0.0;
    let mut max_elevation_time = start_time;
    let mut aos_azimuth = 0.0;
    let mut max_azimuth = 0.0;
    let mut max_range = 0.0;

    let constants = Constants::from_elements(elements)?;

    while current_time < end_time && passes.len() < config.num_passes {
        // Convert current time to minutes since TLE epoch
        let minutes_since_epoch = calculate_minutes_since_epoch_simple(tle_epoch, current_time);

        // Try to propagate, skip if error
        let prediction = match constants.propagate(MinutesSinceEpoch(minutes_since_epoch)) {
            Ok(pred) => pred,
            Err(e) => {
                eprintln!("Warning: Propagation failed at {:?}: {:?}", current_time, e);
                break;
            }
        };

        let sat_pos = nalgebra::Vector3::new(
            prediction.position[0] * 1000.0,
            prediction.position[1] * 1000.0,
            prediction.position[2] * 1000.0,
        );

        // Calculate look angles
        let gmst = calculate_gmst(current_time);
        let look_angles = calculate_look_angles(
            &sat_pos,
            &observer_ecef,
            gmst,
            observer.latitude,
            observer.longitude,
        );

        // Check if satellite is above horizon
        if look_angles.elevation >= config.min_elevation {
            if !in_pass {
                in_pass = true;
                pass_start = current_time;
                aos_azimuth = look_angles.azimuth;
                max_elevation = look_angles.elevation;
                max_elevation_time = current_time;
                max_azimuth = look_angles.azimuth;
                max_range = look_angles.range;
            } else {
                if look_angles.elevation > max_elevation {
                    max_elevation = look_angles.elevation;
                    max_elevation_time = current_time;
                    max_azimuth = look_angles.azimuth;
                    max_range = look_angles.range;
                }
            }
        } else if in_pass {
            let pass = SatellitePass {
                aos_time: pass_start,
                los_time: current_time,
                max_elevation,
                max_elevation_time,
                aos_azimuth,
                max_azimuth,
                los_azimuth: look_angles.azimuth,
                duration_seconds: (current_time - pass_start).num_seconds() as f64,
                max_range_km: max_range,
            };
            passes.push(pass);
            in_pass = false;
        }

        current_time = current_time + time_step;
    }

    Ok(passes)
}

fn calculate_minutes_since_epoch_simple(tle_epoch: &DateTime<Utc>, time: DateTime<Utc>) -> f64 {
    let duration = time.signed_duration_since(*tle_epoch);
    duration.num_milliseconds() as f64 / 60000.0
}

fn year_day_to_datetime(year: i32, day_of_year: f64) -> DateTime<Utc> {
    let year_start = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let days_into_year = day_of_year - 1.0;
    year_start + Duration::milliseconds((days_into_year * 86400000.0) as i64)
}

fn update_alerts(app_state: &mut AppState) {
    if !app_state.config.alerts.enabled {
        return;
    }

    let now = Utc::now();
    app_state.alerts.clear();

    for satellite in &app_state.satellites {
        if let Some(next_pass) = satellite.get_next_pass() {
            // Check if pass meets minimum elevation requirement
            if next_pass.max_elevation < app_state.config.alerts.min_elevation_for_alert {
                continue;
            }

            let time_until = next_pass.aos_time.signed_duration_since(now);
            let minutes_until = time_until.num_minutes();

            if minutes_until > 0 && minutes_until <= app_state.config.alerts.alert_before_pass {
                app_state.alerts.push(Alert {
                    satellite_name: satellite.name.clone(),
                    pass: next_pass.clone(),
                    time_until_minutes: minutes_until,
                    shown: false,
                });
            }
        }
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app_state: &mut AppState,
) -> Result<()> {
    loop {
        match app_state.mode {
            AppMode::Normal => {
                // Update current positions
                let now = Utc::now();
                app_state.current_positions = app_state
                    .satellites
                    .iter()
                    .filter_map(|sat| sat.calculate_position(now, &app_state.observer).ok())
                    .collect();

                // Add radio calculations if enabled
                if app_state.config.radio.enabled {
                    for pos in app_state.current_positions.iter_mut() {
                        pos.doppler = Some(calculate_doppler_shift(
                            pos,
                            app_state.config.radio.downlink_frequency_mhz,
                            app_state.config.radio.uplink_frequency_mhz,
                        ));
                        pos.comm_window = Some(evaluate_communication_window(pos));
                    }
                }

                // Update alerts
                update_alerts(app_state);

                terminal.draw(|f| {
                    ui::draw_ui(f, app_state);
                })?;

                // Handle input for normal mode
                if event::poll(std::time::Duration::from_millis(
                    app_state.config.display.refresh_rate,
                ))? {
                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(());
                            }
                            KeyCode::Char('c') => {
                                // Enter satellite configuration mode
                                if let Err(e) = app_state.sat_config_state.load_from_database(&app_state.database) {
                                    eprintln!("Error loading satellite config: {}", e);
                                }
                                app_state.mode = AppMode::SatelliteConfig;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if app_state.selected_satellite > 0 {
                                    app_state.selected_satellite -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if app_state.selected_satellite < app_state.satellites.len() - 1 {
                                    app_state.selected_satellite += 1;
                                }
                            }
                            KeyCode::Home => {
                                app_state.selected_satellite = 0;
                            }
                            KeyCode::End => {
                                app_state.selected_satellite = app_state.satellites.len() - 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
            AppMode::SatelliteConfig => {
                terminal.draw(|f| {
                    ui::draw_satellite_config(f, app_state);
                })?;

                // Handle input for satellite config mode
                if event::poll(std::time::Duration::from_millis(100))? {
                    if let Event::Key(key) = event::read()? {
                        handle_satellite_config_input(app_state, key.code)?;
                    }
                }
            }
        }
    }
}

fn handle_satellite_config_input(app_state: &mut AppState, key: KeyCode) -> Result<()> {
    let state = &mut app_state.sat_config_state;

    match state.edit_mode {
        ConfigEditMode::List => {
            match key {
                KeyCode::Esc | KeyCode::Char('q') => {
                    // Return to normal mode
                    app_state.mode = AppMode::Normal;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if state.selected_index > 0 {
                        state.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if state.selected_index < state.satellites.len().saturating_sub(1) {
                        state.selected_index += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    // Edit selected satellite
                    if !state.satellites.is_empty() {
                        state.editing_satellite = state.satellites[state.selected_index].clone();
                        state.current_field = ConfigField::Name;
                        state.input_buffer = state.get_field_value(state.current_field);
                        state.edit_mode = ConfigEditMode::Edit;
                    }
                }
                KeyCode::Char('a') => {
                    // Add new satellite
                    state.editing_satellite = SatelliteDetails::default();
                    state.current_field = ConfigField::Name;
                    state.input_buffer.clear();
                    state.edit_mode = ConfigEditMode::Add;
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    // Delete selected satellite
                    if !state.satellites.is_empty() {
                        let sat = &state.satellites[state.selected_index];
                        if let Some(id) = sat.id {
                            if app_state.database.delete(id).is_ok() {
                                state.status_message = Some(format!("Deleted: {}", sat.name));
                                let _ = state.load_from_database(&app_state.database);
                            } else {
                                state.status_message = Some("Failed to delete satellite".to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ConfigEditMode::Edit | ConfigEditMode::Add => {
            match key {
                KeyCode::Esc => {
                    // Cancel edit and return to list
                    state.edit_mode = ConfigEditMode::List;
                    state.status_message = Some("Edit cancelled".to_string());
                }
                KeyCode::Tab => {
                    // Save current field and move to next
                    state.set_field_value(state.current_field, state.input_buffer.clone());
                    state.current_field = state.current_field.next();
                    state.input_buffer = state.get_field_value(state.current_field);
                }
                KeyCode::BackTab => {
                    // Save current field and move to previous
                    state.set_field_value(state.current_field, state.input_buffer.clone());
                    state.current_field = state.current_field.prev();
                    state.input_buffer = state.get_field_value(state.current_field);
                }
                KeyCode::Up => {
                    // Save current field and move to previous
                    state.set_field_value(state.current_field, state.input_buffer.clone());
                    state.current_field = state.current_field.prev();
                    state.input_buffer = state.get_field_value(state.current_field);
                }
                KeyCode::Down => {
                    // Save current field and move to next
                    state.set_field_value(state.current_field, state.input_buffer.clone());
                    state.current_field = state.current_field.next();
                    state.input_buffer = state.get_field_value(state.current_field);
                }
                KeyCode::Enter => {
                    // Save current field value
                    state.set_field_value(state.current_field, state.input_buffer.clone());

                    // Save to database
                    if state.editing_satellite.name.is_empty() {
                        state.status_message = Some("Error: Name is required".to_string());
                    } else {
                        let result = if state.edit_mode == ConfigEditMode::Add {
                            app_state.database.create(&state.editing_satellite)
                        } else {
                            app_state.database.update(&state.editing_satellite)
                                .map(|_| state.editing_satellite.id.unwrap_or(0))
                        };

                        match result {
                            Ok(_) => {
                                state.status_message = Some(format!("Saved: {}", state.editing_satellite.name));
                                let _ = state.load_from_database(&app_state.database);
                                state.edit_mode = ConfigEditMode::List;
                            }
                            Err(e) => {
                                state.status_message = Some(format!("Error saving: {}", e));
                            }
                        }
                    }
                }
                KeyCode::Char(c) => {
                    state.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    state.input_buffer.pop();
                }
                _ => {}
            }
        }
    }

    Ok(())
}