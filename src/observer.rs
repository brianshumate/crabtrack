use nalgebra::Vector3;

pub struct Observer {
    pub name: String,
    pub latitude: f64,  // degrees
    pub longitude: f64, // degrees
    pub altitude: f64,  // meters
}

impl Observer {
    pub fn new(name: String, lat: f64, lon: f64, alt: f64) -> Self {
        Self {
            name,
            latitude: lat,
            longitude: lon,
            altitude: alt,
        }
    }

    pub fn to_ecef(&self) -> Vector3<f64> {
        let lat_rad = self.latitude.to_radians();
        let lon_rad = self.longitude.to_radians();

        const A: f64 = 6378137.0;
        const F: f64 = 1.0 / 298.257223563;
        const E2: f64 = F * (2.0 - F);

        let n = A / (1.0 - E2 * lat_rad.sin().powi(2)).sqrt();

        let x = (n + self.altitude) * lat_rad.cos() * lon_rad.cos();
        let y = (n + self.altitude) * lat_rad.cos() * lon_rad.sin();
        let z = (n * (1.0 - E2) + self.altitude) * lat_rad.sin();

        Vector3::new(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_creation() {
        let obs = Observer::new("Test Station".to_string(), 45.0, -122.0, 100.0);
        assert_eq!(obs.name, "Test Station");
        assert_eq!(obs.latitude, 45.0);
        assert_eq!(obs.longitude, -122.0);
        assert_eq!(obs.altitude, 100.0);
    }

    #[test]
    fn test_ecef_at_equator() {
        let obs = Observer::new("Equator".to_string(), 0.0, 0.0, 0.0);
        let ecef = obs.to_ecef();

        let radius = (ecef.x.powi(2) + ecef.y.powi(2) + ecef.z.powi(2)).sqrt() / 1000.0;
        assert!((radius - 6378.137).abs() < 1.0);
    }

    #[test]
    fn test_ecef_at_pole() {
        let obs = Observer::new("North Pole".to_string(), 90.0, 0.0, 0.0);
        let ecef = obs.to_ecef();

        assert!(ecef.x.abs() < 1.0);
        assert!(ecef.y.abs() < 1.0);
        assert!(ecef.z / 1000.0 > 6300.0);
    }

    #[test]
    fn test_ecef_altitude() {
        let obs_low = Observer::new("Low".to_string(), 0.0, 0.0, 0.0);
        let obs_high = Observer::new("High".to_string(), 0.0, 0.0, 1000.0);

        let ecef_low = obs_low.to_ecef();
        let ecef_high = obs_high.to_ecef();

        let radius_low = (ecef_low.x.powi(2) + ecef_low.y.powi(2) + ecef_low.z.powi(2)).sqrt();
        let radius_high = (ecef_high.x.powi(2) + ecef_high.y.powi(2) + ecef_high.z.powi(2)).sqrt();

        assert!((radius_high - radius_low - 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_latitude_range() {
        for lat in [-90.0, -45.0, 0.0, 45.0, 90.0].iter() {
            let obs = Observer::new("Test".to_string(), *lat, 0.0, 0.0);
            let ecef = obs.to_ecef();
            let radius = (ecef.x.powi(2) + ecef.y.powi(2) + ecef.z.powi(2)).sqrt() / 1000.0;
            assert!(radius > 6300.0 && radius < 6400.0);
        }
    }
}