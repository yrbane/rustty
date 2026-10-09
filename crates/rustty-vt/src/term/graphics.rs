//! Exécution des commandes graphiques kitty reçues par APC : transmission,
//! affichage à la position du curseur, suppression et réponses.

use std::sync::Arc;

use super::Term;
use crate::graphics::{
    Action, ChunkResult, GraphicsCommand, GraphicsError, ImageData, ImageStrip, Placement, decode,
    extent, parse,
};

impl Term {
    /// Charge d'une séquence APC complète ; seules les commandes `G…` comptent.
    pub(crate) fn apc(&mut self, payload: &[u8]) {
        let Some((cmd, data)) = parse(payload) else {
            return;
        };
        match self.chunks.push(cmd, data) {
            ChunkResult::Pending => {}
            ChunkResult::Error(cmd, err) => self.reply(&cmd, Err(err)),
            ChunkResult::Complete(cmd, bytes) => {
                let result = self.run_graphics(&cmd, &bytes);
                self.reply(&cmd, result);
            }
        }
    }

    /// Renseigne la taille d'une cellule en pixels ; les zéros sont ignorés.
    pub fn set_cell_pixels(&mut self, w: u32, h: u32) {
        if w > 0 && h > 0 {
            self.cell_pixels = (w, h);
        }
    }

    fn run_graphics(&mut self, cmd: &GraphicsCommand, bytes: &[u8]) -> Result<(), GraphicsError> {
        match cmd.action {
            Action::Transmit | Action::TransmitAndPut => {
                let image = Arc::new(decode(cmd.format, cmd.width, cmd.height, bytes)?);
                if let Some(id) = cmd.id {
                    self.images.insert(id, Arc::clone(&image));
                }
                if cmd.action == Action::TransmitAndPut {
                    self.place_image(image, cmd.cols, cmd.rows);
                }
                Ok(())
            }
            Action::Put => {
                let image = cmd
                    .id
                    .and_then(|id| self.images.get(id))
                    .ok_or(GraphicsError::NotFound)?;
                self.place_image(image, cmd.cols, cmd.rows);
                Ok(())
            }
            Action::Delete => self.delete_images(cmd),
        }
    }

    /// Pose l'image à partir du curseur : une bande par rangée, en descendant.
    fn place_image(&mut self, image: Arc<ImageData>, cols: Option<u16>, rows: Option<u16>) {
        let (c, r, width_cells, height_cells) = extent(
            (image.width, image.height),
            self.cell_pixels,
            self.cols(),
            cols,
            rows,
        );
        let placement = Arc::new(Placement {
            image,
            cols: c,
            rows: r,
            width_cells,
            height_cells,
        });
        let start_col = self.cursor.col;
        for row in 0..r {
            if row > 0 {
                self.linefeed();
            }
            let strip = ImageStrip {
                placement: Arc::clone(&placement),
                col: start_col as u16,
                row,
            };
            let cursor_row = self.cursor.row;
            self.active_grid_mut()
                .line_mut(cursor_row)
                .push_image(strip);
        }
        let end = start_col + usize::from(c);
        self.cursor.col = end.min(self.cols() - 1);
        self.cursor.pending_wrap = end >= self.cols();
    }

    fn delete_images(&mut self, cmd: &GraphicsCommand) -> Result<(), GraphicsError> {
        match cmd.delete {
            'a' | 'A' => self.drop_strips(|_| true),
            'i' | 'I' => {
                let id = cmd
                    .id
                    .ok_or_else(|| GraphicsError::Invalid("i= requis pour d=i".into()))?;
                if let Some(image) = self.images.get(id) {
                    let target = image.id;
                    self.drop_strips(|s| s.placement.image.id == target);
                }
                self.images.remove(id);
            }
            other => {
                return Err(GraphicsError::Invalid(format!(
                    "cible de suppression inconnue : {other}"
                )));
            }
        }
        Ok(())
    }

    fn drop_strips(&mut self, mut drop: impl FnMut(&ImageStrip) -> bool) {
        for row in 0..self.rows() {
            self.active_grid_mut()
                .line_mut(row)
                .drop_strips_where(&mut drop);
        }
    }

    /// Répond à l'application si `i=` est donné et que `q=` ne l'interdit pas.
    fn reply(&mut self, cmd: &GraphicsCommand, result: Result<(), GraphicsError>) {
        let Some(id) = cmd.id else {
            return;
        };
        let text = match result {
            Ok(()) if cmd.quiet == 0 && cmd.action != Action::Delete => format!("i={id};OK"),
            Ok(()) => return,
            Err(_) if cmd.quiet >= 2 => return,
            Err(e) => format!("i={id};{}:{}", e.code(), message(&e)),
        };
        self.outbox
            .respond(format!("\x1b_G{text}\x1b\\").as_bytes());
    }
}

fn message(err: &GraphicsError) -> String {
    match err {
        GraphicsError::Invalid(m) => m.clone(),
        GraphicsError::TooBig => "image trop grande".into(),
        GraphicsError::NoData => "aucune donnée reçue".into(),
        GraphicsError::NotFound => "image introuvable".into(),
    }
}
