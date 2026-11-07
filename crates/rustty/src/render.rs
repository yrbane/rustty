//! Une image de la fenêtre : panneaux, sélection, barres de split, barre
//! d'onglets, bandeau, fond avec opacité, puis présentation.

use std::collections::BTreeMap;

use rustty_config::TabBarPosition;
use rustty_render::{CellMetrics, Chrome, tab_bar_chrome, tab_bar_height};

use crate::appearance;
use crate::banner::{banner_chrome, banner_height};
use crate::gpu_surface::{Acquire, Surface};
use crate::pane_fonts::SizeKey;
use crate::render_frame::{self, PaneView, Pass};
use crate::window_state::{OsWindow, SELECTION_ALPHA};

/// Une image assemblée, prête à présenter : les panneaux et le chrome de la
/// taille de base, les autres tailles, l'overlay (aperçu de glisser,
/// bandeau) et les surgissants (menu, correcteur, infobulle).
pub struct Assembled {
    pub base: (Vec<PaneView>, Chrome),
    pub sized: BTreeMap<SizeKey, (Vec<PaneView>, Chrome)>,
    pub overlay: Chrome,
    pub popups: Chrome,
}

impl OsWindow {
    pub fn render(&mut self) {
        self.flush_layout();
        let texture = match self.surface.acquire(&self.ctx) {
            Acquire::Frame(texture) => texture,
            Acquire::Retry => {
                self.window.request_redraw();
                return;
            }
            Acquire::Skip => return,
        };
        let assembled = self.assemble();
        self.present(texture, assembled);
    }

    /// Tout ce qu'une image contient, sans toucher au GPU.
    pub fn assemble(&self) -> Assembled {
        let (w, h) = self.surface.size();
        let metrics = self.metrics();
        let mut sized = self.pane_groups();
        let (panes, mut chrome) = sized.remove(&self.renderers.base_key()).unwrap_or_default();
        self.base_chrome(&mut chrome, metrics);
        Assembled {
            base: (panes, chrome),
            sized,
            overlay: self.overlay_chrome((w, h), metrics),
            popups: self.popup_chrome(),
        }
    }

    /// Panneaux (et leur sélection) regroupés par taille de police : une
    /// passe de rendu par taille, la base en premier.
    fn pane_groups(&self) -> BTreeMap<SizeKey, (Vec<PaneView>, Chrome)> {
        let padding = self.config.window.padding;
        let mut groups: BTreeMap<SizeKey, (Vec<PaneView>, Chrome)> = BTreeMap::new();
        if self.model.workspace.is_empty() {
            return groups;
        }
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
                link_hover: self.link_range_of(term),
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
        groups
    }

    /// Barres de split, bandeaux de panneau, barre d'onglets, barre de titre.
    fn base_chrome(&self, chrome: &mut Chrome, metrics: CellMetrics) {
        if !self.model.workspace.is_empty() {
            let tab = self.model.workspace.active_tab();
            let (splits, palette) = (&self.config.splits, &self.palette);
            chrome
                .quads
                .extend(render_frame::divider_quads(&self.dividers, |id| {
                    appearance::divider_color(splits, palette, tab, id)
                }));
        }
        let headers = self.header_chrome();
        chrome.quads.extend(headers.quads);
        chrome.texts.extend(headers.texts);
        if let Some(bar) = &self.tab_bar {
            let titles = self.tab_titles();
            let accents = appearance::tab_accents(
                &self.config.tabs,
                &self.palette,
                self.model.workspace.tabs(),
            );
            let specs = render_frame::tab_specs(&titles, self.model.workspace.active(), &accents);
            let bar_chrome =
                tab_bar_chrome(bar, &specs, &self.tab_style, metrics, self.model.hover);
            chrome.quads.extend(bar_chrome.quads);
            chrome.texts.extend(bar_chrome.texts);
        }
        if let Some(title_bar) = self.title_bar_chrome() {
            chrome.shapes.extend(title_bar.shapes);
            chrome.texts.extend(title_bar.texts);
            chrome.icons.extend(title_bar.icons);
        }
    }

    /// Aperçu de glisser et bandeau de notification.
    fn overlay_chrome(&self, (w, h): (u32, u32), metrics: CellMetrics) -> Chrome {
        let mut overlay = Chrome::default();
        overlay.quads.extend(self.drag_preview_quads());
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
        overlay
    }

    /// Menu du clic droit, infobulle du correcteur, infobulle de lien.
    fn popup_chrome(&self) -> Chrome {
        let mut menu_chrome = self.context_menu_chrome().unwrap_or_default();
        if let Some(popup) = self.spell_chrome() {
            menu_chrome.quads.extend(popup.quads);
            menu_chrome.texts.extend(popup.texts);
        }
        if let Some(tip) = self.link_chrome() {
            menu_chrome.quads.extend(tip.quads);
            menu_chrome.texts.extend(tip.texts);
        }
        menu_chrome
    }

    /// Les passes de rendu puis la présentation : base, autres tailles,
    /// overlay, surgissants.
    fn present(&mut self, texture: wgpu::SurfaceTexture, assembled: Assembled) {
        let view = Surface::view(&texture, self.surface.view_format());
        let (w, h) = self.surface.size();
        let base_key = self.renderers.base_key();
        let background = render_frame::background_color(
            self.palette.background,
            self.model.opacity,
            self.surface.premultiplied(),
        );
        let Assembled {
            base,
            mut sized,
            overlay,
            popups,
        } = assembled;
        let mut base = Some(base);
        let mut overlay = Some(overlay);
        let mut popups = Some(popups);
        let keys: Vec<SizeKey> = sized.keys().copied().collect();
        for pass in render_frame::pass_order(base_key, keys) {
            let (key, (panes, chrome)) = match pass {
                Pass::Base => (base_key, base.take().unwrap_or_default()),
                Pass::Size(key) => (key, sized.remove(&key).unwrap_or_default()),
                Pass::Overlay => (base_key, (Vec::new(), overlay.take().unwrap_or_default())),
                Pass::Menu => (base_key, (Vec::new(), popups.take().unwrap_or_default())),
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
