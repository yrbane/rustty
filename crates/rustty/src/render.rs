//! Une image de la fenêtre : panneaux, sélection, barres de split, barre
//! d'onglets, bandeau, fond avec opacité, puis présentation.

use std::collections::BTreeMap;

use rustty_config::TabBarPosition;
use rustty_render::{Chrome, tab_bar_chrome, tab_bar_height};

use crate::appearance;
use crate::banner::{banner_chrome, banner_height};
use crate::gpu_surface::Surface;
use crate::pane_fonts::SizeKey;
use crate::render_frame::{self, PaneView, Pass};
use crate::window_state::{OsWindow, SELECTION_ALPHA};

impl OsWindow {
    pub fn render(&mut self) {
        let Some(texture) = self.surface.acquire(&self.ctx) else {
            return;
        };
        let view = Surface::view(&texture, self.surface.view_format());
        let (w, h) = self.surface.size();
        let metrics = self.metrics();
        let padding = self.config.window.padding;
        let base_key = self.renderers.base_key();
        // Panneaux (et leur sélection) regroupés par taille de police : une
        // passe de rendu par taille, la base en premier.
        let mut groups: BTreeMap<SizeKey, (Vec<PaneView>, Chrome)> = BTreeMap::new();
        if !self.model.workspace.is_empty() {
            let tab = self.model.workspace.active_tab();
            let focused_window = tab.layout.focused();
            for (wid, rect) in &self.pane_rects {
                let Some(term) = tab.term_at(*wid) else {
                    continue;
                };
                let Some(tw) = self.terms.get(&term) else {
                    continue;
                };
                let key = SizeKey::of(self.pane_fonts.size_of(term));
                let (panes, chrome) = groups.entry(key).or_default();
                panes.push(PaneView {
                    rect: *rect,
                    snapshot: tw.snapshot(),
                    focused: self.focused && Some(*wid) == focused_window,
                });
                if let Some((sel_term, sel)) = &self.selection
                    && *sel_term == term
                    && !sel.is_empty()
                {
                    let color = self.palette.selection.with_alpha(SELECTION_ALPHA);
                    let pane_metrics = self.renderers.metrics(key);
                    chrome.quads.extend(render_frame::selection_quads(
                        sel,
                        *rect,
                        pane_metrics,
                        padding,
                        color,
                    ));
                }
            }
        }
        let (panes, mut chrome) = groups.remove(&base_key).unwrap_or_default();
        if !self.model.workspace.is_empty() {
            let tab = self.model.workspace.active_tab();
            let (splits, palette) = (&self.config.splits, &self.palette);
            chrome
                .quads
                .extend(render_frame::divider_quads(&self.dividers, |id| {
                    appearance::divider_color(splits, palette, tab, id)
                }));
        }
        if let Some(bar) = &self.tab_bar {
            let titles = self.tab_titles();
            let accents = appearance::tab_accents(
                &self.config.tabs,
                &self.palette,
                self.model.workspace.tabs(),
            );
            let specs =
                render_frame::tab_specs(&titles, self.model.workspace.active(), accents.as_deref());
            let bar_chrome =
                tab_bar_chrome(bar, &specs, &self.tab_style, metrics, self.model.hover);
            chrome.quads.extend(bar_chrome.quads);
            chrome.texts.extend(bar_chrome.texts);
        }
        let mut overlay = Chrome::default();
        if let Some(banner) = self.model.banner() {
            let bar_at_bottom =
                self.tab_bar.is_some() && self.config.tabs.position == TabBarPosition::Bottom;
            let reserved = banner_height(metrics)
                + if bar_at_bottom {
                    tab_bar_height(metrics, &self.tab_style)
                } else {
                    0
                };
            let b = banner_chrome(
                &banner,
                w,
                h.saturating_sub(reserved),
                metrics,
                &self.palette,
            );
            overlay.quads.extend(b.quads);
            overlay.texts.extend(b.texts);
        }
        let background = render_frame::background_color(
            self.palette.background,
            self.model.opacity,
            self.surface.premultiplied(),
        );
        let mut base_pass = Some((panes, chrome));
        let mut overlay = Some(overlay);
        let keys: Vec<SizeKey> = groups.keys().copied().collect();
        for pass in render_frame::pass_order(base_key, keys) {
            let (key, (panes, chrome)) = match pass {
                Pass::Base => (base_key, base_pass.take().unwrap_or_default()),
                Pass::Size(key) => (key, groups.remove(&key).unwrap_or_default()),
                Pass::Overlay => (base_key, (Vec::new(), overlay.take().unwrap_or_default())),
            };
            let frame = render_frame::build_frame((w, h), background, &panes, chrome);
            let Some(renderer) = self.renderers.get_mut(key) else {
                continue;
            };
            if pass == Pass::Base {
                renderer.render(&self.ctx, &view, &frame);
            } else {
                renderer.render_onto(&self.ctx, &view, &frame);
            }
        }
        self.ctx.queue.present(texture);
    }
}
