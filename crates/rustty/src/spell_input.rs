//! Réactions de la fenêtre pour l'infobulle de correction : survol d'un mot
//! souligné, réponse de LanguageTool, clic sur une suggestion, fermeture. La
//! logique (mot, requête, placement, dessin) est dans le module `spell`.

use std::sync::Arc;
use std::time::Instant;

use rustty_render::{Chrome, PixelRect, grid_geometry};

use crate::model::Effect;
use crate::mouse::MouseButton;
use crate::spell::{
    LookupKey, PopupLayout, SpellPopup, Suggestion, UnderlinedRun, apply_token, occurrence_index,
    popup_chrome, popup_layout, spawn_lookup, underlined_run,
};
use crate::tab::TermId;
use crate::window_state::OsWindow;

/// Le jeton, entre les marques de collage si le programme les demande.
fn token_bytes(token: &str, bracketed_paste: bool) -> Vec<u8> {
    if bracketed_paste {
        format!("\x1b[200~{token}\x1b[201~").into_bytes()
    } else {
        token.as_bytes().to_vec()
    }
}

impl OsWindow {
    /// Mouvement de souris dans l'infobulle : met à jour le remplacement
    /// survolé. `None` si la souris n'y est pas ; sinon vrai s'il faut redessiner.
    pub(crate) fn spell_popup_hover(&mut self, x: f64, y: f64) -> Option<bool> {
        let layout = self.spell_layout()?;
        if !layout.contains(x, y) {
            return None;
        }
        let popup = self.spell.popup.as_mut()?;
        let hovered = layout.hit(x, y);
        let changed = popup.hovered != hovered;
        popup.hovered = hovered;
        Some(changed)
    }

    /// Mouvement de souris sur la cellule `cell` du panneau `term` : entretient
    /// le survol du mot souligné (requête, infobulle). Vrai s'il faut redessiner.
    pub(crate) fn spell_hover(&mut self, term: TermId, cell: Option<(usize, usize)>) -> bool {
        if !self.config.spellcheck.enabled {
            return false;
        }
        let Some((col, row)) = cell else {
            return self.leave_spell_word();
        };
        if self
            .spell
            .hover
            .as_ref()
            .is_some_and(|k| k.term == term && k.row == row && (k.start..k.end).contains(&col))
        {
            // Toujours sur le même mot : requête partie ou infobulle ouverte.
            return false;
        }
        let Some(tw) = self.terms.get(&term) else {
            return self.leave_spell_word();
        };
        let snapshot = tw.snapshot();
        let Some(run) = underlined_run(&snapshot.lines, row, col) else {
            return self.leave_spell_word();
        };
        let key = LookupKey {
            term,
            row: run.row,
            start: run.start,
            end: run.end,
            context: run.context.clone(),
        };
        let redraw = self.close_spell_popup();
        self.spell.hover = Some(key.clone());
        if self.spell.may_query(Instant::now()) {
            spawn_lookup(
                self.config.spellcheck.url.clone(),
                key,
                run.offset,
                run.length,
                Arc::clone(&self.waker),
            );
        }
        redraw
    }

    /// La souris a quitté le mot (ou le panneau) : plus de survol, infobulle fermée.
    pub(crate) fn leave_spell_word(&mut self) -> bool {
        self.spell.hover = None;
        self.close_spell_popup()
    }

    /// Ferme l'infobulle. Vrai si elle était ouverte.
    pub(crate) fn close_spell_popup(&mut self) -> bool {
        self.spell.popup.take().is_some()
    }

    /// Réponse de LanguageTool : ouvre l'infobulle si la souris est toujours
    /// sur le mot demandé.
    pub(crate) fn spell_result(
        &mut self,
        key: LookupKey,
        result: Result<Option<Suggestion>, String>,
    ) -> Vec<Effect> {
        let suggestion = match result {
            Ok(suggestion) => {
                self.spell.mark_reachable();
                suggestion
            }
            Err(e) => {
                tracing::debug!("LanguageTool injoignable : {e}");
                self.spell.mark_unreachable(Instant::now());
                return Vec::new();
            }
        };
        if self.spell.hover.as_ref() != Some(&key) {
            return Vec::new();
        }
        let Some(suggestion) = suggestion else {
            return Vec::new();
        };
        let Some(tw) = self.terms.get(&key.term) else {
            return Vec::new();
        };
        let snapshot = tw.snapshot();
        // Le panneau a pu se redessiner pendant la requête : le mot doit être encore là.
        let Some(run) = underlined_run(&snapshot.lines, key.row, key.start) else {
            return Vec::new();
        };
        if run.start != key.start || run.end != key.end {
            return Vec::new();
        }
        let Some(anchor) = self.word_anchor(key.term, &run) else {
            return Vec::new();
        };
        self.spell.popup = Some(SpellPopup {
            term: key.term,
            run,
            message: suggestion.message,
            replacements: suggestion.replacements,
            hovered: None,
            anchor,
        });
        vec![Effect::Redraw]
    }

    /// Rectangle du mot à l'écran, en pixels.
    fn word_anchor(&self, term: TermId, run: &UnderlinedRun) -> Option<PixelRect> {
        let tab = self.model.workspace.active_tab();
        let rect = self
            .pane_rects
            .iter()
            .find(|(wid, _)| tab.term_at(*wid) == Some(term))
            .map(|(_, r)| *r)?;
        let metrics = self.metrics_of(term);
        let g = grid_geometry(rect, metrics, self.config.window.padding);
        Some(PixelRect::new(
            g.origin_x + run.start as u32 * metrics.width,
            g.origin_y + run.row as u32 * metrics.height,
            (run.end - run.start) as u32 * metrics.width,
            metrics.height,
        ))
    }

    /// Le panneau `term` s'est redessiné : l'infobulle reste si son mot est
    /// toujours souligné au même endroit, se ferme sinon.
    pub(crate) fn spell_term_updated(&mut self, term: TermId) -> bool {
        let Some(popup) = self.spell.popup.as_ref() else {
            return false;
        };
        if popup.term != term {
            return false;
        }
        let still_there = self.terms.get(&term).is_some_and(|tw| {
            let snapshot = tw.snapshot();
            underlined_run(&snapshot.lines, popup.run.row, popup.run.start)
                .is_some_and(|run| run == popup.run)
        });
        if still_there {
            return false;
        }
        self.leave_spell_word()
    }

    pub(crate) fn spell_layout(&self) -> Option<PopupLayout> {
        let popup = self.spell.popup.as_ref()?;
        Some(popup_layout(popup, self.surface.size(), self.metrics()))
    }

    /// Appui de souris infobulle ouverte : envoie la suggestion cliquée au
    /// programme, ignore un clic ailleurs dans l'infobulle, la ferme sinon et
    /// laisse le clic suivre son cours. `None` si aucune infobulle n'est ouverte.
    pub(crate) fn spell_click(
        &mut self,
        button: MouseButton,
        x: f64,
        y: f64,
    ) -> Option<Vec<Effect>> {
        let layout = self.spell_layout()?;
        if !layout.contains(x, y) {
            self.close_spell_popup();
            self.window.request_redraw();
            return None;
        }
        let hit = (button == MouseButton::Left)
            .then(|| layout.hit(x, y))
            .flatten();
        let Some(index) = hit else {
            return Some(Vec::new());
        };
        let popup = self.spell.popup.take()?;
        self.spell.hover = None;
        let Some(tw) = self.terms.get(&popup.term) else {
            return Some(vec![Effect::Redraw]);
        };
        let occurrence = occurrence_index(&tw.snapshot().lines, &popup.run);
        let token = apply_token(&popup.run.word, &popup.replacements[index], occurrence);
        tw.write(token_bytes(&token, tw.modes().bracketed_paste));
        Some(vec![Effect::Redraw])
    }

    pub(crate) fn spell_chrome(&self) -> Option<Chrome> {
        let popup = self.spell.popup.as_ref()?;
        let layout = popup_layout(popup, self.surface.size(), self.metrics());
        Some(popup_chrome(popup, &layout, &self.palette, self.metrics()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_wrapped_in_paste_marks_when_the_program_asks() {
        assert_eq!(token_bytes("t", false), b"t");
        assert_eq!(token_bytes("t", true), b"\x1b[200~t\x1b[201~");
    }
}
