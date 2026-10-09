//! Une image de la fenêtre : panneaux, sélection, barres de split, barre
//! d'onglets, bandeau, fond avec opacité, puis présentation.

use rustty_config::TabBarPosition;
use rustty_render::{Chrome, tab_bar_chrome, tab_bar_height};

use crate::appearance;
use crate::banner::{banner_chrome, banner_height};
use crate::gpu_surface::Surface;
use crate::render_frame::{self, PaneView};
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
        let mut panes = Vec::new();
        let mut chrome = Chrome::default();
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
                    chrome.quads.extend(render_frame::selection_quads(
                        sel, *rect, metrics, padding, color,
                    ));
                }
            }
        }
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
            chrome.quads.extend(b.quads);
            chrome.texts.extend(b.texts);
        }
        let background = render_frame::background_color(
            self.palette.background,
            self.model.opacity,
            self.surface.premultiplied(),
        );
        let frame = render_frame::build_frame((w, h), background, &panes, chrome);
        self.renderer.render(&self.ctx, &view, &frame);
        self.ctx.queue.present(texture);
    }
}
