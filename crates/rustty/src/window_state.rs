//! La fenêtre système et tout ce qu'elle porte : surface, renderer,
//! terminaux, modèle. Les réactions aux entrées sont dans `input.rs`, les
//! effets dans `effects.rs` ; ici, création, mise en page et rendu.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use rustty_config::{Config, Mods};
use rustty_layout::{Rect, SplitId, WindowId};
use rustty_pty::Shell;
use rustty_render::{
    CellMetrics, GpuContext, HoverTarget, Palette, PixelRect, TabBarLayout, TabBarStyle,
};
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, Icon, Window, WindowAttributes};

use crate::banner::Banner;
use crate::context_menu::ContextMenu;
use crate::events::Waker;
use crate::gpu_surface::{self, Surface};
use crate::model::Model;
use crate::mouse::{DoubleClick, MotionFilter, MouseButton, Selection, WheelAccumulator};
use crate::pane_fonts::{PaneFonts, SizeKey};
use crate::renderers::{RendererSpec, Renderers};
use crate::tab::TermId;
use crate::term_window::TermWindow;
use crate::title;

/// Icône de fenêtre (X11, Windows) ; Wayland passe par le `.desktop`.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/rustty-256.png");
/// Identifiant d'application : `app_id` Wayland et `WM_CLASS` X11.
const APP_ID: &str = "rustty";
const DEFAULT_SIZE: LogicalSize<f64> = LogicalSize::new(960.0, 600.0);
/// Alpha de la surbrillance de sélection posée par-dessus le texte.
pub(crate) const SELECTION_ALPHA: f32 = 0.4;

pub struct OsWindow {
    pub window: Arc<Window>,
    pub ctx: GpuContext,
    pub surface: Surface,
    pub renderers: Renderers,
    /// Taille de police de chaque panneau (zoom par panneau).
    pub pane_fonts: PaneFonts,
    pub palette: Palette,
    pub tab_style: TabBarStyle,
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub model: Model,
    pub terms: HashMap<TermId, TermWindow>,
    pub titles: HashMap<TermId, String>,
    pub modifiers: Mods,
    pub cursor: (f64, f64),
    /// Faux tant que la souris n'est pas entrée (ou après sa sortie).
    pub cursor_inside: bool,
    pub selection: Option<(TermId, Selection)>,
    pub dragging: bool,
    pub held: Option<(TermId, MouseButton)>,
    /// Dernière cellule rapportée en mouvement (une fois par cellule).
    pub motion: MotionFilter,
    pub tab_clicks: DoubleClick,
    pub wheel: WheelAccumulator,
    pub press_target: HoverTarget,
    pub tab_bar: Option<TabBarLayout>,
    pub pane_rects: Vec<(WindowId, PixelRect)>,
    pub dividers: Vec<(SplitId, PixelRect)>,
    /// Zone des panneaux (sous ou sur la barre d'onglets).
    pub content: Rect,
    /// Barre de split en cours de glisser.
    pub drag: Option<SplitId>,
    pub cursor_icon: CursorIcon,
    /// Menu du clic droit ouvert.
    pub menu: Option<ContextMenu>,
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
        let renderers = Renderers::new(
            &RendererSpec {
                ctx: &ctx,
                format: surface.view_format(),
                family: &config.font.family,
                scale_factor: window.scale_factor(),
                palette: &palette,
                padding: config.window.padding,
            },
            config.font.size,
        );
        let pane_fonts = PaneFonts::new(config.font.size);
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
            renderers,
            pane_fonts,
            palette,
            tab_style,
            config,
            config_path,
            model,
            terms: HashMap::new(),
            titles: HashMap::new(),
            modifiers: Mods::empty(),
            cursor: (0.0, 0.0),
            cursor_inside: false,
            selection: None,
            dragging: false,
            held: None,
            motion: MotionFilter::default(),
            tab_clicks: DoubleClick::default(),
            wheel: WheelAccumulator::default(),
            press_target: HoverTarget::None,
            tab_bar: None,
            pane_rects: Vec::new(),
            dividers: Vec::new(),
            content: Rect::new(0, 0, 0, 0),
            drag: None,
            cursor_icon: CursorIcon::Default,
            menu: None,
            focused: true,
            clipboard,
            shell: Shell::default_for_platform(),
            waker,
        };
        this.spawn_term(first)?;
        this.relayout();
        Ok(this)
    }

    /// Métriques de la taille configurée : barre d'onglets, bandeaux.
    pub fn metrics(&self) -> CellMetrics {
        self.renderers.metrics(self.renderers.base_key())
    }

    /// Métriques du panneau de `term`, zoom compris.
    pub fn metrics_of(&self, term: TermId) -> CellMetrics {
        self.renderers
            .metrics(SizeKey::of(self.pane_fonts.size_of(term)))
    }

    fn renderer_spec(&self) -> RendererSpec<'_> {
        RendererSpec {
            ctx: &self.ctx,
            format: self.surface.view_format(),
            family: &self.config.font.family,
            scale_factor: self.window.scale_factor(),
            palette: &self.palette,
            padding: self.config.window.padding,
        }
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

    /// Applique une configuration rechargée : palette, police, onglets, opacité, raccourcis.
    pub fn apply_config(&mut self, new: Config) {
        if new.font.size != self.config.font.size {
            // La taille configurée change : les zooms repartent de la nouvelle base.
            self.pane_fonts.rebase(new.font.size);
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
            self.renderers.set_palette(&self.palette);
        }
        self.relayout();
        self.window.request_redraw();
    }

    /// Renderers refaits avec la police courante (changement de police, de
    /// marge ou d'échelle d'écran) ; les tailles zoomées reviennent au relayout.
    pub fn rebuild_fonts(&mut self) {
        let base = self.pane_fonts.base();
        let spec = self.renderer_spec();
        let fresh = Renderers::new(&spec, base);
        self.renderers = fresh;
    }
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
