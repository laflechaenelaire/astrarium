use crate::{
    app::{App, Modal, ObjectId},
    astro::Horizontal,
    labels::Candidate,
    solar::{Body, AU_KM},
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Context, Line as SkyLine, Points},
        Block, Borders, Clear, Paragraph, Wrap,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

const BG: Color = Color::Rgb(11, 21, 22);
const FG: Color = Color::Rgb(202, 206, 187);
const GOLD: Color = Color::Rgb(184, 162, 94);
const MUTED: Color = Color::Rgb(111, 133, 129);
const EDGE: Color = Color::Rgb(51, 72, 69);
const GRID: Color = Color::Rgb(29, 46, 44);
const LINK: Color = Color::Rgb(67, 88, 81);

fn text(value: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(color))
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(EDGE))
        .title(Span::styled(title, Style::default().fg(MUTED)))
        .style(Style::default().bg(BG).fg(FG))
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let lang = app.language;
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG).fg(FG)), area);
    if area.width < 52 || area.height < 18 {
        app.visible.clear();
        app.sky_rect = Rect::default();
        frame.render_widget(
            Paragraph::new(lang.pick(
                "ASTRARIUM\nResize to at least 52 × 18.\nQ / Ctrl+C to quit. F2 language.",
                "ASTRARIUM\nAmpliá la terminal a 52 × 18 o más.\nQ / Ctrl+C salir. F2 idioma.",
            ))
            .style(Style::default().fg(GOLD)),
            area,
        );
        return;
    }
    let vertical = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);
    let header =
        Layout::horizontal([Constraint::Min(10), Constraint::Length(29)]).split(vertical[0]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                text("  A S T R A R I U M", GOLD),
                text(
                    lang.pick("   /   CELESTIAL ATLAS", "   /   ATLAS CELESTE"),
                    MUTED,
                ),
            ]),
            Line::from(text(
                format!(
                    "  {}  ·  {:.2}° / {:.2}°",
                    app.location.name, app.location.lat, app.location.lon
                ),
                MUTED,
            )),
            Line::from(text(
                format!(
                    "  {} · {}",
                    app.daylight(),
                    if app.horizon {
                        lang.pick("local sky", "cielo local")
                    } else {
                        lang.pick("full sky", "cielo completo")
                    }
                ),
                GOLD,
            )),
        ]),
        header[0],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(text(
                app.time.format("%Y-%m-%d  %H:%M:%S UTC").to_string(),
                FG,
            )),
            Line::from(text(
                if app.live {
                    lang.pick("● LIVE", "● TIEMPO REAL")
                } else {
                    lang.pick("Ⅱ PAUSED", "Ⅱ TIEMPO PAUSADO")
                },
                GOLD,
            )),
            Line::from(text(
                lang.pick("F2 · English → Español", "F2 · Español → English"),
                MUTED,
            )),
        ])
        .alignment(ratatui::layout::Alignment::Right),
        header[1],
    );

    let body = if app.sidebar && area.width >= 106 {
        Layout::horizontal([Constraint::Min(40), Constraint::Length(32)]).split(vertical[1])
    } else {
        Layout::horizontal([Constraint::Percentage(100), Constraint::Length(0)]).split(vertical[1])
    };
    sky(frame, app, body[0]);
    if body[1].width > 0 {
        sidebar(frame, app, body[1]);
    }

    let selection = app
        .selected
        .map(|i| {
            let mut description = format!(
                "{}  ·  mag {:.2}  ·  alt {:+.1}°",
                app.name(i),
                app.magnitude(i),
                app.position(i).alt
            );
            if let ObjectId::Solar(index) = i {
                let body = &app.solar[index];
                if let Some(phase) = body.phase_name(lang) {
                    description.push_str(&format!(
                        "  ·  {} {:.0}%",
                        phase,
                        body.illuminated * 100.0
                    ));
                }
            }
            description
        })
        .unwrap_or_else(|| {
            lang.pick(
                "Drag the sky to look around",
                "Arrastrá el cielo para mirar alrededor",
            )
            .into()
        });
    let selection = app.status.as_deref().unwrap_or(&selection);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                text("  ", FG),
                text(selection, GOLD),
                text(
                    format!(
                        "   /   {} {:.0}°  ·  {} {:+.0}°  ·  {} {:.0}°",
                        lang.direction(app.camera.az),
                        app.camera.az,
                        lang.pick("alt", "altura"),
                        app.camera.alt,
                        lang.pick("FOV", "campo"),
                        app.camera.fov
                    ),
                    MUTED,
                ),
            ]),
            Line::from(text(
                lang.pick("  WASD/↑↓←→ look   wheel/± zoom   / search   L location   ? help   Q quit", "  WASD/↑↓←→ mirar   rueda/± zoom   / buscar   L ubicación   ? ayuda   Q salir"),
                FG,
            )),
            Line::from(text(
                lang.pick("  H horizon   C constellations   G grid   I panel   [ ] ±30 min   Space now", "  H horizonte   C constelaciones   G grilla   I panel   [ ] ±30 min   Espacio ahora"),
                MUTED,
            )),
        ]),
        vertical[2],
    );
    if app.modal != Modal::None {
        modal(frame, app);
    }
}

fn curve(
    ctx: &mut Context<'_>,
    app: &App,
    aspect: f64,
    points: impl IntoIterator<Item = Horizontal>,
    color: Color,
) {
    let mut previous = None;
    for p in points {
        let next = app.camera.project(p, aspect);
        if let (Some((x1, y1)), Some((x2, y2))) = (previous, next) {
            ctx.draw(&SkyLine {
                x1,
                y1,
                x2,
                y2,
                color,
            });
        }
        previous = next;
    }
}

fn sky(frame: &mut Frame, app: &mut App, area: Rect) {
    let lang = app.language;
    let title = format!(
        " I · {}  /  {} ",
        if app.horizon {
            lang.pick("LOCAL SKY", "CIELO LOCAL")
        } else {
            lang.pick("FULL SKY", "CIELO COMPLETO")
        },
        lang.direction(app.camera.az)
    );
    let border = block(&title);
    let inner = border.inner(area);
    app.sky_rect = inner;
    let aspect = inner.width as f64 * 0.5 / inner.height.max(1) as f64;
    let limit = if app.camera.fov > 85.0 {
        4.9
    } else if app.camera.fov > 55.0 {
        5.5
    } else {
        6.0
    };
    let mut projected: Vec<_> = app
        .catalog
        .stars
        .iter()
        .enumerate()
        .filter_map(|(i, star)| {
            let p = app.positions[i];
            if (app.horizon && p.alt < 0.0)
                || (star.mag > limit && app.selected != Some(ObjectId::Star(i)))
            {
                return None;
            }
            app.camera
                .project(p, aspect)
                .map(|(x, y)| (ObjectId::Star(i), x, y))
        })
        .collect();
    // Solar-system objects remain discoverable even when too faint for the
    // naked eye. The atlas is geometric; the sidebar explains visibility.
    projected.extend(app.solar.iter().enumerate().filter_map(|(i, body)| {
        if app.horizon && body.position.alt < 0.0 {
            return None;
        }
        app.camera
            .project(body.position, aspect)
            .map(|(x, y)| (ObjectId::Solar(i), x, y))
    }));
    projected.sort_by(|a, b| app.magnitude(a.0).total_cmp(&app.magnitude(b.0)));
    app.visible = projected
        .iter()
        .map(|&(i, x, y)| {
            let sx =
                inner.x + ((x + 1.0) * 0.5 * inner.width.saturating_sub(1) as f64).round() as u16;
            let sy =
                inner.y + ((1.0 - y) * 0.5 * inner.height.saturating_sub(1) as f64).round() as u16;
            (i, sx, sy)
        })
        .collect();
    let canvas = Canvas::default()
        .block(border)
        .background_color(BG)
        .marker(if app.ascii {
            Marker::Dot
        } else {
            Marker::Braille
        })
        .x_bounds([-1.0, 1.0])
        .y_bounds([-1.0, 1.0])
        .paint(|ctx| {
            if app.grid {
                for alt in ((if app.horizon { 15 } else { -75 })..90).step_by(15) {
                    curve(
                        ctx,
                        app,
                        aspect,
                        (0..=360).step_by(2).map(|az| Horizontal {
                            az: az as f64,
                            alt: alt as f64,
                        }),
                        GRID,
                    );
                }
                for az in (0..360).step_by(30) {
                    curve(
                        ctx,
                        app,
                        aspect,
                        ((if app.horizon { 0 } else { -90 })..=90)
                            .step_by(2)
                            .map(|alt| Horizontal {
                                az: az as f64,
                                alt: alt as f64,
                            }),
                        GRID,
                    );
                }
            }
            if app.horizon {
                for band in 0..18 {
                    curve(
                        ctx,
                        app,
                        aspect,
                        (0..=720).map(|step| {
                            let az = step as f64 / 2.0;
                            let wave = (az.to_radians() * 5.0).sin() * 1.0
                                + (az.to_radians() * 11.0).cos() * 0.6;
                            Horizontal {
                                az,
                                alt: -2.5 - band as f64 * 1.1 + wave * (1.0 + band as f64 * 0.035),
                            }
                        }),
                        if band % 4 == 0 { EDGE } else { GRID },
                    );
                }
                curve(
                    ctx,
                    app,
                    aspect,
                    (0..=720).map(|az| Horizontal {
                        az: az as f64 / 2.0,
                        alt: 0.0,
                    }),
                    MUTED,
                );
                for az in (0..360).step_by(45) {
                    if let Some((x, y)) = app.camera.project(
                        Horizontal {
                            az: az as f64,
                            alt: -1.8,
                        },
                        aspect,
                    ) {
                        ctx.print(x, y, text(lang.direction(az as f64), GOLD));
                    }
                }
            }
            if app.constellations {
                for constellation in &app.catalog.constellations {
                    let active = app.selected.is_some_and(|object| match object {
                        ObjectId::Star(i) => app.catalog.stars[i].con == constellation.name,
                        ObjectId::Solar(_) => false,
                    });
                    for line in &constellation.lines {
                        for pair in line.windows(2) {
                            let delta = (pair[1][0] - pair[0][0] + 180.0).rem_euclid(360.0) - 180.0;
                            let samples = (0..=16).map(|j| {
                                let t = j as f64 / 16.0;
                                app.equatorial(
                                    pair[0][0] + delta * t,
                                    pair[0][1] + (pair[1][1] - pair[0][1]) * t,
                                )
                            });
                            // Only clip at the physical horizon in local-sky mode.
                            let mut last = None;
                            for p in samples {
                                let next = if !app.horizon || p.alt >= 0.0 {
                                    app.camera.project(p, aspect)
                                } else {
                                    None
                                };
                                if let (Some((x1, y1)), Some((x2, y2))) = (last, next) {
                                    ctx.draw(&SkyLine {
                                        x1,
                                        y1,
                                        x2,
                                        y2,
                                        color: if active { GOLD } else { LINK },
                                    });
                                }
                                last = next;
                            }
                        }
                    }
                }
            }
            ctx.layer();
            for (min, max, color) in [
                (3.5, 7.0, MUTED),
                (1.5, 3.5, FG),
                (-3.0, 1.5, Color::Rgb(231, 226, 198)),
            ] {
                let points: Vec<_> = projected
                    .iter()
                    .filter(|&&(i, _, _)| {
                        matches!(i, ObjectId::Star(_)) && (min..max).contains(&app.magnitude(i))
                    })
                    .map(|&(_, x, y)| (x, y))
                    .collect();
                ctx.draw(&Points {
                    coords: &points,
                    color,
                });
            }
            for &(i, x, y) in &projected {
                match i {
                    ObjectId::Star(index) if app.catalog.stars[index].mag < 1.6 => {
                        ctx.print(x, y, text(if app.ascii { "*" } else { "✦" }, FG));
                    }
                    ObjectId::Solar(index) => {
                        let body = &app.solar[index];
                        ctx.print(x, y, text(body.symbol(app.ascii), body_color(body.body)));
                    }
                    _ => {}
                }
                if app.selected == Some(i) {
                    let radius = 0.045;
                    let ring: Vec<_> = (0..=48)
                        .map(|j| {
                            let angle = j as f64 / 48.0 * std::f64::consts::TAU;
                            (x + angle.cos() * radius, y + angle.sin() * radius * aspect)
                        })
                        .collect();
                    for pair in ring.windows(2) {
                        ctx.draw(&SkyLine {
                            x1: pair[0].0,
                            y1: pair[0].1,
                            x2: pair[1].0,
                            y2: pair[1].1,
                            color: GOLD,
                        });
                    }
                    if matches!(i, ObjectId::Star(_)) {
                        ctx.print(x, y, text("+", GOLD));
                    }
                }
            }
        });
    frame.render_widget(canvas, area);
    let candidates: Vec<_> = app
        .visible
        .iter()
        .map(|&(object, x, y)| Candidate {
            object,
            x,
            y,
            width: app.name(object).width().min(u16::MAX as usize) as u16,
            magnitude: app.magnitude(object),
            named: match object {
                ObjectId::Star(i) => app.catalog.stars[i].named,
                ObjectId::Solar(_) => true,
            },
        })
        .collect();
    app.label_layout
        .update(inner, app.camera.fov, app.labels, app.selected, &candidates);
    for placement in &app.label_layout.placements {
        let color = if app.selected == Some(placement.object) {
            GOLD
        } else {
            match placement.object {
                ObjectId::Solar(i) => body_color(app.solar[i].body),
                _ => FG,
            }
        };
        frame.buffer_mut().set_span(
            placement.rect.x,
            placement.rect.y,
            &text(app.name(placement.object), color),
            placement.rect.width,
        );
    }
}

fn body_color(body: Body) -> Color {
    match body {
        Body::Sun => Color::Rgb(224, 185, 94),
        Body::Moon => Color::Rgb(220, 223, 207),
        Body::Mercury => Color::Rgb(168, 161, 145),
        Body::Venus => Color::Rgb(229, 213, 165),
        Body::Mars => Color::Rgb(204, 132, 103),
        Body::Jupiter => Color::Rgb(207, 180, 140),
        Body::Saturn => Color::Rgb(191, 176, 122),
        Body::Uranus => Color::Rgb(139, 191, 185),
        Body::Neptune => Color::Rgb(126, 158, 199),
    }
}

fn sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let lang = app.language;
    let panel = block(lang.pick(" II · OBSERVATORY ", " II · OBSERVATORIO "));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let mut lines = vec![
        Line::from(""),
        Line::from(text(
            lang.pick("  OBSERVER'S CHART", "  CARTA DEL OBSERVADOR"),
            GOLD,
        )),
        Line::from(""),
    ];
    if let Some(ObjectId::Star(index)) = app.selected {
        let star = &app.catalog.stars[index];
        let p = app.positions[index];
        lines.extend([
            Line::from(
                text(format!("  {}", star.name), FG)
                    .style(Style::default().fg(FG).add_modifier(Modifier::BOLD)),
            ),
            Line::from(text(format!("  HIP {}  /  {}", star.id, star.con), MUTED)),
            Line::from(""),
            Line::from(text("  ────────────────────────", EDGE)),
            Line::from(""),
            Line::from(vec![
                text(lang.pick("  Magnitude    ", "  Magnitud     "), MUTED),
                text(format!("{:.2}", star.mag), FG),
            ]),
            Line::from(vec![
                text(lang.pick("  Azimuth      ", "  Azimut       "), MUTED),
                text(format!("{:.1}° {}", p.az, lang.direction(p.az)), GOLD),
            ]),
            Line::from(vec![
                text(lang.pick("  Altitude     ", "  Altura       "), MUTED),
                text(format!("{:+.1}°", p.alt), FG),
            ]),
            Line::from(""),
            Line::from(text(
                if p.alt >= 0.0 {
                    lang.pick("  Above the horizon", "  Sobre el horizonte")
                } else {
                    lang.pick("  Below the horizon", "  Bajo el horizonte")
                },
                if p.alt >= 0.0 { GOLD } else { MUTED },
            )),
            Line::from(""),
            Line::from(text(
                lang.pick("  J2000 COORDINATES", "  COORDENADAS J2000"),
                MUTED,
            )),
            Line::from(text(
                format!(
                    "  {}  {:02.0}h {:04.1}m",
                    lang.pick("RA", "AR"),
                    (star.ra / 15.0).floor(),
                    (star.ra / 15.0).fract() * 60.0
                ),
                FG,
            )),
            Line::from(text(format!("  Dec  {:+.3}°", star.dec), FG)),
            Line::from(""),
            Line::from(text(
                lang.pick("  Enter → center", "  Enter → centrar"),
                GOLD,
            )),
            Line::from(text(
                lang.pick("  Tab   → next object", "  Tab   → siguiente objeto"),
                MUTED,
            )),
        ]);
    } else if let Some(ObjectId::Solar(index)) = app.selected {
        let body = &app.solar[index];
        let p = body.position;
        lines.extend([
            Line::from(
                text(
                    format!("  {} {}", body.symbol(app.ascii), body.body.name(lang)),
                    body_color(body.body),
                )
                .style(Style::default().add_modifier(Modifier::BOLD)),
            ),
            Line::from(text(
                match body.body {
                    Body::Sun => lang.pick("  Star · Solar System", "  Estrella · Sistema Solar"),
                    Body::Moon => lang.pick(
                        "  Earth's natural satellite",
                        "  Satélite natural de la Tierra",
                    ),
                    _ => lang.pick("  Planet · Solar System", "  Planeta · Sistema Solar"),
                },
                MUTED,
            )),
            Line::from(""),
            Line::from(text(
                format!(
                    "  {:<13}{:.2}",
                    lang.pick("Magnitude", "Magnitud"),
                    body.magnitude
                ),
                FG,
            )),
            Line::from(text(
                format!(
                    "  {:<13}{:.1}° {}",
                    lang.pick("Azimuth", "Azimut"),
                    p.az,
                    lang.direction(p.az)
                ),
                GOLD,
            )),
            Line::from(text(
                format!("  {:<13}{:+.1}°", lang.pick("Altitude", "Altura"), p.alt),
                FG,
            )),
            Line::from(text(
                if body.body == Body::Moon {
                    format!(
                        "  {:<13}{:.0} km",
                        lang.pick("Distance", "Distancia"),
                        body.distance_au * AU_KM
                    )
                } else {
                    format!(
                        "  {:<13}{:.3} {}",
                        lang.pick("Distance", "Distancia"),
                        body.distance_au,
                        lang.pick("AU", "UA")
                    )
                },
                FG,
            )),
        ]);
        if let Some(phase) = body.phase_name(lang) {
            lines.push(Line::from(text(format!("  {phase}"), GOLD)));
            lines.push(Line::from(text(
                format!(
                    "  {:<13}{:.1}%",
                    lang.pick("Illuminated", "Iluminada"),
                    body.illuminated * 100.0
                ),
                FG,
            )));
        }
        lines.extend([
            Line::from(""),
            Line::from(text(
                if p.alt >= 0.0 {
                    lang.pick("  Above the horizon", "  Sobre el horizonte")
                } else {
                    lang.pick("  Below the horizon", "  Bajo el horizonte")
                },
                GOLD,
            )),
            Line::from(text(
                lang.pick("  OF-DATE COORD. · TOPOC.", "  COORD. DE FECHA · TOPOC."),
                MUTED,
            )),
            Line::from(text(
                format!(
                    "  {} {:.3}h  Dec {:+.2}°",
                    lang.pick("RA", "AR"),
                    body.ra / 15.0,
                    body.dec
                ),
                FG,
            )),
            Line::from(""),
            Line::from(text(
                lang.pick("  Enter → center", "  Enter → centrar"),
                GOLD,
            )),
            Line::from(text(
                lang.pick("  Tab   → next object", "  Tab   → siguiente objeto"),
                MUTED,
            )),
            Line::from(text(
                lang.pick(
                    "  Atlas: daylight not simulated",
                    "  Atlas: no simula luz diurna",
                ),
                MUTED,
            )),
        ]);
        if matches!(body.body, Body::Uranus | Body::Neptune) {
            lines.push(Line::from(text(
                lang.pick("  May require a telescope", "  Puede requerir telescopio"),
                MUTED,
            )));
        }
    } else {
        lines.extend([
            Line::from(text(lang.pick("  A small atlas", "  Un pequeño atlas"), FG)),
            Line::from(text(
                lang.pick("  of an immense sky.", "  de un cielo inmenso."),
                FG,
            )),
            Line::from(""),
            Line::from(text("  ────────────────────────", EDGE)),
            Line::from(""),
            Line::from(text(
                lang.pick("  Click an object", "  Clic en un objeto"),
                GOLD,
            )),
            Line::from(text(
                lang.pick("  to discover its position.", "  para conocer su posición."),
                MUTED,
            )),
            Line::from(""),
            Line::from(text(
                lang.pick("  Drag to look around.", "  Arrastrá para girar."),
                FG,
            )),
            Line::from(text(
                lang.pick("  Scroll to zoom in.", "  Rueda para acercarte."),
                FG,
            )),
            Line::from(""),
            Line::from(text(
                lang.pick("  /  Search by name", "  /  Buscar por nombre"),
                MUTED,
            )),
            Line::from(text(
                lang.pick("  L  Change location", "  L  Cambiar ubicación"),
                MUTED,
            )),
            Line::from(text(
                lang.pick("  ?  All controls", "  ?  Todos los controles"),
                MUTED,
            )),
        ]);
    }
    if inner.height >= 29 {
        while lines.len() < inner.height as usize - 6 {
            lines.push(Line::from(""));
        }
        lines.extend([
            Line::from(text("  ────────────────────────", EDGE)),
            Line::from(text(
                format!(
                    "  {} {}",
                    app.visible.len(),
                    lang.pick("objects in this view", "objetos en esta vista")
                ),
                MUTED,
            )),
            Line::from(text(
                lang.pick("  Hipparcos + Solar System", "  Hipparcos + Sistema Solar"),
                MUTED,
            )),
            Line::from(text(
                if app.horizon {
                    lang.pick("  Geometric horizon", "  Horizonte geométrico")
                } else {
                    lang.pick(
                        "  Horizon hidden · 360° sky",
                        "  Horizonte oculto · cielo 360°",
                    )
                },
                MUTED,
            )),
            Line::from(text(
                lang.pick("  H → toggle horizon", "  H → alternar horizonte"),
                MUTED,
            )),
        ]);
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn modal(frame: &mut Frame, app: &App) {
    let lang = app.language;
    let area = frame.area();
    let height = match app.modal {
        Modal::Help => 24,
        Modal::Search => 18,
        _ => 14,
    }
    .min(area.height - 2);
    let width = 72.min(area.width - 4);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    let title = match app.modal {
        Modal::Help => lang.pick(" OBSERVER'S GUIDE ", " GUÍA DEL OBSERVADOR "),
        Modal::Search => lang.pick(" SEARCH THE CATALOGUE ", " BUSCAR EN EL CATÁLOGO "),
        _ => lang.pick(" YOUR PLACE IN THE WORLD ", " TU LUGAR EN EL MUNDO "),
    };
    let panel = block(title).border_style(Style::default().fg(GOLD));
    let inner = panel.inner(rect);
    frame.render_widget(panel, rect);
    let mut lines = vec![Line::from("")];
    match app.modal {
        Modal::Help => {
            for (key, description) in [
                (
                    lang.pick("Drag", "Arrastrar"),
                    lang.pick(
                        "Look around with the left mouse button",
                        "Girar la mirada con el botón izquierdo",
                    ),
                ),
                (
                    lang.pick("WASD / Arrows", "WASD / Flechas"),
                    lang.pick(
                        "Look around · Shift to move faster",
                        "Mirar alrededor · Shift para ir más rápido",
                    ),
                ),
                (
                    lang.pick("Wheel / + -", "Rueda / + -"),
                    lang.pick("Zoom in or out", "Acercar o alejar"),
                ),
                (
                    lang.pick("Click / Tab", "Clic / Tab"),
                    lang.pick(
                        "Select a star, planet, Sun or Moon",
                        "Seleccionar estrella, planeta, Sol o Luna",
                    ),
                ),
                (
                    "Enter",
                    lang.pick(
                        "Center the selected object",
                        "Centrar el objeto seleccionado",
                    ),
                ),
                (
                    "/",
                    lang.pick(
                        "Search: Moon, Jupiter, Sirius, HIP, Ori…",
                        "Buscar: Luna, Jupiter, Sirius, HIP, Ori…",
                    ),
                ),
                (
                    "L",
                    lang.pick(
                        "Set and save observer location",
                        "Configurar y guardar ubicación",
                    ),
                ),
                (
                    "F2",
                    lang.pick(
                        "Switch to Spanish and remember",
                        "Cambiar a inglés y recordar",
                    ),
                ),
                (
                    "C / G / N",
                    lang.pick(
                        "Constellations / grid / names",
                        "Constelaciones / grilla / nombres",
                    ),
                ),
                (
                    "H",
                    lang.pick(
                        "Local sky / full sky without horizon",
                        "Cielo local / cielo completo sin horizonte",
                    ),
                ),
                (
                    "I",
                    lang.pick("Show / hide sidebar", "Mostrar / ocultar panel lateral"),
                ),
                (
                    "[ / ]",
                    lang.pick(
                        "Step backward / forward 30 minutes",
                        "Retroceder / avanzar 30 minutos",
                    ),
                ),
                (
                    lang.pick("P / Space", "P / Espacio"),
                    lang.pick(
                        "Pause / return to the current time",
                        "Pausar / volver a la hora actual",
                    ),
                ),
                (
                    "Home",
                    lang.pick(
                        "Reset view facing north",
                        "Restablecer la vista hacia el norte",
                    ),
                ),
                ("Q / Ctrl+C", lang.pick("Quit", "Salir")),
            ] {
                lines.push(Line::from(vec![
                    text(format!("  {key:<16}"), GOLD),
                    text(description, FG),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(text(
                lang.pick(
                    "  Geometric positions; daylight and atmosphere not simulated.",
                    "  Posiciones geométricas; no simula luz diurna ni atmósfera.",
                ),
                MUTED,
            )));
            lines.push(Line::from(text(
                lang.pick(
                    "  Any other key to return to the sky.",
                    "  Cualquier otra tecla para volver al cielo.",
                ),
                GOLD,
            )));
        }
        Modal::Location => {
            lines.extend([
                Line::from(text(
                    lang.pick(
                        "  The sky changes with your location.",
                        "  El cielo cambia según dónde estés.",
                    ),
                    FG,
                )),
                Line::from(text(
                    lang.pick(
                        "  Enter: Name; latitude; longitude",
                        "  Ingresá: Nombre; latitud; longitud",
                    ),
                    MUTED,
                )),
                Line::from(text(
                    lang.pick(
                        "  South/west negative. Use a decimal point.",
                        "  Sur y oeste negativos. Decimales con punto.",
                    ),
                    MUTED,
                )),
                Line::from(""),
                Line::from(text(
                    format!(
                        "  > {}▏",
                        tail(&app.input, inner.width.saturating_sub(7) as usize)
                    ),
                    GOLD,
                )),
                Line::from(""),
                Line::from(text(
                    lang.pick(
                        "  Example: Buenos Aires; -34.6037; -58.3816",
                        "  Ej.: Buenos Aires; -34.6037; -58.3816",
                    ),
                    MUTED,
                )),
                Line::from(text(
                    lang.pick(
                        "  Ctrl+U clear · Enter save · Esc keep location",
                        "  Ctrl+U borrar · Enter guardar · Esc conservar ubicación",
                    ),
                    FG,
                )),
                Line::from(text(format!("  {}", app.error), Color::Rgb(210, 141, 117))),
            ]);
        }
        Modal::Search => {
            lines.push(Line::from(text(
                format!(
                    "  / {}▏",
                    tail(&app.input, inner.width.saturating_sub(7) as usize)
                ),
                GOLD,
            )));
            lines.push(Line::from(""));
            let matches = app.matches();
            let start = app.search_index.saturating_sub(7);
            for (row, &i) in matches
                .iter()
                .enumerate()
                .skip(start)
                .take(inner.height.saturating_sub(6) as usize)
            {
                let position = app.position(i);
                lines.push(Line::from(text(
                    format!(
                        "  {} {:<22} {:>5.1}°  {}",
                        if row == app.search_index { "›" } else { " " },
                        app.name(i),
                        position.alt,
                        if position.alt >= 0.0 {
                            lang.pick("above horizon", "sobre horizonte")
                        } else {
                            lang.pick("below horizon", "bajo horizonte")
                        }
                    ),
                    if row == app.search_index { GOLD } else { FG },
                )));
            }
            if matches.is_empty() {
                lines.push(Line::from(text(
                    lang.pick("  No objects found.", "  No se encontraron objetos."),
                    MUTED,
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(text(
                lang.pick(
                    "  ↑↓ select · Enter center · Esc back",
                    "  ↑↓ elegir · Enter centrar · Esc volver",
                ),
                MUTED,
            )));
        }
        Modal::None => {}
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn tail(input: &str, width: usize) -> String {
    let mut out = String::new();
    for c in input.chars().rev() {
        let next = format!("{c}{out}");
        if next.width() > width {
            break;
        }
        out = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::Location, i18n::Language};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn rendering_handles_resize_modals_and_selection() {
        let mut app = App::new(
            Location::default(),
            Some("2026-11-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        app.selected = Some(ObjectId::Star(0));
        for (width, height) in [(160, 48), (80, 24), (52, 18), (20, 8), (1, 1)] {
            for modal in [Modal::None, Modal::Help, Modal::Location, Modal::Search] {
                app.modal = modal;
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal.draw(|frame| draw(frame, &mut app)).unwrap();
                let buffer = terminal.backend().buffer();
                assert_eq!(buffer.area.width, width);
            }
        }
    }

    #[test]
    fn search_focus_and_visible_hit_targets_agree() {
        let mut app = App::new(
            Location {
                name: "Test".into(),
                lat: 0.0,
                lon: 0.0,
            },
            Some("2026-01-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        app.input = "Sirius".into();
        app.modal = Modal::Search;
        app.key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        let selected = app.selected.unwrap();
        assert_eq!(app.name(selected), "Sirius");
        assert!(app.position(selected).alt > 0.0);
        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let (_, x, y) = app.visible.iter().find(|(i, _, _)| *i == selected).unwrap();
        assert!((*x as i32 - (app.sky_rect.x + app.sky_rect.width / 2) as i32).abs() <= 1);
        assert!((*y as i32 - (app.sky_rect.y + app.sky_rect.height / 2) as i32).abs() <= 1);
    }

    #[test]
    fn solar_objects_render_select_by_mouse_and_stay_below_horizon() {
        use crossterm::event::{
            KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
        };
        let mut app = App::new(
            Location {
                name: "Test".into(),
                lat: 0.0,
                lon: 0.0,
            },
            Some("2026-01-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        for name in [
            "Luna", "Jupiter", "Sol", "Mercurio", "Venus", "Marte", "Saturno", "Urano", "Neptuno",
        ] {
            app.input = name.into();
            app.modal = Modal::Search;
            app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            let selected = app.selected.unwrap();
            for (width, height) in [(140, 40), (80, 24), (52, 18)] {
                for ascii in [false, true] {
                    app.ascii = ascii;
                    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
                    if width >= 106 {
                        let screen = terminal
                            .backend()
                            .buffer()
                            .content
                            .iter()
                            .map(|cell| cell.symbol())
                            .collect::<String>();
                        let status = if app.position(selected).alt >= 0.0 {
                            "Above the horizon"
                        } else {
                            "Below the horizon"
                        };
                        assert!(screen.contains(status), "missing horizon status for {name}");
                        if name == "Luna" {
                            assert!(screen.contains("Illuminated"));
                        }
                    }
                    assert!(app
                        .visible
                        .iter()
                        .all(|&(id, _, _)| app.position(id).alt >= 0.0));
                    if app.position(selected).alt >= 0.0 {
                        let &(_, x, y) = app
                            .visible
                            .iter()
                            .find(|(id, _, _)| *id == selected)
                            .unwrap();
                        app.selected = None;
                        for kind in [
                            MouseEventKind::Down(MouseButton::Left),
                            MouseEventKind::Up(MouseButton::Left),
                        ] {
                            app.mouse(MouseEvent {
                                kind,
                                column: x,
                                row: y,
                                modifiers: KeyModifiers::NONE,
                            });
                        }
                        assert_eq!(app.selected, Some(selected), "{name} at {width}x{height}");
                    } else {
                        assert!(!app.visible.iter().any(|(id, _, _)| *id == selected));
                    }
                }
            }
        }
    }

    #[test]
    fn english_and_spanish_render_all_dialogs_and_lunar_information() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::new(
            Location {
                name: "Sol del Sur".into(),
                lat: 0.0,
                lon: 0.0,
            },
            Some("2026-01-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        assert_eq!(app.language, Language::En);
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        app.input = "Luna".into();
        app.modal = Modal::Search;
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        for lang in [Language::En, Language::Es] {
            app.language = lang;
            for (modal, expected) in [
                (Modal::None, lang.pick("Illuminated", "Iluminada")),
                (
                    Modal::Help,
                    lang.pick("OBSERVER'S GUIDE", "GUÍA DEL OBSERVADOR"),
                ),
                (
                    Modal::Location,
                    lang.pick("YOUR PLACE IN THE WORLD", "TU LUGAR EN EL MUNDO"),
                ),
                (
                    Modal::Search,
                    lang.pick("SEARCH THE CATALOGUE", "BUSCAR EN EL CATÁLOGO"),
                ),
            ] {
                app.modal = modal;
                terminal.draw(|frame| draw(frame, &mut app)).unwrap();
                let screen = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                assert!(screen.contains(expected), "{lang:?}: {expected}");
                assert!(
                    screen.contains("Sol del Sur"),
                    "user's location name was translated"
                );
            }
            assert_eq!(app.name(app.selected.unwrap()), lang.pick("Moon", "Luna"));
            assert!(app
                .solar
                .iter()
                .find(|s| s.body == Body::Moon)
                .unwrap()
                .phase_name(lang)
                .unwrap()
                .contains(lang.pick("Waxing", "creciente")));
            assert!(Location::parse_localized("Test;NaN;0", lang)
                .err()
                .unwrap()
                .contains(lang.pick("Latitude", "Latitud")));
        }
    }

    #[test]
    fn zoom_labels_real_catalog_names_and_designations() {
        let mut app = App::new(
            Location::default(),
            Some("2026-01-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        app.horizon = false;
        let named = app
            .catalog
            .stars
            .iter()
            .position(|s| s.named && s.mag > 3.2 && s.mag < 3.8)
            .unwrap();
        let designation = app
            .catalog
            .stars
            .iter()
            .position(|s| !s.named && !s.name.starts_with("HIP ") && s.mag > 4.0 && s.mag < 5.0)
            .unwrap();
        let hip = app
            .catalog
            .stars
            .iter()
            .position(|s| s.name.starts_with("HIP ") && s.mag > 5.5)
            .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        for (index, fov) in [(named, 60.0), (designation, 40.0), (hip, 20.0)] {
            let object = ObjectId::Star(index);
            app.camera.az = app.position(object).az;
            app.camera.alt = app.position(object).alt;
            app.selected = None;
            app.labels = true;
            app.camera.fov = 105.0;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert!(!app
                .label_layout
                .placements
                .iter()
                .any(|p| p.object == object));
            app.camera.fov = fov;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert!(
                app.label_layout
                    .placements
                    .iter()
                    .any(|p| p.object == object),
                "missing {}",
                app.name(object)
            );
            let screen = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(screen.contains(app.name(object)));
            app.labels = false;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert!(app.label_layout.placements.is_empty());
            app.selected = Some(object);
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert_eq!(app.label_layout.placements.len(), 1);
            assert_eq!(app.label_layout.placements[0].object, object);
        }
    }

    #[test]
    fn full_sky_reveals_objects_below_ground_and_reaches_nadir() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::new(
            Location {
                name: "Test".into(),
                lat: 0.0,
                lon: 0.0,
            },
            Some("2026-01-01T00:00:00Z".parse().unwrap()),
            false,
            false,
        );
        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        app.input = "Sol".into();
        app.modal = Modal::Search;
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let sun = app.selected.unwrap();
        assert!(app.position(sun).alt < -60.0);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(!app.visible.iter().any(|(id, _, _)| *id == sun));

        app.key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.horizon);
        assert!((app.camera.alt - app.position(sun).alt).abs() < 1e-8);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(app.visible.iter().any(|(id, _, _)| *id == sun));
        assert!(app
            .visible
            .iter()
            .any(|(id, _, _)| matches!(id, ObjectId::Star(_)) && app.position(*id).alt < 0.0));
        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("FULL SKY"));
        assert!(screen.contains("Below the horizon"));
        for _ in 0..80 {
            app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        }
        assert_eq!(app.camera.alt, -90.0);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(!app.visible.is_empty());
        assert!(app
            .visible
            .iter()
            .all(|(id, _, _)| app.position(*id).alt < 0.0));

        app.key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert!(app.horizon);
        assert!(app.camera.alt >= -25.0);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(app
            .visible
            .iter()
            .all(|(id, _, _)| app.position(*id).alt >= 0.0));
    }
}
