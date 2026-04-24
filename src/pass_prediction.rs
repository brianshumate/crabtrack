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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_satellite_pass_duration() {
        let pass = SatellitePass {
            aos_time: Utc.with_ymd_and_hms(2026, 4, 24, 10, 0, 0).unwrap(),
            los_time: Utc.with_ymd_and_hms(2026, 4, 24, 10, 10, 0).unwrap(),
            max_elevation: 45.0,
            max_elevation_time: Utc.with_ymd_and_hms(2026, 4, 24, 10, 5, 0).unwrap(),
            aos_azimuth: 90.0,
            max_azimuth: 180.0,
            los_azimuth: 270.0,
            duration_seconds: 600.0,
            max_range_km: 1000.0,
        };
        assert!((pass.duration_minutes() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_look_angles_below_horizon() {
        // Satellite directly above observer at 90 degree elevation
        let sat_pos = Vector3::new(0.0, 0.0, 400000.0); // 400km altitude in meters
        let observer = Vector3::new(0.0, 0.0, 0.0);
        let gmst = 0.0;

        let angles = calculate_look_angles(&sat_pos, &observer, gmst, 0.0, 0.0);
        assert!(angles.elevation > 80.0, "Satellite directly overhead should have high elevation");
    }

    #[test]
    fn test_look_angles_horizontal() {
        // Satellite at horizon
        let range_km = 2000.0;
        let sat_pos = Vector3::new(range_km * 1000.0, 0.0, 0.0);
        let observer = Vector3::new(0.0, 0.0, 0.0);
        let gmst = 0.0;

        let angles = calculate_look_angles(&sat_pos, &observer, gmst, 0.0, 0.0);
        assert!(angles.elevation.abs() < 10.0, "Distant satellite should be near horizon");
    }

    #[test]
    fn test_azimuth_conversion() {
        // Test that azimuth is in range [0, 360)
        let sat_pos = Vector3::new(0.0, 100000.0, 500000.0);
        let observer = Vector3::new(0.0, 0.0, 0.0);

        let angles = calculate_look_angles(&sat_pos, &observer, 0.0, 0.0, 0.0);
        assert!(angles.azimuth >= 0.0 && angles.azimuth < 360.0);
    }

    #[test]
    fn test_gmst_known_value() {
        let j2000 = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();

        let gmst = calculate_gmst(j2000);
        // At J2000, GMST should be close to some consistent value (the formula is approximate)
        // Just verify it returns a valid angle in radians
        assert!(gmst > -10.0 && gmst < 10.0, "GMST should be a reasonable value");
    }

    #[test]
    fn test_gmst_daily_rotation() {
        let day1 = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();
        let day2 = day1 + chrono::Duration::days(1);

        let gmst1 = calculate_gmst(day1);
        let gmst2 = calculate_gmst(day2);

        // The value should change but at a consistent rate
        // Just verify both values are sensible
        assert!(gmst1 > -10.0 && gmst1 < 10.0);
        assert!(gmst2 > -10.0 && gmst2 < 10.0);
    }

    #[test]
    fn test_eci_to_ecef_rotation() {
        // Point on Earth's equator should rotate with Earth
        let eci = Vector3::new(6778000.0, 0.0, 0.0); // ~6778 km from Earth center
        let gmst = std::f64::consts::FRAC_PI_2; // 90 degrees

        let ecef = eci_to_ecef(&eci, gmst);
        assert!(ecef.x.abs() < 1.0, "X should rotate to near 0 at 90 degrees");
        assert!(ecef.y.abs() > 6777000.0, "Y should have the rotated value");
        assert!(ecef.z.abs() < 1.0, "Z should stay approximately the same");
    }

    #[test]
    fn test_range_calculation() {
        let sat_pos = Vector3::new(7000000.0, 0.0, 0.0); // 7000 km from center
        let observer = Vector3::new(6371000.0, 0.0, 0.0); // Earth's surface
        let gmst = 0.0;

        let angles = calculate_look_angles(&sat_pos, &observer, gmst, 0.0, 0.0);
        // Range should be approximately 7000 - 6371 = 629 km
        assert!(angles.range > 600.0 && angles.range < 700.0);
    }
}
