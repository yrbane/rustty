//! La fenêtre système et tout ce qu'elle porte : surface, renderer,
//! terminaux, modèle. Les réactions aux entrées sont dans `input.rs`, les
//! effets dans `effects.rs` ; ici, création, mise en page et rendu.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use rustty_config::{Config, Mods, TabBarPosition};
use rustty_layout::WindowId;
use rustty_pty::Shell;
use rustty_render::{
    CellMetrics, Chrome, FontSet, GpuContext, HoverTarget, Palette, PixelRect, Renderer,
    TabBarLayout, TabBarStyle, layout_tab_bar, tab_bar_chrome, tab_bar_height,
};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Icon, Window};

use crate::banner::{Banner, banner_chrome, banner_height};
use crate::events::Waker;
use crate::geometry;
use crate::gpu_surface::{self, Surface};
use crate::model::Model;
use crate::mouse::{MouseButton, Selection};
use crate::render_frame::{self, PaneView};
use crate::tab::TermId;
use crate::term_window::TermWindow;
use crate::title;

const LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo.png");
const DEFAULT_SIZE: LogicalSize<f64> = LogicalSize::new(960.0, 600.0);
/// Taille de police de la config en points typographiques → pixels à 96 dpi.
const PT_TO_PX: f32 = 96.0 / 72.0;
/// Alpha de la surbrillance de sélection posée par-dessus le texte.
const SELECTION_ALPHA: f32 = 0.4;

pub struct OsWindow {
    pub window: Arc<Window>,
    pub ctx: GpuContext,
    pub surface: Surface,
    pub renderer: Renderer,
    pub palette: Palette,
    pub tab_style: TabBarStyle,
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub model: Model,
    pub terms: HashMap<TermId, TermWindow>,
    pub titles: HashMap<TermId, String>,
    pub modifiers: Mods,
    pub cursor: (f64, f64),
    pub selection: Option<(TermId, Selection)>,
    pub dragging: bool,
    pub held: Option<MouseButton>,
    pub press_target: HoverTarget,
    pub tab_bar: Option<TabBarLayout>,
    pub pane_rects: Vec<(WindowId, PixelRect)>,
    pub focused: bool,
    pub clipboard: Option<arboard::Clipboard>,
    pub shell: Shell,
    pub waker: Waker,
}

impl OsWindow {
    pub fn open(
        event_loop: &ActiveEventLoop,
        config: Config,
        hold: bool,
        config_error: Option<String>,
        config_path: Option<PathBuf>,
        waker: Waker,
    ) -> anyhow::Result<Self> {
        let attrs = Window::default_attributes()
            .with_title(title::window_title("", crate::VERSION))
            .with_inner_size(DEFAULT_SIZE)
            .with_transparent(true)
            .with_window_icon(load_icon());
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .context("création de la fenêtre")?,
        );
        let (ctx, raw_surface) = gpu_surface::init_gpu(Arc::clone(&window))?;
        let size = window.inner_size();
        let surface = Surface::new(raw_surface, &ctx, size.width, size.height)?;
        let palette = Palette::from_config(&config.colors, config.font.bold_is_bright);
        let fonts = load_fonts(&config, window.scale_factor());
        let renderer = Renderer::new(
            &ctx,
            surface.view_format(),
            fonts,
            palette.clone(),
            config.window.padding,
        );
        let tab_style = TabBarStyle::from_config(
            &palette,
            &config.tabs.close_button_style,
            config.tabs.close_button,
        );
        let (mut model, first) = Model::new(config.window.opacity, hold);
        if let Some(e) = config_error {
            model.set_notice(Some(Banner::error(format!("configuration : {e}"))));
        }
        let clipboard = arboard::Clipboard::new()
            .map_err(|e| tracing::warn!("presse-papiers indisponible : {e}"))
            .ok();
        let mut this = Self {
            window,
            ctx,
            surface,
            renderer,
            palette,
            tab_style,
            config,
            config_path,
            model,
            terms: HashMap::new(),
            titles: HashMap::new(),
            modifiers: Mods::empty(),
            cursor: (0.0, 0.0),
            selection: None,
            dragging: false,
            held: None,
            press_target: HoverTarget::None,
            tab_bar: None,
            pane_rects: Vec::new(),
            focused: true,
            clipboard,
            shell: Shell::default_for_platform(),
            waker,
        };
        this.spawn_term(first)?;
        this.relayout();
        Ok(this)
    }

    pub fn metrics(&self) -> CellMetrics {
        self.renderer.metrics()
    }

    pub fn spawn_term(&mut self, id: TermId) -> anyhow::Result<()> {
        let tw = TermWindow::spawn(
            id,
            &self.shell,
            80,
            24,
            (0, 0),
            self.config.window.scrollback_lines,
            Arc::clone(&self.waker),
        )
        .with_context(|| format!("lancement de {}", self.shell.program))?;
        self.terms.insert(id, tw);
        Ok(())
    }

    /// Titres des onglets selon le gabarit, à partir du terminal focalisé de chacun.
    pub fn tab_titles(&self) -> Vec<String> {
        let template = &self.config.tabs.title_template;
        self.model
            .workspace
            .tabs()
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let raw = tab
                    .focused_term()
                    .and_then(|t| self.titles.get(&t))
                    .map(String::as_str)
                    .unwrap_or("");
                title::tab_title(template, i, raw)
            })
            .collect()
    }

    pub fn update_title(&self) {
        let raw = self
            .model
            .workspace
            .focused_term()
            .and_then(|t| self.titles.get(&t))
            .map(String::as_str)
            .unwrap_or("");
        self.window
            .set_title(&title::window_title(raw, crate::VERSION));
    }

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
            tab_bar_height(metrics),
        );
        let titles = self.tab_titles();
        let active = self.model.workspace.active();
        self.tab_bar = g.tab_bar_y.map(|y| {
            layout_tab_bar(
                w,
                y,
                &render_frame::tab_specs(&titles, active),
                &self.tab_style,
                metrics,
            )
        });
        if self.model.workspace.is_empty() {
            self.pane_rects.clear();
            return;
        }
        let padding = self.config.window.padding;
        let tab = self.model.workspace.active_tab();
        self.pane_rects = geometry::pane_rects(&tab.layout, g.content);
        for (wid, rect) in &self.pane_rects {
            if let Some(term) = tab.term_at(*wid)
                && let Some(tw) = self.terms.get_mut(&term)
            {
                let (cols, rows) = geometry::grid_size(*rect, metrics, padding);
                tw.resize(cols, rows, (rect.width, rect.height));
            }
        }
        self.update_title();
    }

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
        if let Some(bar) = &self.tab_bar {
            let titles = self.tab_titles();
            let specs = render_frame::tab_specs(&titles, self.model.workspace.active());
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
                    tab_bar_height(metrics)
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

    /// Applique une configuration rechargée : palette, police, onglets, opacité, raccourcis.
    pub fn apply_config(&mut self, new: Config) {
        let font_changed = new.font.family != self.config.font.family
            || new.font.size != self.config.font.size
            || new.window.padding != self.config.window.padding;
        self.palette = Palette::from_config(&new.colors, new.font.bold_is_bright);
        self.tab_style = TabBarStyle::from_config(
            &self.palette,
            &new.tabs.close_button_style,
            new.tabs.close_button,
        );
        self.model.opacity = new.window.opacity;
        self.config = new;
        if font_changed {
            self.rebuild_fonts();
        } else {
            self.renderer.set_palette(self.palette.clone());
        }
        self.relayout();
        self.window.request_redraw();
    }

    /// Nouveau renderer avec la police courante (changement de police ou d'échelle d'écran).
    pub fn rebuild_fonts(&mut self) {
        let fonts = load_fonts(&self.config, self.window.scale_factor());
        self.renderer = Renderer::new(
            &self.ctx,
            self.surface.view_format(),
            fonts,
            self.palette.clone(),
            self.config.window.padding,
        );
    }
}

fn load_fonts(config: &Config, scale_factor: f64) -> FontSet {
    let px = config.font.size * PT_TO_PX * scale_factor as f32;
    FontSet::load(&config.font.family, px).unwrap_or_else(|e| {
        tracing::warn!("police « {} » : {e} — police embarquée", config.font.family);
        FontSet::embedded(px)
    })
}

fn load_icon() -> Option<Icon> {
    let image = image::load_from_memory(LOGO_PNG)
        .map_err(|e| tracing::warn!("icône : {e}"))
        .ok()?
        .to_rgba8();
    let (w, h) = image.dimensions();
    Icon::from_rgba(image.into_raw(), w, h)
        .map_err(|e| tracing::warn!("icône : {e}"))
        .ok()
}
