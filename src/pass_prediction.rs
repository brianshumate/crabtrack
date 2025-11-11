use chrono::{DateTime, Utc};
use nalgebra::Vector3;

#[derive(Debug, Clone)]
pub struct SatellitePass {
    pub aos_time: DateTime<Utc>, // Acquisition of Signal
    pub los_time: DateTime<Utc>, // Loss of Signal
    pub max_elevation: f64,
    pub max_elevation_time: DateTime<Utc>,
    pub aos_azimuth: f64,
    pub max_azimuth: f64,
    pub los_azimuth: f64,
    pub duration_seconds: f64,
    pub max_range_km: f64,
}

#[derive(Debug)]
pub struct LookAngles {
    pub azimuth: f64,   // degrees
    pub elevation: f64, // degrees
    pub range: f64,     // kilometers
}

impl SatellitePass {
    pub fn duration_minutes(&self) -> f64 {
        self.duration_seconds / 60.0
    }
}

pub fn calculate_look_angles(
    sat_pos_eci: &Vector3<f64>,
    observer_ecef: &Vector3<f64>,
    gmst: f64,
    observer_lat: f64,
    observer_lon: f64,
) -> LookAngles {
    // Convert satellite ECI to ECEF
    let sat_ecef = eci_to_ecef(sat_pos_eci, gmst);

    // Range vector from observer to satellite
    let range_vec = sat_ecef - observer_ecef;
    let range_km = range_vec.norm() / 1000.0;

    // Convert to topocentric (SEZ) coordinates
    let lat_rad = observer_lat.to_radians();
    let lon_rad = observer_lon.to_radians();

    let south = range_vec.x * lat_rad.cos() * lon_rad.cos()
        + range_vec.y * lat_rad.cos() * lon_rad.sin()
        - range_vec.z * lat_rad.sin();

    let east = -range_vec.x * lon_rad.sin() + range_vec.y * lon_rad.cos();

    let zenith = range_vec.x * lat_rad.sin() * lon_rad.cos()
        + range_vec.y * lat_rad.sin() * lon_rad.sin()
        + range_vec.z * lat_rad.cos();

    // Calculate azimuth and elevation
    let azimuth = east.atan2(-south).to_degrees();
    let azimuth = if azimuth < 0.0 {
        azimuth + 360.0
    } else {
        azimuth
    };

    let elevation = (zenith / range_km / 1000.0).asin().to_degrees();

    LookAngles {
        azimuth,
        elevation,
        range: range_km,
    }
}

fn eci_to_ecef(eci: &Vector3<f64>, gmst: f64) -> Vector3<f64> {
    let cos_gmst = gmst.cos();
    let sin_gmst = gmst.sin();

    Vector3::new(
        eci.x * cos_gmst + eci.y * sin_gmst,
        -eci.x * sin_gmst + eci.y * cos_gmst,
        eci.z,
    )
}

pub fn calculate_gmst(time: DateTime<Utc>) -> f64 {
    // Julian date calculation
    let j2000 = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();

    let jd_epoch = time.signed_duration_since(j2000).num_milliseconds() as f64 / 86400000.0;

    // GMST calculation (simplified)
    let gmst_hours = 18.697374558 + 24.06570982441908 * jd_epoch;
    let gmst_hours = gmst_hours % 24.0;

    (gmst_hours * 15.0).to_radians() // Convert hours to radians
}
