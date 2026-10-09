//! La boucle winit : dispatch pur des événements vers `OsWindow`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustty_config::Config;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::banner::Banner;
use crate::config_watch::ConfigWatcher;
use crate::events::UserEvent;
use crate::model::{CloseRequest, Effect};
use crate::window_state::OsWindow;

/// Période de sondage de la fin des shells (Windows n'a pas de fin de flux).
const EXIT_POLL: Duration = Duration::from_millis(500);

struct App {
    window: Option<OsWindow>,
    config: Option<Config>,
    config_error: Option<String>,
    config_path: Option<PathBuf>,
    hold: bool,
    proxy: EventLoopProxy<UserEvent>,
    watcher: Option<ConfigWatcher>,
    failure: Option<anyhow::Error>,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let config = self.config.take().unwrap_or_default();
        let waker = Arc::new(self.proxy.clone());
        match OsWindow::open(
            event_loop,
            config,
            self.hold,
            self.config_error.take(),
            self.config_path.clone(),
            Arc::clone(&waker) as _,
        ) {
            Ok(window) => {
                let path = self.config_path.clone().or_else(Config::default_path);
                self.watcher = path.and_then(|p| ConfigWatcher::start(&p, waker));
                self.window = Some(window);
            }
            Err(e) => {
                self.failure = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(w) = self.window.as_mut() else {
            return;
        };
        let effects = match event {
            WindowEvent::CloseRequested => {
                let confirm = w.config.window.confirm_close_with_running_children;
                let terms = &w.terms;
                let running = |t| {
                    terms
                        .get(&t)
                        .is_some_and(crate::term_window::TermWindow::has_running_children)
                };
                w.model.request_close_window(confirm, &running)
            }
            WindowEvent::Resized(size) => {
                w.menu = None;
                w.surface.resize(&w.ctx, size.width, size.height);
                vec![Effect::Relayout]
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                w.rebuild_fonts();
                vec![Effect::Relayout]
            }
            WindowEvent::RedrawRequested => {
                w.render();
                Vec::new()
            }
            WindowEvent::KeyboardInput { event, .. } => w.on_key(&event),
            WindowEvent::ModifiersChanged(m) => {
                w.on_modifiers(m.state());
                Vec::new()
            }
            WindowEvent::CursorMoved { position, .. } => w.on_cursor_moved(position.x, position.y),
            WindowEvent::CursorLeft { .. } => w.on_cursor_left(),
            WindowEvent::MouseInput { state, button, .. } => w.on_mouse_input(state, button),
            WindowEvent::MouseWheel { delta, .. } => w.on_wheel(delta),
            WindowEvent::Focused(focused) => w.on_focus(focused),
            _ => Vec::new(),
        };
        w.run_effects(effects, event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let Some(w) = self.window.as_mut() else {
            return;
        };
        let effects = match event {
            UserEvent::TermUpdated(_) | UserEvent::TermModes(_) => vec![Effect::Redraw],
            UserEvent::TermTitle(id, title) => {
                w.titles.insert(id, title);
                vec![Effect::Relayout]
            }
            UserEvent::TermBell(id) => {
                tracing::debug!("cloche dans {id:?}");
                Vec::new()
            }
            UserEvent::SetClipboard(_) if !clipboard_write_allowed(&w.config) => {
                tracing::debug!("OSC 52 ignoré : window.osc52_clipboard = false");
                Vec::new()
            }
            UserEvent::SetClipboard(text) => {
                if let Some(cb) = w.clipboard.as_mut()
                    && let Err(e) = cb.set_text(text)
                {
                    tracing::warn!("OSC 52 : {e}");
                }
                Vec::new()
            }
            UserEvent::PtyEof(id) => w.check_exit(id),
            UserEvent::TermFailed(id, message) => {
                tracing::error!("thread lecteur du panneau {id:?} : {message}");
                w.model.set_notice(Some(Banner::error(format!(
                    "panneau fermé après une erreur interne : {message}"
                ))));
                w.model.request_close(CloseRequest::Term(id), false)
            }
            UserEvent::ConfigChanged => {
                w.reload_config();
                Vec::new()
            }
        };
        w.run_effects(effects, event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(w) = self.window.as_mut() else {
            return;
        };
        let alive: Vec<_> = w
            .terms
            .iter()
            .filter(|(_, t)| t.exit.is_none())
            .map(|(id, _)| *id)
            .collect();
        let mut effects = Vec::new();
        for id in &alive {
            effects.extend(w.check_exit(*id));
        }
        w.run_effects(effects, event_loop);
        if alive.is_empty() {
            event_loop.set_control_flow(ControlFlow::Wait);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + EXIT_POLL));
        }
    }
}

pub fn run(
    config: Config,
    config_error: Option<String>,
    config_path: Option<PathBuf>,
    hold: bool,
) -> anyhow::Result<()> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = App {
        window: None,
        config: Some(config),
        config_error,
        config_path,
        hold,
        proxy,
        watcher: None,
        failure: None,
    };
    event_loop.run_app(&mut app)?;
    match app.failure.take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Le terminal a-t-il le droit d'écrire dans le presse-papiers (OSC 52) ?
pub fn clipboard_write_allowed(config: &Config) -> bool {
    config.window.osc52_clipboard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_writes_follow_the_config() {
        assert!(clipboard_write_allowed(&Config::default()));
        let denied = Config::from_str("[window]\nosc52_clipboard = false\n").unwrap();
        assert!(!clipboard_write_allowed(&denied));
    }
}
