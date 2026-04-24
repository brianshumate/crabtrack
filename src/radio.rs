use crate::satellite::SatellitePosition;

const SPEED_OF_LIGHT: f64 = 299792458.0; // m/s

#[derive(Debug, Clone)]
pub struct DopplerShift {
    #[allow(dead_code)]
    pub downlink_frequency_mhz: f64,
    pub downlink_shift_hz: f64,
    pub downlink_observed_mhz: f64,
    pub uplink_frequency_mhz: f64,
    #[allow(dead_code)]
    pub uplink_shift_hz: f64,
    pub uplink_corrected_mhz: f64,
}

#[derive(Debug, Clone)]
pub struct CommunicationWindow {
    pub is_viable: bool,
    pub reason: String,
    pub signal_strength_estimate: SignalStrength,
    pub recommended_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalStrength {
    Excellent,
    Good,
    Fair,
    Poor,
    NoSignal,
}

impl SignalStrength {
    pub fn as_str(&self) -> &str {
        match self {
            SignalStrength::Excellent => "Excellent",
            SignalStrength::Good => "Good",
            SignalStrength::Fair => "Fair",
            SignalStrength::Poor => "Poor",
            SignalStrength::NoSignal => "No Signal",
        }
    }
}

pub fn calculate_doppler_shift(
    position: &SatellitePosition,
    downlink_freq_mhz: f64,
    uplink_freq_mhz: f64,
) -> DopplerShift {
    // Calculate radial velocity using velocity vectors and range
    // Positive radial velocity = moving away (red shift)
    // Negative radial velocity = moving toward (blue shift)

    // Get the velocity components in km/s
let _vx = position.velocity_x;
    let _vy = position.velocity_y;
    let _vz = position.velocity_z;
    let sat_velocity_ms = position.velocity_km_s * 1000.0;

    let elevation_rad = position.elevation.to_radians();
    let _azimuth_rad = position.azimuth.to_radians();

    // Approximate radial velocity using velocity magnitude and direction
    // The elevation angle determines how much of the velocity is toward/away from observer
    // At horizon (el=0): all horizontal velocity is tangential, radial is small
    // At zenith (el=90): velocity is mostly radial or perpendicular
    let radial_velocity = sat_velocity_ms * elevation_rad.cos() * 0.7; // Rough approximation with directional factor

    // Simple sign based on elevation change direction (approaching vs receding)
    // When satellite is low and rising, it's approaching
    // When satellite is high and setting, it's receding
    let radial_sign = if position.elevation < 45.0 { -1.0 } else { 1.0 };
    let radial_velocity = radial_velocity * radial_sign;

    let downlink_shift_hz = -(radial_velocity / SPEED_OF_LIGHT) * (downlink_freq_mhz * 1_000_000.0);
    let downlink_observed_mhz = downlink_freq_mhz + (downlink_shift_hz / 1_000_000.0);

    let uplink_shift_hz = (radial_velocity / SPEED_OF_LIGHT) * (uplink_freq_mhz * 1_000_000.0);
    let uplink_corrected_mhz = uplink_freq_mhz + (uplink_shift_hz / 1_000_000.0);

    DopplerShift {
        downlink_frequency_mhz: downlink_freq_mhz,
        downlink_shift_hz,
        downlink_observed_mhz,
        uplink_frequency_mhz: uplink_freq_mhz,
        uplink_shift_hz,
        uplink_corrected_mhz,
    }
}

pub fn evaluate_communication_window(position: &SatellitePosition) -> CommunicationWindow {
    if !position.is_visible {
        return CommunicationWindow {
            is_viable: false,
            reason: "Satellite below horizon".to_string(),
            signal_strength_estimate: SignalStrength::NoSignal,
            recommended_mode: None,
        };
    }

    let elevation = position.elevation;
    let range_km = position.range_km;

    // Evaluate signal strength based on elevation and range
    let signal_strength = if elevation >= 45.0 && range_km < 2000.0 {
        SignalStrength::Excellent
    } else if elevation >= 30.0 && range_km < 2500.0 {
        SignalStrength::Good
    } else if elevation >= 15.0 && range_km < 3000.0 {
        SignalStrength::Fair
    } else if elevation >= 5.0 {
        SignalStrength::Poor
    } else {
        SignalStrength::NoSignal
    };

    // Determine if communication is viable
    let is_viable = elevation >= 10.0 && signal_strength != SignalStrength::NoSignal;

    // Recommend mode based on conditions
    let recommended_mode = if elevation >= 30.0 {
        Some("FM/SSB".to_string())
    } else if elevation >= 15.0 {
        Some("SSB".to_string())
    } else if elevation >= 10.0 {
        Some("SSB (difficult)".to_string())
    } else {
        None
    };

    let reason = if is_viable {
        format!(
            "Good pass - El: {:.1}°, Range: {:.0}km",
            elevation, range_km
        )
    } else {
        format!("Elevation too low ({:.1}°) for reliable contact", elevation)
    };

    CommunicationWindow {
        is_viable,
        reason,
        signal_strength_estimate: signal_strength,
        recommended_mode,
    }
}
