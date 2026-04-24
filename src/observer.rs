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

    // Convert observer location to ECEF coordinates
    pub fn to_ecef(&self) -> Vector3<f64> {
        let lat_rad = self.latitude.to_radians();
        let lon_rad = self.longitude.to_radians();

        // WGS84 ellipsoid parameters
        const A: f64 = 6378137.0; // semi-major axis (meters)
        const F: f64 = 1.0 / 298.257223563; // flattening
        const E2: f64 = F * (2.0 - F); // eccentricity squared

        let n = A / (1.0 - E2 * lat_rad.sin().powi(2)).sqrt();

        let x = (n + self.altitude) * lat_rad.cos() * lon_rad.cos();
        let y = (n + self.altitude) * lat_rad.cos() * lon_rad.sin();
        let z = (n * (1.0 - E2) + self.altitude) * lat_rad.sin();

        Vector3::new(x, y, z)
    }
}
