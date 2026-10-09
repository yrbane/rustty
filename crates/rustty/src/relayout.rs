//! Mise en page de la fenêtre : barre d'onglets, rectangles des panneaux et
//! des barres de split, renderers des tailles de police en usage, taille de
//! la grille de chaque terminal.

use rustty_render::{layout_tab_bar, tab_bar_height};

use crate::geometry;
use crate::pane_fonts::SizeKey;
use crate::render_frame;
use crate::renderers::RendererSpec;
use crate::window_state::OsWindow;

impl OsWindow {
    /// Recalcule barre d'onglets, rectangles des panneaux et tailles des terminaux.
    pub fn relayout(&mut self) {
        let (w, h) = self.surface.size();
        let metrics = self.metrics();
        let tabs = &self.config.tabs;
        let g = geometry::window_geometry(
            w,
            h,
            tabs.position,
            self.model.workspace.tabs().len(),
            tabs.min_tabs,
            tab_bar_height(metrics, &self.tab_style),
        );
        let titles = self.tab_titles();
        let active = self.model.workspace.active();
        self.tab_bar = g.tab_bar_y.map(|y| {
            layout_tab_bar(
                w,
                y,
                &render_frame::tab_specs(&titles, active, None),
                &self.tab_style,
                metrics,
            )
        });
        if self.model.workspace.is_empty() {
            self.pane_rects.clear();
            self.dividers.clear();
            return;
        }
        let padding = self.config.window.padding;
        let gap = geometry::split_gap(&self.config.splits);
        let keys = self.pane_fonts.keys_in_use(self.terms.keys().copied());
        let spec = RendererSpec {
            ctx: &self.ctx,
            format: self.surface.view_format(),
            family: &self.config.font.family,
            scale_factor: self.window.scale_factor(),
            palette: &self.palette,
            padding,
        };
        self.renderers.sync(&spec, &keys);
        let tab = self.model.workspace.active_tab();
        self.pane_rects = geometry::pane_rects(&tab.layout, g.content, gap);
        self.dividers = geometry::divider_rects(&tab.layout, g.content, gap);
        self.content = g.content;
        for (wid, rect) in &self.pane_rects {
            if let Some(term) = tab.term_at(*wid)
                && let Some(tw) = self.terms.get_mut(&term)
            {
                let metrics = self
                    .renderers
                    .metrics(SizeKey::of(self.pane_fonts.size_of(term)));
                let (cols, rows) = geometry::grid_size(*rect, metrics, padding);
                tw.resize(
                    cols,
                    rows,
                    (rect.width, rect.height),
                    (metrics.width, metrics.height),
                );
            }
        }
        self.update_title();
    }
}
