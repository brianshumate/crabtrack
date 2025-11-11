mod config;
mod observer;
mod pass_prediction;
mod radio;
mod satellite;
mod ui;

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

struct AppState {
    satellites: Vec<Satellite>,
    current_positions: Vec<SatellitePosition>,
    selected_satellite: usize,
    observer: Observer,
    config: Config,
    alerts: Vec<Alert>,
}

#[derive(Clone, Debug)]
struct Alert {
    satellite_name: String,
    pass: SatellitePass,
    time_until_minutes: i64,
    #[allow(dead_code)]
    shown: bool,
}

fn main() -> Result<()> {
    // Load configuration
    let config = Config::load("config.toml")?;

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

    let mut app_state = AppState {
        satellites,
        current_positions,
        selected_satellite: 0,
        observer,
        config,
        alerts: Vec::new(),
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

        // Handle input
        if event::poll(std::time::Duration::from_millis(
            app_state.config.display.refresh_rate,
        ))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
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
}
