use chrono::{Local, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};

use crate::radio::SignalStrength;
use crate::{AppState, ConfigEditMode, ConfigField, UtilityMenuStatus, TLE_SOURCES};

pub fn draw_ui(f: &mut Frame, app_state: &AppState) {
    let has_alerts = !app_state.alerts.is_empty();
    let show_radio = app_state.config.radio.enabled && app_state.config.radio.show_doppler;
    let show_sky_map = app_state.config.display.show_sky_map;

    // Main horizontal split
    let main_chunks = if show_sky_map {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Left side (info)
                Constraint::Percentage(40), // Right side (sky map + details)
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(f.area())
    };

    // Left side layout
    let mut left_constraints = vec![
        Constraint::Length(5), // Header
    ];

    if has_alerts {
        left_constraints.push(Constraint::Length(4)); // Alerts
    }

    if show_radio {
        left_constraints.push(Constraint::Length(10)); // Radio info
    }

    left_constraints.push(Constraint::Length(12)); // Real-time positions
    left_constraints.push(Constraint::Min(10)); // Pass table
    left_constraints.push(Constraint::Length(3)); // Footer

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(left_constraints)
        .split(main_chunks[0]);

    let mut chunk_idx = 0;

    // Draw header
    draw_header(f, left_chunks[chunk_idx], app_state);
    chunk_idx += 1;

    // Draw alerts if any
    if has_alerts {
        draw_alerts(f, left_chunks[chunk_idx], app_state);
        chunk_idx += 1;
    }

    // Draw radio info if enabled
    if show_radio {
        draw_radio_info(f, left_chunks[chunk_idx], app_state);
        chunk_idx += 1;
    }

    // Draw real-time positions
    draw_realtime_positions(f, left_chunks[chunk_idx], app_state);
    chunk_idx += 1;

    // Draw pass table for selected satellite
    draw_pass_table(f, left_chunks[chunk_idx], app_state);
    chunk_idx += 1;

    // Draw footer
    draw_footer(f, left_chunks[chunk_idx]);

    // Draw sky map and detailed info on right side if enabled
    // Draw sky map and detailed info on right side if enabled
    if show_sky_map {
        // Split right side vertically for sky map and detailed info
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Percentage(55), // Sky map (will be square, ~55% of height)
                Constraint::Percentage(45), // Detailed satellite info
            ])
            .split(main_chunks[1]);

        draw_sky_map(f, right_chunks[0], app_state);
        draw_satellite_details(f, right_chunks[1], app_state);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app_state: &AppState) {
    let header_text = vec![
        Line::from(vec![
            Span::styled("Observer: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} ", app_state.observer.name)),
            Span::styled("Location: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{:.4}°N, {:.4}°E, {:.0}m",
                app_state.observer.latitude,
                app_state.observer.longitude,
                app_state.observer.altitude
            )),
        ]),
        Line::from(vec![
            Span::styled("Tracking: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} satellites", app_state.satellites.len())),
            Span::raw("  "),
            Span::styled("Time: ", Style::default().fg(Color::Cyan)),
            Span::raw(Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string()),
        ]),
    ];

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Crabtrack")
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(header, area);
}

fn draw_alerts(f: &mut Frame, area: Rect, app_state: &AppState) {
    let alert_lines: Vec<Line> = app_state
        .alerts
        .iter()
        .map(|alert| {
            Line::from(vec![
                Span::styled(
                    "⚠ ALERT: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(
                    "{} pass in {} minutes (Max El: {:.1}°)",
                    alert.satellite_name, alert.time_until_minutes, alert.pass.max_elevation
                )),
            ])
        })
        .collect();

    let alerts = Paragraph::new(alert_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Upcoming pass alerts")
            .style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(alerts, area);
}

fn draw_radio_info(f: &mut Frame, area: Rect, app_state: &AppState) {
    if app_state.current_positions.is_empty() {
        return;
    }

    let selected_pos = &app_state.current_positions[app_state
        .selected_satellite
        .min(app_state.current_positions.len() - 1)];

    let mut info_lines = vec![Line::from(vec![
        Span::styled("Satellite: ", Style::default().fg(Color::Cyan)),
        Span::raw(&selected_pos.name),
    ])];

    if let Some(doppler) = &selected_pos.doppler {
        info_lines.push(Line::from(vec![
            Span::styled("Downlink: ", Style::default().fg(Color::Green)),
            Span::raw(format!(
                "{:.6} MHz (shift: {:+.0} Hz)",
                doppler.downlink_observed_mhz, doppler.downlink_shift_hz
            )),
        ]));

        info_lines.push(Line::from(vec![
            Span::styled("Uplink:   ", Style::default().fg(Color::Yellow)),
            Span::raw(format!(
                "{:.6} MHz (correct to: {:.6} MHz)",
                doppler.uplink_frequency_mhz, doppler.uplink_corrected_mhz
            )),
        ]));
    }

    if let Some(comm) = &selected_pos.comm_window {
        let status_color = if comm.is_viable {
            Color::Green
        } else {
            Color::Red
        };
        let signal_color = match comm.signal_strength_estimate {
            SignalStrength::Excellent => Color::Green,
            SignalStrength::Good => Color::LightGreen,
            SignalStrength::Fair => Color::Yellow,
            SignalStrength::Poor => Color::LightRed,
            SignalStrength::NoSignal => Color::Red,
        };

        info_lines.push(Line::from(vec![
            Span::styled("Status:   ", Style::default().fg(Color::Cyan)),
            Span::styled(
                if comm.is_viable {
                    "VIABLE"
                } else {
                    "NOT VIABLE"
                },
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Signal: "),
            Span::styled(
                comm.signal_strength_estimate.as_str(),
                Style::default().fg(signal_color),
            ),
        ]));

        if let Some(mode) = &comm.recommended_mode {
            info_lines.push(Line::from(vec![
                Span::styled("Mode:     ", Style::default().fg(Color::Cyan)),
                Span::raw(mode),
            ]));
        }

        info_lines.push(Line::from(vec![
            Span::styled("Info:     ", Style::default().fg(Color::Gray)),
            Span::raw(&comm.reason),
        ]));
    }

    let radio_info = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Satcomm")
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(radio_info, area);
}

fn draw_realtime_positions(f: &mut Frame, area: Rect, app_state: &AppState) {
    if !app_state.config.display.show_current_position {
        return;
    }

    let header_cells = [
        "Satellite",
        "Lat",
        "Lon",
        "Alt",
        "Vel",
        "Az",
        "El",
        "Range",
        "Status",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let positions_to_show = if app_state.config.display.show_all_positions {
        &app_state.current_positions
    } else {
        &app_state.current_positions[app_state.selected_satellite..=app_state.selected_satellite]
    };

    let rows = positions_to_show.iter().enumerate().map(|(_idx, pos)| {
        let status = if pos.is_visible {
            ("VISIBLE", Color::Green)
        } else {
            ("BELOW HORIZON", Color::Gray)
        };

        let is_selected = if !app_state.config.display.show_all_positions {
            true
        } else {
            app_state
                .current_positions
                .iter()
                .position(|p| p.name == pos.name)
                .map_or(false, |idx| idx == app_state.selected_satellite)
        };

        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let cells = vec![
            Cell::from(pos.name.clone()).style(style),
            Cell::from(format!("{:.2}°", pos.latitude)),
            Cell::from(format!("{:.2}°", pos.longitude)),
            Cell::from(format!("{:.0} km", pos.altitude_km)),
            Cell::from(format!("{:.2} km/s", pos.velocity_km_s)),
            Cell::from(format!("{:.0}°", pos.azimuth)),
            Cell::from(format!("{:.1}°", pos.elevation)),
            Cell::from(format!("{:.0} km", pos.range_km)),
            Cell::from(status.0).style(Style::default().fg(status.1)),
        ];

        Row::new(cells).height(1).style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(14),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Real-time satellite positions")
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(table, area);
}

fn draw_pass_table(f: &mut Frame, area: Rect, app_state: &AppState) {
    let selected_satellite = &app_state.satellites[app_state.selected_satellite];
    let passes = &selected_satellite.passes;

    let header_cells = [
        "#", "AOS Time", "Max Time", "LOS Time", "Duration", "Max El", "AOS Az", "Max Az",
        "LOS Az", "Range",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let now = Utc::now();
    let rows = passes.iter().enumerate().map(|(i, pass)| {
        let is_upcoming = pass.aos_time > now;
        let is_current = pass.aos_time <= now && pass.los_time >= now;
        let is_alerting = app_state.config.alerts.enabled
            && pass.max_elevation >= app_state.config.alerts.min_elevation_for_alert
            && is_upcoming;

        let style = if is_current {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else if is_alerting {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_upcoming {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let cells = vec![
            Cell::from(format!("{}", i + 1)),
            Cell::from(
                pass.aos_time
                    .with_timezone(&Local)
                    .format("%m/%d %H:%M")
                    .to_string(),
            ),
            Cell::from(
                pass.max_elevation_time
                    .with_timezone(&Local)
                    .format("%H:%M:%S")
                    .to_string(),
            ),
            Cell::from(
                pass.los_time
                    .with_timezone(&Local)
                    .format("%m/%d %H:%M")
                    .to_string(),
            ),
            Cell::from(format!("{:.1} min", pass.duration_minutes())),
            Cell::from(format!("{:.1}°", pass.max_elevation)),
            Cell::from(format!("{:.0}°", pass.aos_azimuth)),
            Cell::from(format!("{:.0}°", pass.max_azimuth)),
            Cell::from(format!("{:.0}°", pass.los_azimuth)),
            Cell::from(format!("{:.0} km", pass.max_range_km)),
        ];

        Row::new(cells).height(1).style(style)
    });

    let next_pass_info = selected_satellite
        .get_next_pass()
        .map(|pass| {
            let time_until = (pass.aos_time - now).num_minutes();
            if time_until > 60 {
                format!(" (Next pass in {}h {}m)", time_until / 60, time_until % 60)
            } else {
                format!(" (Next pass in {}m)", time_until)
            }
        })
        .unwrap_or_else(|| " (No upcoming passes)".to_string());

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Passes for: {}{}",
                selected_satellite.name, next_pass_info
            ))
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(table, area);
}

fn draw_sky_map(f: &mut Frame, area: Rect, app_state: &AppState) {
    use ratatui::symbols;
    use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine};

    // Use the full width, but limit height to make it square
    // Account for borders (2 chars on each side)
    let available_width = area.width.saturating_sub(2);
    let available_height = area.height.saturating_sub(2);

    // Make it square based on available space
    // Prefer width since we have more horizontal space
    let size = available_width.min(available_height);

    // Center horizontally if needed
    let _x_offset = (available_width.saturating_sub(size)) / 2;

    // For the canvas area (the actual square drawing area)
    let canvas_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: size + 2, // Add back the border space
    };

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Sky map (polar view)")
                .style(Style::default().fg(Color::White)),
        )
        .x_bounds([-1.2, 1.2])
        .y_bounds([-1.2, 1.2])
        .marker(symbols::Marker::Braille)
        .paint(|ctx| {
            // Draw horizon circle (outer edge)
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 1.0,
                color: Color::White,
            });

            // Draw elevation circles (30°, 60°)
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 0.667, // 60° elevation
                color: Color::DarkGray,
            });

            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 0.333, // 30° elevation
                color: Color::DarkGray,
            });

            // Draw cardinal direction lines
            // North (top)
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: 0.0,
                y2: 1.0,
                color: Color::Gray,
            });

            // East (right)
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: 1.0,
                y2: 0.0,
                color: Color::Gray,
            });

            // South (bottom)
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: 0.0,
                y2: -1.0,
                color: Color::Gray,
            });

            // West (left)
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: -1.0,
                y2: 0.0,
                color: Color::Gray,
            });

            // Draw satellites
            for (idx, pos) in app_state.current_positions.iter().enumerate() {
                if !pos.is_visible {
                    continue; // Skip satellites below horizon
                }

                // Convert azimuth/elevation to x,y coordinates
                // Azimuth: 0° = North, 90° = East, 180° = South, 270° = West
                // Elevation: 0° = horizon (r=1.0), 90° = zenith (r=0.0)

                let azimuth_rad = pos.azimuth.to_radians();

                // Radius on map: 0 at zenith (90°), 1 at horizon (0°)
                let radius = (90.0 - pos.elevation) / 90.0;

                // Convert to cartesian (rotate so North is up)
                // Azimuth 0° (North) should point up (negative y)
                let x = radius * azimuth_rad.sin();
                let y = -radius * azimuth_rad.cos();

                // Determine color based on selection and signal
                let color = if idx == app_state.selected_satellite {
                    Color::Cyan
                } else if pos.elevation > 45.0 {
                    Color::Green
                } else if pos.elevation > 20.0 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                // Draw satellite as a circle
                ctx.draw(&Circle {
                    x,
                    y,
                    radius: 0.05,
                    color,
                });

                // Draw satellite marker
                ctx.print(x, y, "●");
            }

            // Draw labels for cardinal directions
            ctx.print(0.0, 1.05, "N");
            ctx.print(1.05, 0.0, "E");
            ctx.print(0.0, -1.05, "S");
            ctx.print(-1.05, 0.0, "W");

            // Draw zenith marker
            ctx.print(0.0, 0.0, "+");
        });

    f.render_widget(canvas, canvas_area);

    // Draw legend below the map
    let legend_y = canvas_area.y + canvas_area.height;
    if legend_y + 7 <= area.y + area.height {
        let legend_area = Rect {
            x: area.x + 2,
            y: legend_y,
            width: area.width.saturating_sub(4),
            height: (area.y + area.height).saturating_sub(legend_y).min(7),
        };

        let mut legend_lines = vec![Line::from(vec![
            Span::styled(
                "Legend: ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("● ", Style::default().fg(Color::Cyan)),
            Span::raw("Selected  "),
            Span::styled("● ", Style::default().fg(Color::Green)),
            Span::raw("El>45°  "),
            Span::styled("● ", Style::default().fg(Color::Yellow)),
            Span::raw("El>20°  "),
            Span::styled("● ", Style::default().fg(Color::Red)),
            Span::raw("El<20°"),
        ])];

        // Add satellite names (limit to available space)
        let max_sat_lines = legend_area.height.saturating_sub(1).min(4) as usize;
        for (idx, pos) in app_state
            .current_positions
            .iter()
            .enumerate()
            .take(max_sat_lines)
        {
            if pos.is_visible {
                let color = if idx == app_state.selected_satellite {
                    Color::Cyan
                } else {
                    Color::White
                };

                legend_lines.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(color)),
                    Span::raw(format!(
                        "{} ({:.0}°/{:.0}°)",
                        pos.name.chars().take(10).collect::<String>(),
                        pos.azimuth,
                        pos.elevation
                    )),
                ]));
            }
        }

        let legend = Paragraph::new(legend_lines).style(Style::default().fg(Color::White));

        f.render_widget(legend, legend_area);
    }
}

fn draw_satellite_details(f: &mut Frame, area: Rect, app_state: &AppState) {
    if app_state.current_positions.is_empty() || app_state.satellites.is_empty() {
        let empty = Paragraph::new("No satellite data available").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Satellite details")
                .style(Style::default().fg(Color::White)),
        );
        f.render_widget(empty, area);
        return;
    }

    let selected_satellite = &app_state.satellites[app_state.selected_satellite];
    let selected_pos = &app_state.current_positions[app_state
        .selected_satellite
        .min(app_state.current_positions.len() - 1)];

    let mut detail_lines = vec![
        Line::from(vec![
            Span::styled(
                "Satellite: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&selected_pos.name),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Position:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw("  Latitude:  "),
            Span::styled(
                format!("{:.4}°", selected_pos.latitude),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Longitude: "),
            Span::styled(
                format!("{:.4}°", selected_pos.longitude),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Altitude:  "),
            Span::styled(
                format!("{:.2} km", selected_pos.altitude_km),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Velocity:  "),
            Span::styled(
                format!("{:.2} km/s", selected_pos.velocity_km_s),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Observer View:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw("  Azimuth:   "),
            Span::styled(
                format!("{:.1}°", selected_pos.azimuth),
                Style::default().fg(Color::White),
            ),
            Span::raw(format!(" ({})", azimuth_to_cardinal(selected_pos.azimuth))),
        ]),
        Line::from(vec![
            Span::raw("  Elevation: "),
            Span::styled(
                format!("{:.1}°", selected_pos.elevation),
                Style::default().fg(if selected_pos.elevation > 45.0 {
                    Color::Green
                } else if selected_pos.elevation > 20.0 {
                    Color::Yellow
                } else if selected_pos.elevation > 0.0 {
                    Color::Red
                } else {
                    Color::Gray
                }),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Range:     "),
            Span::styled(
                format!("{:.1} km", selected_pos.range_km),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Status:    "),
            Span::styled(
                if selected_pos.is_visible {
                    "VISIBLE"
                } else {
                    "BELOW HORIZON"
                },
                Style::default().fg(if selected_pos.is_visible {
                    Color::Green
                } else {
                    Color::Gray
                }),
            ),
        ]),
    ];

    // Add next pass info
    if let Some(next_pass) = selected_satellite.get_next_pass() {
        let now = Utc::now();
        let time_until = next_pass.aos_time.signed_duration_since(now);
        let minutes_until = time_until.num_minutes();

        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![Span::styled(
            "Next Pass:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        if minutes_until > 60 {
            detail_lines.push(Line::from(vec![
                Span::raw("  In:        "),
                Span::styled(
                    format!("{}h {}m", minutes_until / 60, minutes_until % 60),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        } else {
            detail_lines.push(Line::from(vec![
                Span::raw("  In:        "),
                Span::styled(
                    format!("{} minutes", minutes_until),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        detail_lines.push(Line::from(vec![
            Span::raw("  Max El:    "),
            Span::styled(
                format!("{:.1}°", next_pass.max_elevation),
                Style::default().fg(Color::White),
            ),
        ]));

        detail_lines.push(Line::from(vec![
            Span::raw("  Duration:  "),
            Span::styled(
                format!("{:.1} min", next_pass.duration_minutes()),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    let details = Paragraph::new(detail_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Satellite details")
            .style(Style::default().fg(Color::White)),
    );

    f.render_widget(details, area);
}

fn azimuth_to_cardinal(azimuth: f64) -> &'static str {
    let az = azimuth % 360.0;
    match az {
        a if a >= 337.5 || a < 22.5 => "N",
        a if a >= 22.5 && a < 67.5 => "NE",
        a if a >= 67.5 && a < 112.5 => "E",
        a if a >= 112.5 && a < 157.5 => "SE",
        a if a >= 157.5 && a < 202.5 => "S",
        a if a >= 202.5 && a < 247.5 => "SW",
        a if a >= 247.5 && a < 292.5 => "W",
        a if a >= 292.5 && a < 337.5 => "NW",
        _ => "?",
    }
}

fn draw_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(
        "↑/↓ or j/k: Select | c: Config | u: Utilities | q/ESC: Quit | Home/End: First/Last",
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, area);
}

/// Draw the satellite configuration screen
pub fn draw_satellite_config(f: &mut Frame, app_state: &AppState) {
    let state = &app_state.sat_config_state;

    // Create centered area for the config window
    let area = centered_rect(90, 90, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    match state.edit_mode {
        ConfigEditMode::List => {
            draw_satellite_list(f, area, app_state);
        }
        ConfigEditMode::Edit | ConfigEditMode::Add => {
            draw_satellite_edit_form(f, area, app_state);
        }
    }
}

/// Draw the satellite list view
fn draw_satellite_list(f: &mut Frame, area: Rect, app_state: &AppState) {
    let state = &app_state.sat_config_state;

    // Split into header, content, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Status
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Satellite Configuration",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" ({} satellites)", state.satellites.len())),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White)),
    );
    f.render_widget(header, chunks[0]);

    // Satellite list
    if state.satellites.is_empty() {
        let empty_msg = Paragraph::new("No satellites configured. Press 'a' to add a new satellite.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Satellites")
                    .style(Style::default().fg(Color::White)),
            );
        f.render_widget(empty_msg, chunks[1]);
    } else {
        let header_cells = [
            "Name",
            "Type",
            "Country",
            "Operator",
            "Downlink",
            "Uplink",
        ]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

        let header_row = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = state.satellites.iter().enumerate().map(|(idx, sat)| {
            let is_selected = idx == state.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let cells = vec![
                Cell::from(truncate_string(&sat.name, 20)),
                Cell::from(truncate_string(
                    sat.satellite_type.as_deref().unwrap_or("-"),
                    15,
                )),
                Cell::from(truncate_string(
                    sat.country_of_origin.as_deref().unwrap_or("-"),
                    12,
                )),
                Cell::from(truncate_string(
                    sat.operator.as_deref().unwrap_or("-"),
                    15,
                )),
                Cell::from(
                    sat.downlink_frequency_mhz
                        .map(|f| format!("{:.3}", f))
                        .unwrap_or_else(|| "-".to_string()),
                ),
                Cell::from(
                    sat.uplink_frequency_mhz
                        .map(|f| format!("{:.3}", f))
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ];

            Row::new(cells).height(1).style(style)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(22),
                Constraint::Length(17),
                Constraint::Length(14),
                Constraint::Length(17),
                Constraint::Length(12),
                Constraint::Length(12),
            ],
        )
        .header(header_row)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Satellites")
                .style(Style::default().fg(Color::White)),
        );

        f.render_widget(table, chunks[1]);
    }

    // Status message
    let status_text = state
        .status_message
        .as_deref()
        .unwrap_or("");
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);

    // Footer with keybindings
    let footer = Paragraph::new(
        "a: Add | e/Enter: Edit | d/Del: Delete | ↑/↓: Navigate | q/ESC: Back",
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[3]);
}

/// Draw the edit form for satellite details
fn draw_satellite_edit_form(f: &mut Frame, area: Rect, app_state: &AppState) {
    let state = &app_state.sat_config_state;

    let title = if state.edit_mode == ConfigEditMode::Add {
        "Add New Satellite"
    } else {
        "Edit Satellite"
    };

    // Split into header, form fields, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(15),    // Form fields
            Constraint::Length(3),  // Status
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White)),
        );
    f.render_widget(header, chunks[0]);

    // Form fields
    let fields = [
        ConfigField::Name,
        ConfigField::TleLine1,
        ConfigField::TleLine2,
        ConfigField::LaunchDate,
        ConfigField::LaunchSite,
        ConfigField::CountryOfOrigin,
        ConfigField::Operator,
        ConfigField::SatelliteType,
        ConfigField::DownlinkFrequency,
        ConfigField::UplinkFrequency,
        ConfigField::Notes,
    ];

    let field_lines: Vec<Line> = fields
        .iter()
        .map(|field| {
            let is_current = *field == state.current_field;
            let value = if is_current {
                format!("{}|", state.input_buffer)
            } else {
                state.get_field_value(*field)
            };

            let label_style = if is_current {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let value_style = if is_current {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let indicator = if is_current { "> " } else { "  " };

            Line::from(vec![
                Span::styled(indicator, label_style),
                Span::styled(format!("{:16}", field.label()), label_style),
                Span::raw(": "),
                Span::styled(truncate_string(&value, 55), value_style),
            ])
        })
        .collect();

    let form = Paragraph::new(field_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Fields (Tab/↑↓ to navigate)")
            .style(Style::default().fg(Color::White)),
    );
    f.render_widget(form, chunks[1]);

    // Status message
    let status_text = state.status_message.as_deref().unwrap_or("");
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);

    // Footer
    let footer = Paragraph::new("Tab/↑↓: Next/Prev field | Enter: Save | ESC: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[3]);
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Helper function to truncate strings for display
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Draw the utility menu for TLE downloads
pub fn draw_utility_menu(f: &mut Frame, app_state: &AppState) {
    let state = &app_state.utility_menu_state;

    // Create centered area for the menu (60% width, 70% height)
    let area = centered_rect(60, 70, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    // Split into header, content, status, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Content (TLE source list)
            Constraint::Length(3),  // Status message
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Download TLE Data",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White)),
    );
    f.render_widget(header, chunks[0]);

    // TLE Source List
    let header_cells = ["Source", "Description"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header_row = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = TLE_SOURCES.iter().enumerate().map(|(idx, source)| {
        let is_selected = idx == state.selected_index;
        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let indicator = if is_selected { "> " } else { "  " };

        let cells = vec![
            Cell::from(format!("{}{}", indicator, source.name)),
            Cell::from(source.description),
        ];

        Row::new(cells).height(1).style(style)
    });

    let table = Table::new(
        rows,
        [Constraint::Length(25), Constraint::Min(30)],
    )
    .header(header_row)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Celestrak TLE Sources")
            .style(Style::default().fg(Color::White)),
    );
    f.render_widget(table, chunks[1]);

    // Status message
    let (status_text, status_color) = match state.status {
        UtilityMenuStatus::Browsing => {
            ("Select a source and press Enter to download".to_string(), Color::Gray)
        }
        UtilityMenuStatus::Downloading => {
            (state.status_message.clone().unwrap_or_default(), Color::Yellow)
        }
        UtilityMenuStatus::Success => {
            (state.status_message.clone().unwrap_or_default(), Color::Green)
        }
        UtilityMenuStatus::Error => {
            (state.status_message.clone().unwrap_or_default(), Color::Red)
        }
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);

    // Footer
    let footer_text = match state.status {
        UtilityMenuStatus::Browsing => "Enter: Download | j/k/↑↓: Navigate | q/ESC: Close",
        UtilityMenuStatus::Success | UtilityMenuStatus::Error => "Press any key to continue",
        UtilityMenuStatus::Downloading => "Please wait...",
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[3]);
}
