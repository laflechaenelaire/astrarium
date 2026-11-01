mod app;
mod astro;
mod i18n;
mod labels;
mod solar;
mod ui;

use app::{App, Location};
use chrono::{DateTime, Datelike, Utc};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use i18n::Language;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    error::Error,
    io::{self, IsTerminal},
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(
    version,
    about = "ASTRARIUM — a celestial atlas in your terminal",
    after_help = "Controls: drag / WASD / arrows · wheel / +/- zoom · / search · L location · F2 language · ? help · Q quit\nExample: astrarium --lat=-34.6037 --lon=-58.3816 --location 'Buenos Aires' --lang es"
)]
struct Args {
    /// Observer latitude, north positive
    #[arg(long, requires = "lon", allow_hyphen_values = true, value_parser = latitude)]
    lat: Option<f64>,
    /// Observer longitude, east positive
    #[arg(long, requires = "lat", allow_hyphen_values = true, value_parser = longitude)]
    lon: Option<f64>,
    /// Display name for the supplied coordinates
    #[arg(long, requires = "lat")]
    location: Option<String>,
    /// Interface language for this run (default: English, or saved F2 preference)
    #[arg(long, value_enum)]
    lang: Option<Language>,
    /// Start paused at an RFC 3339 date (1900–2100), e.g. 2026-11-01T00:00:00Z
    #[arg(long, value_parser = parse_time)]
    at: Option<DateTime<Utc>>,
    /// Use plain dot plotting instead of Unicode braille
    #[arg(long)]
    ascii: bool,
    /// Explore the entire celestial sphere, including below the horizon (toggle H)
    #[arg(long)]
    no_horizon: bool,
    /// Validate embedded data and print observer details without opening the TUI
    #[arg(long)]
    check: bool,
}

fn coordinate(value: &str, bound: f64) -> Result<f64, String> {
    let n: f64 = value.parse().map_err(|_| "Expected a decimal number")?;
    if n.is_finite() && (-bound..=bound).contains(&n) {
        Ok(n)
    } else {
        Err(format!(
            "Expected a finite number between -{bound} and {bound}"
        ))
    }
}
fn latitude(value: &str) -> Result<f64, String> {
    coordinate(value, 90.0)
}
fn longitude(value: &str) -> Result<f64, String> {
    coordinate(value, 180.0)
}
fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    let time = DateTime::parse_from_rfc3339(value)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    if (1900..=2100).contains(&time.year()) {
        Ok(time)
    } else {
        Err("Supported years: 1900–2100".into())
    }
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let language = args.lang.unwrap_or_else(Language::load);
    let saved = Location::load();
    let first_run = args.lat.is_none() && saved.is_none();
    let location = match (args.lat, args.lon) {
        (Some(lat), Some(lon)) => Location {
            name: args
                .location
                .unwrap_or_else(|| language.pick("Observer", "Observador").into()),
            lat,
            lon,
        },
        _ => saved.unwrap_or_default(),
    };
    let mut app = App::new(location, args.at, first_run, args.ascii);
    app.language = language;
    app.horizon = !args.no_horizon;
    if args.check {
        if let Some(error) = &app.solar_error {
            return Err(error.clone().into());
        }
        println!(
            "ASTRARIUM {}\n{} {} · {} {}\n{}: {} ({:.4}, {:.4}){}\nUTC: {}\n{}: {}",
            env!("CARGO_PKG_VERSION"),
            app.catalog.stars.len(),
            language.pick("stars", "estrellas"),
            app.catalog.constellations.len(),
            language.pick("constellation line groups", "grupos de constelaciones"),
            language.pick("Observer", "Observador"),
            app.location.name,
            app.location.lat,
            app.location.lon,
            if first_run {
                language.pick(
                    " [default; choose your location with L]",
                    " [por defecto; elegí tu ubicación con L]",
                )
            } else {
                ""
            },
            app.time.to_rfc3339(),
            language.pick("Above geometric horizon", "Sobre el horizonte geométrico"),
            app.positions.iter().filter(|p| p.alt >= 0.0).count()
        );
        println!(
            "{}: {} {} · {}",
            language.pick("Solar system", "Sistema Solar"),
            app.solar.len(),
            language.pick("bodies", "cuerpos"),
            app.daylight()
        );
        for body in &app.solar {
            println!(
                "{}: az {:.3}° · alt {:+.3}° · mag {:.2} · {} {:.6} {}{}",
                body.body.name(language),
                body.position.az,
                body.position.alt,
                body.magnitude,
                language.pick("distance", "distancia"),
                body.distance_au,
                language.pick("AU", "UA"),
                body.phase_name(language)
                    .map(|phase| format!(
                        " · {phase} · {} {:.1}%",
                        language.pick("illuminated", "iluminada"),
                        body.illuminated * 100.0
                    ))
                    .unwrap_or_default()
            );
        }
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(language
            .pick(
                "Open astrarium in an interactive terminal, or use --check.",
                "Abrí astrarium en una terminal interactiva, o usá --check.",
            )
            .into());
    }
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        drop(TerminalGuard);
        previous_hook(info);
    }));
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;
    let mut last_tick = Instant::now();
    let mut dirty = true;
    while !app.quit {
        if dirty {
            app.update();
            terminal.draw(|frame| ui::draw(frame, &mut app))?;
            dirty = false;
        }
        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    app.key(key);
                    dirty = true;
                }
                Event::Mouse(mouse) => {
                    app.mouse(mouse);
                    dirty = true;
                }
                Event::Resize(_, _) => {
                    app.drag = None;
                    dirty = true;
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= Duration::from_secs(1) {
            dirty |= app.live;
            last_tick = Instant::now();
        }
    }
    Ok(())
}
