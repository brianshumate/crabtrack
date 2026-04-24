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
    // Radial velocity from satellite.rs: positive = receding (redshift), negative = approaching (blueshift)
    let radial_velocity = position.range_rate_km_s * 1000.0; // convert km/s to m/s

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
