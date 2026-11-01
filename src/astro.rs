//! Geometric sky positions: J2000 precession, sidereal time, local horizon.
use chrono::{DateTime, Utc};

#[derive(Clone, Copy, Debug)]
pub struct Horizontal {
    pub az: f64,
    pub alt: f64,
}

pub fn julian_date(time: DateTime<Utc>) -> f64 {
    time.timestamp_millis() as f64 / 86_400_000.0 + 2_440_587.5
}

pub fn sidereal(time: DateTime<Utc>, longitude: f64) -> f64 {
    let d = julian_date(time) - 2_451_545.0;
    let t = d / 36_525.0;
    (280.46061837 + 360.98564736629 * d + 0.000387933 * t * t - t * t * t / 38_710_000.0
        + longitude)
        .rem_euclid(360.0)
}

pub fn precess(ra: f64, dec: f64, time: DateTime<Utc>) -> (f64, f64) {
    let t = (julian_date(time) - 2_451_545.0) / 36_525.0;
    let zeta = ((2306.2181 * t + 0.30188 * t * t + 0.017998 * t.powi(3)) / 3600.0).to_radians();
    let z = ((2306.2181 * t + 1.09468 * t * t + 0.018203 * t.powi(3)) / 3600.0).to_radians();
    let theta = ((2004.3109 * t - 0.42665 * t * t - 0.041833 * t.powi(3)) / 3600.0).to_radians();
    let (r, d) = (ra.to_radians() + zeta, dec.to_radians());
    let a = d.cos() * r.sin();
    let b = theta.cos() * d.cos() * r.cos() - theta.sin() * d.sin();
    let c = theta.sin() * d.cos() * r.cos() + theta.cos() * d.sin();
    (
        (a.atan2(b) + z).to_degrees().rem_euclid(360.0),
        c.clamp(-1.0, 1.0).asin().to_degrees(),
    )
}

pub fn horizontal(ra: f64, dec: f64, lat: f64, lst: f64) -> Horizontal {
    let h = (lst - ra).to_radians();
    let d = dec.to_radians();
    let p = lat.to_radians();
    let east = -d.cos() * h.sin();
    let north = d.sin() * p.cos() - d.cos() * h.cos() * p.sin();
    let up = d.sin() * p.sin() + d.cos() * h.cos() * p.cos();
    Horizontal {
        az: east.atan2(north).to_degrees().rem_euclid(360.0),
        alt: up.clamp(-1.0, 1.0).asin().to_degrees(),
    }
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub az: f64,
    pub alt: f64,
    pub fov: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            az: 0.0,
            alt: 30.0,
            fov: 105.0,
        }
    }
}

impl Camera {
    pub fn turn(&mut self, az: f64, alt: f64) {
        self.az = (self.az + az).rem_euclid(360.0);
        self.alt = (self.alt + alt).clamp(-90.0, 90.0);
    }

    pub fn zoom(&mut self, factor: f64) {
        self.fov = (self.fov * factor).clamp(12.0, 130.0);
    }

    /// Perspective projection. `aspect` is physical width / height, not cell ratio.
    pub fn project(&self, point: Horizontal, aspect: f64) -> Option<(f64, f64)> {
        let a = (point.az - self.az).to_radians();
        let h = point.alt.to_radians();
        let c = self.alt.to_radians();
        let x = h.cos() * a.sin();
        let y = h.sin() * c.cos() - h.cos() * a.cos() * c.sin();
        let z = h.sin() * c.sin() + h.cos() * a.cos() * c.cos();
        if z <= 0.02 {
            return None;
        }
        let scale = (self.fov.to_radians() / 2.0).tan();
        let (x, y) = (x / z / scale, y / z / scale * aspect);
        (x.abs() <= 1.0 && y.abs() <= 1.0).then_some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_reference_sidereal_time() {
        let t = "2000-01-01T12:00:00Z".parse().unwrap();
        assert!((julian_date(t) - 2_451_545.0).abs() < 1e-8);
        assert!((sidereal(t, 0.0) - 280.46061837).abs() < 1e-7);
        let (r, d) = precess(101.287, -16.716, t);
        assert!((r - 101.287).abs() < 1e-8 && (d + 16.716).abs() < 1e-8);
    }

    #[test]
    fn meridian_and_rising_star() {
        let zenith = horizontal(50.0, -34.0, -34.0, 50.0);
        assert!((zenith.alt - 90.0).abs() < 1e-6);
        let east = horizontal(90.0, 0.0, 0.0, 0.0);
        assert!((east.az - 90.0).abs() < 1e-8 && east.alt.abs() < 1e-8);
        let west = horizontal(270.0, 0.0, 0.0, 0.0);
        assert!((west.az - 270.0).abs() < 1e-8);
    }

    #[test]
    fn celestial_pole_altitude_matches_observer_latitude() {
        let pole = horizontal(0.0, 90.0, 51.5, 123.0);
        assert!((pole.alt - 51.5).abs() < 1e-8);
        assert!(pole.az.min(360.0 - pole.az).abs() < 1e-8);
    }

    #[test]
    fn camera_wraps_and_rejects_points_behind() {
        let mut c = Camera::default();
        let center = c
            .project(
                Horizontal {
                    az: c.az,
                    alt: c.alt,
                },
                2.0,
            )
            .unwrap();
        assert!(center.0.abs() < 1e-8 && center.1.abs() < 1e-8);
        assert!(c
            .project(
                Horizontal {
                    az: 180.0,
                    alt: -30.0
                },
                2.0
            )
            .is_none());
        c.turn(-10.0, 200.0);
        assert_eq!(c.az, 350.0);
        assert_eq!(c.alt, 90.0);
    }
}
