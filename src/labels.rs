//! Progressive, cell-aligned labels with zoom hysteresis and stable anchors.
use crate::app::ObjectId;
use ratatui::layout::Rect;

#[derive(Clone, Copy)]
pub struct Candidate {
    pub object: ObjectId,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub magnitude: f64,
    pub named: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub object: ObjectId,
    pub rect: Rect,
    anchor: usize,
}

#[derive(Default)]
pub struct LabelLayout {
    level: usize,
    pub placements: Vec<Placement>,
}

impl LabelLayout {
    pub fn update(
        &mut self,
        area: Rect,
        fov: f64,
        enabled: bool,
        selected: Option<ObjectId>,
        candidates: &[Candidate],
    ) {
        let thresholds = [75.0, 45.0, 25.0];
        while self.level < 3 && fov <= thresholds[self.level] {
            self.level += 1;
        }
        // A small dead band avoids flashing names when zoom oscillates at a boundary.
        while self.level > 0 && fov > thresholds[self.level - 1] + 3.0 {
            self.level -= 1;
        }
        let magnitude_limit = [2.6, 3.8, 5.0, 6.0][self.level];
        let budget = (area.width as usize * area.height as usize / 100).clamp(4, 64);
        let retained = |id| self.placements.iter().any(|p| p.object == id);
        let mut eligible: Vec<_> = candidates
            .iter()
            .filter(|c| {
                selected == Some(c.object)
                    || (enabled
                        && (matches!(c.object, ObjectId::Solar(_))
                            || (c.magnitude <= magnitude_limit && (c.named || self.level >= 2))))
            })
            .collect();
        eligible.sort_by(|a, b| {
            let rank = |c: &Candidate| {
                (
                    selected != Some(c.object),
                    matches!(c.object, ObjectId::Star(_)),
                    c.magnitude.floor() as i32,
                    !retained(c.object),
                )
            };
            rank(a)
                .cmp(&rank(b))
                .then_with(|| a.magnitude.total_cmp(&b.magnitude))
        });
        let mut placed: Vec<Placement> = Vec::new();
        for candidate in eligible {
            if placed.len() >= budget {
                break;
            }
            let previous = self
                .placements
                .iter()
                .find(|p| p.object == candidate.object)
                .map(|p| p.anchor);
            let anchors = previous
                .into_iter()
                .chain((0..8).filter(|a| Some(*a) != previous));
            let width = candidate.width.min(area.width);
            let mut chosen = None;
            for anchor in anchors {
                let (x, y) = anchor_position(candidate, width, anchor);
                if x < area.x as i32
                    || y < area.y as i32
                    || x + width as i32 > area.right() as i32
                    || y >= area.bottom() as i32
                {
                    continue;
                }
                let rect = Rect::new(x as u16, y as u16, width, 1);
                if placed.iter().any(|p| conflicts(p.rect, rect)) {
                    continue;
                }
                // Don't cover another object's hit target (including unnamed stars).
                if candidates.iter().any(|c| rect.contains((c.x, c.y).into())) {
                    continue;
                }
                chosen = Some(Placement {
                    object: candidate.object,
                    rect,
                    anchor,
                });
                break;
            }
            // The selection gets first priority, including at the edges. In a
            // crowded field its fallback may cover a dot, never another label.
            if chosen.is_none()
                && selected == Some(candidate.object)
                && area.width > 0
                && area.height > 0
            {
                let x = candidate
                    .x
                    .saturating_add(2)
                    .clamp(area.x, area.right() - width);
                let y = if candidate.y > area.y {
                    candidate.y - 1
                } else {
                    (candidate.y + 1).min(area.bottom() - 1)
                };
                chosen = Some(Placement {
                    object: candidate.object,
                    rect: Rect::new(x, y, width, 1),
                    anchor: 0,
                });
            }
            if let Some(placement) = chosen {
                placed.push(placement);
            }
        }
        self.placements = placed;
    }
}

fn anchor_position(c: &Candidate, width: u16, anchor: usize) -> (i32, i32) {
    let (x, y, w) = (c.x as i32, c.y as i32, width as i32);
    match anchor {
        0 => (x + 2, y - 1),
        1 => (x + 2, y),
        2 => (x + 2, y + 1),
        3 => (x - w - 1, y - 1),
        4 => (x - w - 1, y),
        5 => (x - w - 1, y + 1),
        6 => (x - w / 2, y - 2),
        _ => (x - w / 2, y + 2),
    }
}

fn conflicts(a: Rect, b: Rect) -> bool {
    a.x <= b.right() && b.x <= a.right() && a.y.abs_diff(b.y) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(i: usize, x: u16, y: u16, magnitude: f64, named: bool) -> Candidate {
        Candidate {
            object: ObjectId::Star(i),
            x,
            y,
            magnitude,
            named,
            width: 10,
        }
    }

    #[test]
    fn zoom_reveals_names_then_designations_without_boundary_flicker() {
        let area = Rect::new(0, 0, 120, 40);
        let candidates = [
            star(0, 5, 5, 2.0, true),
            star(1, 30, 12, 3.5, true),
            star(2, 60, 20, 4.8, false),
            star(3, 90, 30, 5.9, false),
        ];
        let mut layout = LabelLayout::default();
        for (fov, expected) in [
            (105.0, 1),
            (75.0, 2),
            (76.0, 2),
            (79.0, 1),
            (45.0, 3),
            (25.0, 4),
            (28.1, 3),
        ] {
            layout.update(area, fov, true, None, &candidates);
            assert_eq!(layout.placements.len(), expected, "fov {fov}");
        }
        layout.update(area, 12.0, false, Some(ObjectId::Star(3)), &candidates);
        assert_eq!(layout.placements.len(), 1);
        assert_eq!(layout.placements[0].object, ObjectId::Star(3));
    }

    #[test]
    fn crowded_labels_fit_budget_keep_selection_and_do_not_overlap() {
        let area = Rect::new(10, 5, 60, 20);
        let mut candidates: Vec<_> = (0..100)
            .map(|i| star(i, 12 + (i % 10) as u16 * 5, 7 + (i / 10) as u16, 4.0, false))
            .collect();
        candidates.push(star(100, area.right() - 1, area.y, 6.0, false));
        let mut layout = LabelLayout::default();
        layout.update(area, 12.0, true, Some(ObjectId::Star(100)), &candidates);
        assert_eq!(layout.placements[0].object, ObjectId::Star(100));
        assert!(layout.placements.len() <= 12);
        for (i, p) in layout.placements.iter().enumerate() {
            assert_eq!(area.intersection(p.rect), p.rect);
            assert!(layout.placements[i + 1..]
                .iter()
                .all(|q| !conflicts(p.rect, q.rect)));
        }
    }

    #[test]
    fn small_pan_keeps_the_chosen_side_of_a_label() {
        let mut layout = LabelLayout::default();
        let area = Rect::new(0, 0, 60, 20);
        let mut c = star(0, 58, 10, 1.0, true);
        layout.update(area, 105.0, true, None, &[c]);
        let previous = layout.placements[0];
        c.x -= 1;
        layout.update(area, 105.0, true, None, &[c]);
        assert_eq!(layout.placements[0].anchor, previous.anchor);
        assert_eq!(layout.placements[0].rect.x, previous.rect.x - 1);
    }
}
