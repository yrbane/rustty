//! La fenêtre système et tout ce qu'elle porte : surface, renderer,
//! terminaux, modèle. Les réactions aux entrées sont dans `input.rs`, les
//! effets dans `effects.rs` ; ici, création, mise en page et rendu.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use rustty_config::{Config, Mods};
use rustty_layout::{SplitId, WindowId};
use rustty_pty::Shell;
use rustty_render::{
    CellMetrics, FontSet, GpuContext, HoverTarget, Palette, PixelRect, Renderer, TabBarLayout,
    TabBarStyle, layout_tab_bar, tab_bar_height,
};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Icon, Window, WindowAttributes};

use crate::banner::Banner;
use crate::events::Waker;
use crate::geometry;
use crate::gpu_surface::{self, Surface};
use crate::model::Model;
use crate::mouse::{DoubleClick, MouseButton, Selection, WheelAccumulator};
use crate::render_frame;
use crate::tab::TermId;
use crate::term_window::TermWindow;
use crate::title;

/// Icône de fenêtre (X11, Windows) ; Wayland passe par le `.desktop`.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/rustty-256.png");
/// Identifiant d'application : `app_id` Wayland et `WM_CLASS` X11.
const APP_ID: &str = "rustty";
const DEFAULT_SIZE: LogicalSize<f64> = LogicalSize::new(960.0, 600.0);
/// Taille de police de la config en points typographiques → pixels à 96 dpi.
const PT_TO_PX: f32 = 96.0 / 72.0;
/// Alpha de la surbrillance de sélection posée par-dessus le texte.
pub(crate) const SELECTION_ALPHA: f32 = 0.4;

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
    pub tab_clicks: DoubleClick,
    pub wheel: WheelAccumulator,
    pub press_target: HoverTarget,
    pub tab_bar: Option<TabBarLayout>,
    pub pane_rects: Vec<(WindowId, PixelRect)>,
    pub dividers: Vec<(SplitId, PixelRect)>,
    /// Taille de police courante en points (zoom compris).
    pub font_size: f32,
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
        let attrs = with_app_id(attrs);
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .context("création de la fenêtre")?,
        );
        let (ctx, raw_surface) = gpu_surface::init_gpu(Arc::clone(&window))?;
        let size = window.inner_size();
        let surface = Surface::new(raw_surface, &ctx, size.width, size.height)?;
        let palette = Palette::from_config(&config.colors, config.font.bold_is_bright);
        let font_size = config.font.size;
        let fonts = load_fonts(&config.font.family, config.font.size, window.scale_factor());
        let renderer = Renderer::new(
            &ctx,
            surface.view_format(),
            fonts,
            palette.clone(),
            config.window.padding,
        );
        let tab_style = TabBarStyle::from_config(&palette, &config.tabs);
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
            tab_clicks: DoubleClick::default(),
            wheel: WheelAccumulator::default(),
            press_target: HoverTarget::None,
            tab_bar: None,
            pane_rects: Vec::new(),
            dividers: Vec::new(),
            font_size,
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
                let editing = self
                    .model
                    .renaming
                    .as_ref()
                    .filter(|r| r.tab == i)
                    .map(|r| r.buffer.as_str());
                title::display_title(template, i, raw, tab.custom_title.as_deref(), editing)
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
        let tab = self.model.workspace.active_tab();
        self.pane_rects = geometry::pane_rects(&tab.layout, g.content, gap);
        self.dividers = geometry::divider_rects(&tab.layout, g.content, gap);
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

    /// Applique une configuration rechargée : palette, police, onglets, opacité, raccourcis.
    pub fn apply_config(&mut self, new: Config) {
        if new.font.size != self.config.font.size {
            // La taille configurée change : le zoom repart de la nouvelle base.
            self.font_size = new.font.size;
        }
        let font_changed = new.font.family != self.config.font.family
            || new.font.size != self.config.font.size
            || new.window.padding != self.config.window.padding;
        self.palette = Palette::from_config(&new.colors, new.font.bold_is_bright);
        self.tab_style = TabBarStyle::from_config(&self.palette, &new.tabs);
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
        let fonts = load_fonts(
            &self.config.font.family,
            self.font_size,
            self.window.scale_factor(),
        );
        self.renderer = Renderer::new(
            &self.ctx,
            self.surface.view_format(),
            fonts,
            self.palette.clone(),
            self.config.window.padding,
        );
    }
}

fn load_fonts(family: &str, size_pt: f32, scale_factor: f64) -> FontSet {
    let px = size_pt * PT_TO_PX * scale_factor as f32;
    FontSet::load(family, px).unwrap_or_else(|e| {
        tracing::warn!("police « {family} » : {e} — police embarquée");
        FontSet::embedded(px)
    })
}

fn load_icon() -> Option<Icon> {
    let image = image::load_from_memory(ICON_PNG)
        .map_err(|e| tracing::warn!("icône : {e}"))
        .ok()?
        .to_rgba8();
    let (w, h) = image.dimensions();
    Icon::from_rgba(image.into_raw(), w, h)
        .map_err(|e| tracing::warn!("icône : {e}"))
        .ok()
}

/// Annonce `rustty` comme identifiant d'application, pour que le bureau
/// associe la fenêtre au lanceur et à son icône.
#[cfg(target_os = "linux")]
fn with_app_id(attrs: WindowAttributes) -> WindowAttributes {
    use winit::platform::wayland::WindowAttributesExtWayland;
    use winit::platform::x11::WindowAttributesExtX11;
    let attrs = WindowAttributesExtWayland::with_name(attrs, APP_ID, APP_ID);
    WindowAttributesExtX11::with_name(attrs, APP_ID, APP_ID)
}

#[cfg(not(target_os = "linux"))]
fn with_app_id(attrs: WindowAttributes) -> WindowAttributes {
    let _ = APP_ID;
    attrs
}
