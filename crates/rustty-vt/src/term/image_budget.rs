//! Budget mémoire des images posées (écran et historique) : au-delà, les
//! placements les plus anciens de l'historique sont retirés d'abord.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::Term;
use crate::graphics::{ImageStrip, Placement};
use crate::line::Line;

/// Octets RGBA maximum des images posées par terminal (320 Mio).
pub const PLACED_BUDGET: usize = 320 * 1024 * 1024;

impl Term {
    /// Octets RGBA des images posées, chaque image comptée une fois.
    #[cfg(test)]
    pub(crate) fn placed_bytes(&self) -> usize {
        self.placement_usage().sizes.values().sum()
    }

    /// Retire d'abord les placements absents de l'écran, du plus ancien au
    /// plus récent, puis ceux de l'écran ; `newest` reste toujours affiché.
    /// Une image partagée par plusieurs placements n'est libérée qu'avec le
    /// dernier d'entre eux.
    pub(super) fn enforce_placed_budget(&mut self, newest: &Arc<Placement>) {
        let usage = self.placement_usage();
        let mut total: usize = usage.sizes.values().sum();
        if total <= self.placed_budget {
            return;
        }
        let mut users = usage.users;
        let (history, screen): (Vec<_>, Vec<_>) = usage
            .order
            .into_iter()
            .filter(|p| !Arc::ptr_eq(p, newest))
            .partition(|p| !usage.on_screen.contains(&Arc::as_ptr(p)));
        let mut victims = HashSet::new();
        for p in history.into_iter().chain(screen) {
            if total <= self.placed_budget {
                break;
            }
            victims.insert(Arc::as_ptr(&p));
            let id = p.image.id;
            let left = users.get_mut(&id).map(|n| {
                *n -= 1;
                *n
            });
            if left == Some(0) {
                total -= usage.sizes[&id];
            }
        }
        self.drop_placements(&victims);
    }

    /// Placements distincts dans l'ordre d'apparition, avec leur présence à
    /// l'écran, la taille de chaque image et son nombre de placements.
    fn placement_usage(&self) -> Usage {
        let mut usage = Usage::default();
        let mut seen = HashSet::new();
        for (line, screen) in self.placed_lines() {
            for s in line.images() {
                let ptr = Arc::as_ptr(&s.placement);
                if screen {
                    usage.on_screen.insert(ptr);
                }
                if seen.insert(ptr) {
                    let image = &s.placement.image;
                    usage.sizes.insert(image.id, image.rgba.len());
                    *usage.users.entry(image.id).or_default() += 1;
                    usage.order.push(Arc::clone(&s.placement));
                }
            }
        }
        usage
    }

    fn drop_placements(&mut self, victims: &HashSet<*const Placement>) {
        let mut drop = |s: &ImageStrip| victims.contains(&Arc::as_ptr(&s.placement));
        for line in self.scrollback.iter_mut() {
            line.drop_strips_where(&mut drop);
        }
        for grid in [&mut self.grid, &mut self.alt_grid] {
            for row in 0..grid.rows() {
                grid.line_mut(row).drop_strips_where(&mut drop);
            }
        }
    }

    /// Lignes portant des images, de la plus ancienne à la plus récente, avec
    /// vrai si elles sont à l'écran (principal ou alternatif).
    fn placed_lines(&self) -> impl Iterator<Item = (&Line, bool)> {
        let history = self.scrollback.iter().map(|l| (l, false));
        let screens = self.grid.lines().iter().chain(self.alt_grid.lines());
        history.chain(screens.map(|l| (l, true)))
    }
}

#[derive(Default)]
struct Usage {
    order: Vec<Arc<Placement>>,
    on_screen: HashSet<*const Placement>,
    sizes: HashMap<u64, usize>,
    users: HashMap<u64, usize>,
}
