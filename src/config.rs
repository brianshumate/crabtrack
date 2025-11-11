use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub observer: ObserverConfig,
    pub satellites: SatellitesConfig,
    pub prediction: PredictionConfig,
    pub display: DisplayConfig,
    pub radio: RadioConfig,
    pub alerts: AlertsConfig,
}

#[derive(Debug, Deserialize)]
pub struct ObserverConfig {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

#[derive(Debug, Deserialize)]
pub struct SatellitesConfig {
    pub tle_file: PathBuf,
    pub tracked_satellites: Vec<String>,
    pub max_satellites: usize,
}

#[derive(Debug, Deserialize)]
pub struct PredictionConfig {
    pub num_passes: usize,
    pub min_elevation: f64,
    pub search_days: f64,
    pub time_step: f64,
}

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub refresh_rate: u64,
    pub show_current_position: bool,
    pub show_all_positions: bool,
    pub show_sky_map: bool,
}

#[derive(Debug, Deserialize)]
pub struct RadioConfig {
    pub enabled: bool,
    pub downlink_frequency_mhz: f64,
    pub uplink_frequency_mhz: f64,
    pub show_doppler: bool,
}

#[derive(Debug, Deserialize)]
pub struct AlertsConfig {
    pub enabled: bool,
    pub alert_before_pass: i64, // minutes
    pub min_elevation_for_alert: f64,
    #[allow(dead_code)]
    pub play_sound: bool,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}
