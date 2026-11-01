//! Offline ephemerides from Astronomy Engine (MIT), statically linked.
//!
//! Equator supplies topocentric coordinates of date, including lunar parallax.
//! Feed them directly to Horizon: precessing them again as J2000 would be wrong.
use crate::astro::{julian_date, Horizontal};
use crate::i18n::Language;
use astronomy_engine_bindings as ae;
use chrono::{DateTime, Utc};
use std::sync::Mutex;

// The C engine has shared caches. Serialize all access, including parallel tests.
static ENGINE: Mutex<()> = Mutex::new(());
pub const AU_KM: f64 = 149_597_870.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

pub const BODIES: [Body; 9] = [
    Body::Sun,
    Body::Moon,
    Body::Mercury,
    Body::Venus,
    Body::Mars,
    Body::Jupiter,
    Body::Saturn,
    Body::Uranus,
    Body::Neptune,
];

impl Body {
    pub fn name(self, lang: Language) -> &'static str {
        match self {
            Self::Sun => lang.pick("Sun", "Sol"),
            Self::Moon => lang.pick("Moon", "Luna"),
            Self::Mercury => lang.pick("Mercury", "Mercurio"),
            Self::Venus => "Venus",
            Self::Mars => lang.pick("Mars", "Marte"),
            Self::Jupiter => lang.pick("Jupiter", "Júpiter"),
            Self::Saturn => lang.pick("Saturn", "Saturno"),
            Self::Uranus => lang.pick("Uranus", "Urano"),
            Self::Neptune => lang.pick("Neptune", "Neptuno"),
        }
    }

    pub fn english(self) -> &'static str {
        match self {
            Self::Sun => "sun",
            Self::Moon => "moon",
            Self::Mercury => "mercury",
            Self::Venus => "venus",
            Self::Mars => "mars",
            Self::Jupiter => "jupiter",
            Self::Saturn => "saturn",
            Self::Uranus => "uranus",
            Self::Neptune => "neptune",
        }
    }

    fn engine(self) -> ae::astro_body_t {
        match self {
            Self::Sun => ae::astro_body_t_BODY_SUN,
            Self::Moon => ae::astro_body_t_BODY_MOON,
            Self::Mercury => ae::astro_body_t_BODY_MERCURY,
            Self::Venus => ae::astro_body_t_BODY_VENUS,
            Self::Mars => ae::astro_body_t_BODY_MARS,
            Self::Jupiter => ae::astro_body_t_BODY_JUPITER,
            Self::Saturn => ae::astro_body_t_BODY_SATURN,
            Self::Uranus => ae::astro_body_t_BODY_URANUS,
            Self::Neptune => ae::astro_body_t_BODY_NEPTUNE,
        }
    }
}

pub struct SolarObject {
    pub body: Body,
    pub position: Horizontal,
    /// Topocentric equator of date, degrees (engine RA is converted from hours).
    pub ra: f64,
    pub dec: f64,
    pub distance_au: f64,
    pub magnitude: f64,
    /// Geocentric illuminated fraction, not a fraction of the synodic month.
    pub illuminated: f64,
    /// Geocentric ecliptic elongation: 0=new, 90=first quarter, 180=full.
    pub lunar_phase: Option<f64>,
}

impl SolarObject {
    pub fn phase_name(&self, lang: Language) -> Option<&'static str> {
        self.lunar_phase.map(|angle| {
            match ((angle.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize % 8 {
                0 => lang.pick("New Moon", "Luna nueva"),
                1 => lang.pick("Waxing crescent", "Creciente"),
                2 => lang.pick("First quarter", "Cuarto creciente"),
                3 => lang.pick("Waxing gibbous", "Gibosa creciente"),
                4 => lang.pick("Full Moon", "Luna llena"),
                5 => lang.pick("Waning gibbous", "Gibosa menguante"),
                6 => lang.pick("Last quarter", "Cuarto menguante"),
                _ => lang.pick("Waning crescent", "Menguante"),
            }
        })
    }

    pub fn symbol(&self, ascii: bool) -> &'static str {
        if ascii {
            return match self.body {
                Body::Sun => "@",
                Body::Moon => "O",
                _ => "o",
            };
        }
        match self.body {
            Body::Sun => "☉",
            Body::Moon => match self.lunar_phase.unwrap_or(0.0) {
                a if !(22.5..337.5).contains(&a) => "○",
                a if a < 157.5 => "◐",
                a if a < 202.5 => "●",
                _ => "◑",
            },
            _ => "◆",
        }
    }
}

pub fn daylight(sun_altitude: f64, lang: Language) -> &'static str {
    if sun_altitude >= 0.0 {
        lang.pick("DAY", "DÍA")
    } else if sun_altitude >= -6.0 {
        lang.pick("CIVIL TWILIGHT", "CREPÚSCULO CIVIL")
    } else if sun_altitude >= -12.0 {
        lang.pick("NAUTICAL TWILIGHT", "CREPÚSCULO NÁUTICO")
    } else if sun_altitude >= -18.0 {
        lang.pick("ASTRONOMICAL TWILIGHT", "CREPÚSCULO ASTRONÓMICO")
    } else {
        lang.pick("NIGHT", "NOCHE")
    }
}

pub fn calculate(time: DateTime<Utc>, lat: f64, lon: f64) -> Result<Vec<SolarObject>, String> {
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err("Invalid ephemeris coordinates".into());
    }
    let _guard = ENGINE.lock().map_err(|_| "Ephemeris engine unavailable")?;
    // SAFETY: all C calls are serialized. The only pointers are to a live local
    // astro_time_t. Bodies are supported enum values, coordinates are validated,
    // and status codes/finite results are checked before exposing them to the UI.
    unsafe {
        let mut t = ae::Astronomy_TimeFromDays(julian_date(time) - 2_451_545.0);
        let observer = ae::Astronomy_MakeObserver(lat, lon, 0.0);
        let phase = ae::Astronomy_MoonPhase(t);
        if phase.status != ae::astro_status_t_ASTRO_SUCCESS || !phase.angle.is_finite() {
            return Err("Could not calculate lunar phase".into());
        }
        BODIES
            .iter()
            .map(|&body| {
                let eq = ae::Astronomy_Equator(
                    body.engine(),
                    &mut t,
                    observer,
                    ae::astro_equator_date_t_EQUATOR_OF_DATE,
                    ae::astro_aberration_t_ABERRATION,
                );
                let light = ae::Astronomy_Illumination(body.engine(), t);
                if eq.status != ae::astro_status_t_ASTRO_SUCCESS
                    || light.status != ae::astro_status_t_ASTRO_SUCCESS
                {
                    return Err(format!("Could not calculate {}", body.name(Language::En)));
                }
                let horizon = ae::Astronomy_Horizon(
                    &mut t,
                    observer,
                    eq.ra,
                    eq.dec,
                    ae::astro_refraction_t_REFRACTION_NONE,
                );
                if ![
                    eq.ra,
                    eq.dec,
                    eq.dist,
                    horizon.azimuth,
                    horizon.altitude,
                    light.mag,
                    light.phase_fraction,
                ]
                .iter()
                .all(|v| v.is_finite())
                {
                    return Err(format!("Invalid ephemerides: {}", body.name(Language::En)));
                }
                Ok(SolarObject {
                    body,
                    position: Horizontal {
                        az: horizon.azimuth,
                        alt: horizon.altitude,
                    },
                    ra: eq.ra * 15.0,
                    dec: eq.dec,
                    distance_au: eq.dist,
                    magnitude: light.mag,
                    illuminated: light.phase_fraction,
                    lunar_phase: (body == Body::Moon).then_some(phase.angle),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topocentric_positions_match_jpl_horizons() {
        // Independent NASA/JPL Horizons observer tables, retrieved 2026-11-01.
        // UTC 2026-01-01 00:00, coord@399, GEODETIC 0,0,0, AIRLESS,
        // QUANTITIES=4,9,10,20. See data/ephemeris-validation.md.
        let objects = calculate("2026-01-01T00:00:00Z".parse().unwrap(), 0.0, 0.0).unwrap();
        for (body, az, alt, distance) in [
            (Body::Sun, 181.957796, -66.969505, 0.98336589953355),
            (Body::Moon, 309.690817, 45.154603, 0.00238302142621),
            (Body::Jupiter, 28.131672, 64.886725, 4.24262710885954),
        ] {
            let o = objects.iter().find(|o| o.body == body).unwrap();
            assert!(
                (o.position.az - az).abs() < 0.05,
                "{body:?}: az={}",
                o.position.az
            );
            assert!(
                (o.position.alt - alt).abs() < 0.05,
                "{body:?}: alt={}",
                o.position.alt
            );
            assert!(
                (o.distance_au - distance).abs() / distance < 0.001,
                "{body:?}: distance={}",
                o.distance_au
            );
        }
    }

    #[test]
    fn lunar_phase_distinguishes_new_full_and_waxing() {
        for (date, full) in [
            ("2024-04-08T18:21:00Z", false),
            ("2024-03-25T07:00:00Z", true),
        ] {
            let objects = calculate(date.parse().unwrap(), 0.0, 0.0).unwrap();
            let moon = objects.iter().find(|o| o.body == Body::Moon).unwrap();
            assert!((moon.illuminated - if full { 1.0 } else { 0.0 }).abs() < 0.001);
            assert_eq!(
                moon.phase_name(Language::Es),
                Some(if full { "Luna llena" } else { "Luna nueva" })
            );
        }
        let objects = calculate("2026-01-01T00:00:00Z".parse().unwrap(), 0.0, 0.0).unwrap();
        let moon = objects.iter().find(|o| o.body == Body::Moon).unwrap();
        assert_eq!(moon.phase_name(Language::Es), Some("Gibosa creciente"));
        assert!((moon.illuminated - 0.911).abs() < 0.01);
    }

    #[test]
    fn all_bodies_are_finite_at_date_and_location_extremes() {
        for date in ["1900-01-01T00:00:00Z", "2100-12-31T23:59:59Z"] {
            for (lat, lon) in [(-90.0, -180.0), (90.0, 180.0)] {
                let objects = calculate(date.parse().unwrap(), lat, lon).unwrap();
                assert_eq!(objects.len(), 9);
                assert!(objects.iter().all(|o| o.distance_au > 0.0
                    && (0.0..=1.0).contains(&o.illuminated)
                    && (-90.0..=90.0).contains(&o.position.alt)));
            }
        }
    }

    #[test]
    fn daylight_uses_geometric_solar_center() {
        for (alt, label) in [
            (0.0, "DÍA"),
            (-0.1, "CREPÚSCULO CIVIL"),
            (-6.1, "CREPÚSCULO NÁUTICO"),
            (-12.1, "CREPÚSCULO ASTRONÓMICO"),
            (-18.1, "NOCHE"),
        ] {
            assert_eq!(daylight(alt, Language::Es), label);
        }
    }
}
