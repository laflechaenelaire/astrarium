use crate::astro::{self, Camera, Horizontal};
use crate::i18n::Language;
use crate::labels::LabelLayout;
use crate::solar::{self, Body, SolarObject};
use chrono::{DateTime, Duration, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Deserialize)]
pub struct Star {
    pub id: u32,
    pub name: String,
    pub named: bool,
    pub con: String,
    pub ra: f64,
    pub dec: f64,
    pub mag: f64,
}

#[derive(Deserialize)]
pub struct Constellation {
    pub name: String,
    pub lines: Vec<Vec<[f64; 2]>>,
}

#[derive(Deserialize)]
pub struct Catalog {
    pub stars: Vec<Star>,
    pub constellations: Vec<Constellation>,
}

impl Catalog {
    pub fn load() -> Self {
        serde_json::from_str(include_str!("../data/sky.json")).expect("invalid embedded catalogue")
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            name: "Greenwich".into(),
            lat: 51.4779,
            lon: 0.0,
        }
    }
}

impl Location {
    pub fn parse(input: &str) -> Result<Self, String> {
        Self::parse_localized(input, Language::En)
    }

    pub fn parse_localized(input: &str, lang: Language) -> Result<Self, String> {
        let parts: Vec<_> = input.split(';').map(str::trim).collect();
        if parts.len() != 3 || parts[0].is_empty() {
            return Err(lang
                .pick(
                    "Use: Name; latitude; longitude",
                    "Use: Nombre; latitud; longitud",
                )
                .into());
        }
        let lat: f64 = parts[1].parse().map_err(|_| {
            lang.pick(
                "Invalid latitude (use a decimal point)",
                "Latitud inválida (use punto decimal)",
            )
        })?;
        let lon: f64 = parts[2].parse().map_err(|_| {
            lang.pick(
                "Invalid longitude (use a decimal point)",
                "Longitud inválida (use punto decimal)",
            )
        })?;
        if !lat.is_finite()
            || !lon.is_finite()
            || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon)
        {
            return Err(lang
                .pick(
                    "Latitude: -90 to 90. Longitude: -180 to 180.",
                    "Latitud: -90 a 90. Longitud: -180 a 180.",
                )
                .into());
        }
        Ok(Self {
            name: parts[0]
                .chars()
                .filter(|c| !c.is_control())
                .take(48)
                .collect(),
            lat,
            lon,
        })
    }

    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("astrarium/location.json"))
    }

    pub fn load() -> Option<Self> {
        let location: Self = serde_json::from_str(&fs::read_to_string(Self::path()?).ok()?).ok()?;
        Self::parse(&format!(
            "{};{};{}",
            location.name, location.lat, location.lon
        ))
        .ok()
    }

    pub fn save(&self) -> io::Result<()> {
        let path =
            Self::path().ok_or_else(|| io::Error::other("Configuration directory unavailable"))?;
        fs::create_dir_all(path.parent().unwrap())?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        fs::rename(temporary, path)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Modal {
    None,
    Help,
    Location,
    Search,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectId {
    Star(usize),
    Solar(usize),
}

pub struct App {
    pub language: Language,
    pub status: Option<String>,
    pub catalog: Catalog,
    pub location: Location,
    pub time: DateTime<Utc>,
    pub live: bool,
    pub camera: Camera,
    pub positions: Vec<Horizontal>,
    pub solar: Vec<SolarObject>,
    pub solar_error: Option<String>,
    last_calculation: Option<(i64, u64, u64)>,
    pub selected: Option<ObjectId>,
    pub constellations: bool,
    pub grid: bool,
    pub labels: bool,
    pub label_layout: LabelLayout,
    pub horizon: bool,
    pub sidebar: bool,
    pub ascii: bool,
    pub modal: Modal,
    pub input: String,
    pub error: String,
    pub search_index: usize,
    pub sky_rect: Rect,
    pub visible: Vec<(ObjectId, u16, u16)>,
    pub drag: Option<(u16, u16)>,
    pub dragged: bool,
    pub quit: bool,
}

impl App {
    pub fn new(
        location: Location,
        time: Option<DateTime<Utc>>,
        first_run: bool,
        ascii: bool,
    ) -> Self {
        let mut app = Self {
            language: Language::En,
            status: None,
            catalog: Catalog::load(),
            location,
            time: time.unwrap_or_else(Utc::now),
            live: time.is_none(),
            camera: Camera::default(),
            positions: vec![],
            solar: vec![],
            solar_error: None,
            last_calculation: None,
            selected: None,
            constellations: true,
            grid: true,
            labels: true,
            label_layout: LabelLayout::default(),
            horizon: true,
            sidebar: true,
            ascii,
            modal: Modal::None,
            input: String::new(),
            error: String::new(),
            search_index: 0,
            sky_rect: Rect::default(),
            visible: vec![],
            drag: None,
            dragged: false,
            quit: false,
        };
        if first_run {
            app.open_location();
        }
        app.update();
        app
    }

    pub fn update(&mut self) {
        if self.live {
            self.time = Utc::now();
        }
        let key = (
            self.time.timestamp(),
            self.location.lat.to_bits(),
            self.location.lon.to_bits(),
        );
        if self.last_calculation == Some(key) {
            return;
        }
        self.last_calculation = Some(key);
        let lst = astro::sidereal(self.time, self.location.lon);
        self.positions = self
            .catalog
            .stars
            .iter()
            .map(|star| {
                let (ra, dec) = astro::precess(star.ra, star.dec, self.time);
                astro::horizontal(ra, dec, self.location.lat, lst)
            })
            .collect();
        match solar::calculate(self.time, self.location.lat, self.location.lon) {
            Ok(objects) => {
                self.solar = objects;
                self.solar_error = None;
            }
            Err(error) => {
                self.solar.clear();
                self.solar_error = Some(error);
                if matches!(self.selected, Some(ObjectId::Solar(_))) {
                    self.selected = None;
                }
            }
        }
    }

    pub fn name(&self, object: ObjectId) -> &str {
        match object {
            ObjectId::Star(i) => &self.catalog.stars[i].name,
            ObjectId::Solar(i) => self.solar[i].body.name(self.language),
        }
    }

    pub fn position(&self, object: ObjectId) -> Horizontal {
        match object {
            ObjectId::Star(i) => self.positions[i],
            ObjectId::Solar(i) => self.solar[i].position,
        }
    }

    pub fn magnitude(&self, object: ObjectId) -> f64 {
        match object {
            ObjectId::Star(i) => self.catalog.stars[i].mag,
            ObjectId::Solar(i) => self.solar[i].magnitude,
        }
    }

    pub fn daylight(&self) -> &str {
        self.solar
            .iter()
            .find(|s| s.body == Body::Sun)
            .map(|sun| solar::daylight(sun.position.alt, self.language))
            .unwrap_or(
                self.language
                    .pick("EPHEMERIDES UNAVAILABLE", "EFEMÉRIDES NO DISPONIBLES"),
            )
    }

    pub fn equatorial(&self, ra: f64, dec: f64) -> Horizontal {
        let (ra, dec) = astro::precess(ra, dec, self.time);
        astro::horizontal(
            ra,
            dec,
            self.location.lat,
            astro::sidereal(self.time, self.location.lon),
        )
    }

    pub fn open_location(&mut self) {
        self.input = format!(
            "{}; {}; {}",
            self.location.name, self.location.lat, self.location.lon
        );
        self.error.clear();
        self.modal = Modal::Location;
    }

    pub fn matches(&self) -> Vec<ObjectId> {
        let query = normalize(self.input.trim());
        let bodies = self
            .solar
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                normalize(s.body.name(Language::Es)).contains(&query)
                    || s.body.english().contains(&query)
            })
            .map(|(i, _)| ObjectId::Solar(i));
        bodies
            .chain(
                self.catalog
                    .stars
                    .iter()
                    .enumerate()
                    .filter(|(_, star)| {
                        normalize(&star.name).contains(&query)
                            || format!("hip {}", star.id).contains(&query)
                            || (!query.is_empty() && star.con.to_lowercase() == query)
                    })
                    .map(|(i, _)| ObjectId::Star(i)),
            )
            .take(40)
            .collect()
    }

    fn focus(&mut self, index: ObjectId) {
        self.selected = Some(index);
        let point = self.position(index);
        self.camera.az = point.az;
        self.camera.alt = point
            .alt
            .clamp(if self.horizon { -25.0 } else { -90.0 }, 90.0);
    }

    fn turn_camera(&mut self, az: f64, alt: f64) {
        self.camera.turn(az, alt);
        if self.horizon {
            self.camera.alt = self.camera.alt.max(-25.0);
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
        self.status = None;
        if key.code == KeyCode::F(2) {
            self.language = self.language.other();
            self.error.clear();
            if let Err(error) = self.language.save() {
                self.status = Some(format!(
                    "{}: {error}",
                    self.language
                        .pick("Could not save language", "No se pudo guardar el idioma")
                ));
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.modal != Modal::None {
            self.modal_key(key);
            return;
        }
        let step = self.camera.fov
            / if key.modifiers.contains(KeyModifiers::SHIFT) {
                8.0
            } else {
                32.0
            };
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Left | KeyCode::Char('a' | 'A') => self.turn_camera(-step, 0.0),
            KeyCode::Right | KeyCode::Char('d' | 'D') => self.turn_camera(step, 0.0),
            KeyCode::Up | KeyCode::Char('w' | 'W') => self.turn_camera(0.0, step),
            KeyCode::Down | KeyCode::Char('s' | 'S') => self.turn_camera(0.0, -step),
            KeyCode::Char('+' | '=') => self.camera.zoom(0.88),
            KeyCode::Char('-') => self.camera.zoom(1.0 / 0.88),
            KeyCode::Char('c' | 'C') => self.constellations = !self.constellations,
            KeyCode::Char('g' | 'G') => self.grid = !self.grid,
            KeyCode::Char('n' | 'N') => self.labels = !self.labels,
            KeyCode::Char('h' | 'H') => {
                self.horizon = !self.horizon;
                if self.horizon {
                    self.camera.alt = self.camera.alt.max(-25.0);
                }
            }
            KeyCode::Char('i' | 'I') => self.sidebar = !self.sidebar,
            KeyCode::Char('l' | 'L') => self.open_location(),
            KeyCode::Char('?') | KeyCode::F(1) => self.modal = Modal::Help,
            KeyCode::Char('/') => {
                self.modal = Modal::Search;
                self.input.clear();
                self.search_index = 0;
            }
            KeyCode::Char(' ') => {
                self.live = true;
                self.update();
            }
            KeyCode::Char('p' | 'P') => self.live = !self.live,
            KeyCode::Char('[' | ']') => {
                self.live = false;
                let minutes = if key.code == KeyCode::Char(']') {
                    30
                } else {
                    -30
                };
                let next = self.time + Duration::minutes(minutes);
                use chrono::Datelike;
                if (1900..=2100).contains(&next.year()) {
                    self.time = next;
                }
            }
            KeyCode::Home => self.camera = Camera::default(),
            KeyCode::Enter => {
                if let Some(i) = self.selected {
                    self.focus(i);
                }
            }
            KeyCode::Tab | KeyCode::BackTab if !self.visible.is_empty() => {
                let current = self
                    .visible
                    .iter()
                    .position(|(i, _, _)| Some(*i) == self.selected);
                let len = self.visible.len();
                let next = match (current, key.code) {
                    (Some(i), KeyCode::BackTab) => (i + len - 1) % len,
                    (Some(i), _) => (i + 1) % len,
                    _ => 0,
                };
                self.selected = Some(self.visible[next].0);
            }
            _ => {}
        }
    }

    fn modal_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc || self.modal == Modal::Help {
            self.modal = Modal::None;
            return;
        }
        match key.code {
            KeyCode::Enter if self.modal == Modal::Location => {
                match Location::parse_localized(&self.input, self.language) {
                    Ok(location) => match location.save() {
                        Ok(()) => {
                            self.location = location;
                            self.modal = Modal::None;
                            self.update();
                        }
                        Err(e) => {
                            self.error = format!(
                                "{}: {e}",
                                self.language.pick("Could not save", "No se pudo guardar")
                            )
                        }
                    },
                    Err(error) => self.error = error,
                }
            }
            KeyCode::Enter if self.modal == Modal::Search => {
                if let Some(&index) = self.matches().get(self.search_index) {
                    self.focus(index);
                    self.modal = Modal::None;
                }
            }
            KeyCode::Up if self.modal == Modal::Search => {
                self.search_index = self.search_index.saturating_sub(1)
            }
            KeyCode::Down if self.modal == Modal::Search => {
                self.search_index =
                    (self.search_index + 1).min(self.matches().len().saturating_sub(1));
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
                self.search_index = 0;
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.search_index = 0;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && self.input.len() < 160 =>
            {
                self.input.push(c);
                self.search_index = 0;
            }
            _ => {}
        }
    }

    pub fn mouse(&mut self, event: MouseEvent) {
        if self.modal != Modal::None {
            return;
        }
        let (x, y) = (event.column, event.row);
        let inside = self.sky_rect.contains((x, y).into());
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) if inside => {
                self.drag = Some((x, y));
                self.dragged = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((px, py)) = self.drag {
                    let dx = x as f64 - px as f64;
                    let dy = y as f64 - py as f64;
                    if dx != 0.0 || dy != 0.0 {
                        self.dragged = true;
                        let scale = self.camera.fov / self.sky_rect.width.max(1) as f64;
                        self.turn_camera(-dx * scale, dy * scale * 2.0);
                        self.drag = Some((x, y));
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.drag.is_some() && !self.dragged && inside {
                    self.selected = self
                        .visible
                        .iter()
                        .filter_map(|&(i, sx, sy)| {
                            let distance = ((x as f64 - sx as f64) * 0.5).powi(2)
                                + (y as f64 - sy as f64).powi(2);
                            (distance <= 6.25).then_some((i, distance))
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map(|(i, _)| i);
                }
                self.drag = None;
            }
            MouseEventKind::ScrollUp if inside => self.camera.zoom(0.9),
            MouseEventKind::ScrollDown if inside => self.camera.zoom(1.0 / 0.9),
            _ => {}
        }
    }
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_rejects_nonfinite_and_out_of_range() {
        for value in ["A;NaN;0", "A;0;inf", "A;91;0", "A;0;-181", ";0;0", "A;0"] {
            assert!(Location::parse(value).is_err(), "{value}");
        }
        let location = Location::parse("Buenos Aires; -34.6037; -58.3816").unwrap();
        assert_eq!(location.lon, -58.3816);
    }

    #[test]
    fn embedded_catalog_has_sirius_and_valid_positions() {
        let c = Catalog::load();
        assert!(c.stars.len() > 5000);
        let sirius = c.stars.iter().find(|s| s.name == "Sirius").unwrap();
        assert_eq!(sirius.id, 32349);
        assert!((sirius.ra - 101.287).abs() < 0.01);
        assert!(c.stars.iter().all(|s| s.ra.is_finite()
            && (0.0..360.0).contains(&s.ra)
            && (-90.0..=90.0).contains(&s.dec)));
        assert!(c.constellations.iter().any(|c| c.name == "Ori"));
    }

    #[test]
    fn dragging_rotates_and_does_not_select() {
        let mut app = App::new(Location::default(), None, false, false);
        app.sky_rect = Rect::new(0, 0, 100, 40);
        for (kind, column) in [
            (MouseEventKind::Down(MouseButton::Left), 50),
            (MouseEventKind::Drag(MouseButton::Left), 60),
            (MouseEventKind::Up(MouseButton::Left), 60),
        ] {
            app.mouse(MouseEvent {
                kind,
                column,
                row: 20,
                modifiers: KeyModifiers::NONE,
            });
        }
        assert!((app.camera.az - 349.5).abs() < 1e-8);
        assert!(app.selected.is_none());
        assert!(app.drag.is_none());
    }

    #[test]
    fn solar_search_and_selection_survive_time_and_location_changes() {
        let mut app = App::new(
            Location::default(),
            Some("2026-01-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        for (query, expected) in [
            ("sol", Body::Sun),
            ("moon", Body::Moon),
            ("mercurio", Body::Mercury),
            ("Venus", Body::Venus),
            ("Marte", Body::Mars),
            ("JUPITER", Body::Jupiter),
            ("Júpiter", Body::Jupiter),
            ("saturn", Body::Saturn),
            ("urano", Body::Uranus),
            ("Neptune", Body::Neptune),
        ] {
            app.input = query.into();
            let ObjectId::Solar(i) = app.matches()[0] else {
                panic!("solar object not found: {query}")
            };
            assert_eq!(app.solar[i].body, expected);
        }
        app.input = "Luna".into();
        app.modal = Modal::Search;
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let selected = app.selected.unwrap();
        let before = app.position(selected);
        let ObjectId::Solar(i) = selected else {
            panic!("not the Moon")
        };
        let phase = app.solar[i].illuminated;
        app.key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        app.update();
        assert_eq!(app.selected, Some(selected));
        assert_eq!(app.name(selected), "Moon");
        assert!((app.position(selected).alt - before.alt).abs() > 0.1);
        assert!((app.solar[i].illuminated - phase).abs() > 0.0001);
        let position = app.position(selected);
        app.location.lat = -34.6;
        app.location.lon = -58.4;
        app.update();
        assert!((app.position(selected).alt - position.alt).abs() > 1.0);
    }
}
