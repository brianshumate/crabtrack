use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use nalgebra::Vector3;
use sgp4::{Constants, Elements, MinutesSinceEpoch};

use crate::observer::Observer;
use crate::pass_prediction::{calculate_gmst, calculate_look_angles, SatellitePass};
use crate::radio::{CommunicationWindow, DopplerShift};

#[derive(Clone)]
pub struct Satellite {
    pub name: String,
    pub elements: Elements,
    pub passes: Vec<SatellitePass>,
    pub epoch: DateTime<Utc>, // Add this field
}

#[derive(Debug, Clone)]
pub struct SatellitePosition {
    pub name: String,
    #[allow(dead_code)]
    pub time: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_km: f64,
    pub velocity_km_s: f64,
    pub azimuth: f64,
    pub elevation: f64,
    pub range_km: f64,
    pub is_visible: bool,
    pub doppler: Option<DopplerShift>,
    pub comm_window: Option<CommunicationWindow>,
}

impl Satellite {
    pub fn new(name: String, elements: Elements, epoch: DateTime<Utc>) -> Self {
        Self {
            name,
            elements,
            passes: Vec::new(),
            epoch,
        }
    }

    pub fn calculate_position(
        &self,
        time: DateTime<Utc>,
        observer: &Observer,
    ) -> Result<SatellitePosition> {
        let constants = Constants::from_elements(&self.elements)?;
        let epoch_time = self.elements.epoch();
        let minutes_since_epoch = self.calculate_minutes_since_epoch(time, epoch_time);

        let prediction = constants.propagate(MinutesSinceEpoch(minutes_since_epoch))?;

        // Get position in ECI (km)
        let sat_pos_km = Vector3::new(
            prediction.position[0],
            prediction.position[1],
            prediction.position[2],
        );

        // Get velocity in ECI (km/s)
        let sat_vel_km_s = Vector3::new(
            prediction.velocity[0],
            prediction.velocity[1],
            prediction.velocity[2],
        );

        let velocity_km_s = sat_vel_km_s.norm();

        // Convert to meters for calculations
        let sat_pos = sat_pos_km * 1000.0;

        // Calculate look angles
        let observer_ecef = observer.to_ecef();
        let gmst = calculate_gmst(time);
        let look_angles = calculate_look_angles(
            &sat_pos,
            &observer_ecef,
            gmst,
            observer.latitude,
            observer.longitude,
        );

        // Convert ECI to geodetic coordinates
        let (lat, lon, alt_km) = eci_to_geodetic(&sat_pos_km, gmst);

        Ok(SatellitePosition {
            name: self.name.clone(),
            time,
            latitude: lat,
            longitude: lon,
            altitude_km: alt_km,
            velocity_km_s,
            azimuth: look_angles.azimuth,
            elevation: look_angles.elevation,
            range_km: look_angles.range,
            is_visible: look_angles.elevation > 0.0,
            doppler: None,     // Will be calculated separately if radio enabled
            comm_window: None, // Will be calculated separately if radio enabled
        })
    }

    pub fn get_next_pass(&self) -> Option<&SatellitePass> {
        let now = Utc::now();
        self.passes.iter().find(|pass| pass.aos_time > now)
    }

    fn calculate_minutes_since_epoch(&self, time: DateTime<Utc>, epoch_day_of_year: f64) -> f64 {
        let current_year = time.year();

        let mut epoch_time = year_day_to_datetime(current_year, epoch_day_of_year);

        let diff_current = (time.timestamp() - epoch_time.timestamp()).abs();

        let epoch_time_prev = year_day_to_datetime(current_year - 1, epoch_day_of_year);
        let diff_prev = (time.timestamp() - epoch_time_prev.timestamp()).abs();

        let epoch_time_next = year_day_to_datetime(current_year + 1, epoch_day_of_year);
        let diff_next = (time.timestamp() - epoch_time_next.timestamp()).abs();

        if diff_prev < diff_current && diff_prev < diff_next {
            epoch_time = epoch_time_prev;
        } else if diff_next < diff_current && diff_next < diff_prev {
            epoch_time = epoch_time_next;
        }

        let duration = time.signed_duration_since(epoch_time);
        duration.num_milliseconds() as f64 / 60000.0
    }
}

fn year_day_to_datetime(year: i32, day_of_year: f64) -> DateTime<Utc> {
    use chrono::Duration;
    let year_start = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let days_into_year = day_of_year - 1.0;
    year_start + Duration::milliseconds((days_into_year * 86400000.0) as i64)
}

fn eci_to_geodetic(eci: &Vector3<f64>, gmst: f64) -> (f64, f64, f64) {
    // Convert ECI to ECEF
    let cos_gmst = gmst.cos();
    let sin_gmst = gmst.sin();

    let x = eci.x * cos_gmst + eci.y * sin_gmst;
    let y = -eci.x * sin_gmst + eci.y * cos_gmst;
    let z = eci.z;

    // WGS84 parameters
    const A: f64 = 6378.137; // km
    const F: f64 = 1.0 / 298.257223563;
    const E2: f64 = F * (2.0 - F);

    // Calculate longitude
    let lon = y.atan2(x);

    // Iteratively calculate latitude
    let p = (x * x + y * y).sqrt();
    let mut lat = (z / p).atan();

    for _ in 0..5 {
        let sin_lat = lat.sin();
        let n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();
        let h = p / lat.cos() - n;
        lat = (z / p / (1.0 - E2 * n / (n + h))).atan();
    }

    let sin_lat = lat.sin();
    let n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();
    let alt = p / lat.cos() - n;

    (lat.to_degrees(), lon.to_degrees(), alt)
}
