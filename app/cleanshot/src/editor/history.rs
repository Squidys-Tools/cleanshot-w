//! Undo/redo with tldraw's `markHistoryStoppingPoint` semantics: an
//! interaction calls [`History::begin`] when it starts and
//! [`History::commit`] when it ends; all the mutations in between collapse
//! into a single undo step. An interaction that changed nothing commits
//! nothing.
//!
//! History stores full snapshots (document +, when the interaction touched
//! the image itself, the base RGBA image — only crop does today). The editor
//! document is small (a few hundred shapes), so snapshot simplicity costs
//! nothing and buys command-pattern-level correctness, including undoing a
//! crop's pixel changes.

use super::model::Document;
use image::RgbaImage;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub doc: Document,
    pub image: Option<RgbaImage>,
}

#[derive(Debug, Default)]
pub struct History {
    /// Snapshots of past states, most recent last. The top of this stack is
    /// what undo restores.
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    /// The snapshot taken by the open interaction, waiting for `commit`.
    pending: Option<Snapshot>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the start of an interaction (a drag, a stroke, a delete…).
    /// Snapshots the current state; pairs with a later `commit`. Pass
    /// `image: Some` when the interaction mutates the base image (crop).
    pub fn begin(&mut self, current: &Document, image: Option<&RgbaImage>) {
        self.pending = Some(Snapshot {
            doc: current.clone(),
            image: image.cloned(),
        });
    }

    /// Mark the end of an interaction. If the state actually changed since
    /// `begin`, the pre-interaction snapshot becomes one undo step.
    pub fn commit(&mut self, current: &Document, image: Option<&RgbaImage>) {
        if let Some(before) = self.pending.take() {
            let image_changed = before
                .image
                .as_ref()
                .is_some_and(|before| image.is_none_or(|now| before.as_raw() != now.as_raw()));
            let changed = before.doc != *current || image_changed;
            if changed {
                self.undo_stack.push(before);
                self.redo_stack.clear();
            }
        }
    }

    /// Discard the open interaction's pending snapshot without committing
    /// (used when an interaction is cancelled).
    pub fn cancel(&mut self) -> Option<Snapshot> {
        self.pending.take()
    }

    /// Step back one undo step. Returns the restored document and, when that
    /// step carried an image change, the base image to restore. An
    /// uncommitted interaction commits first.
    pub fn undo(
        &mut self,
        current: &Document,
        current_image: Option<&RgbaImage>,
    ) -> Option<Snapshot> {
        self.commit(current, current_image);
        let previous = self.undo_stack.pop()?;
        let image = previous.image.as_ref().and_then(|_| current_image.cloned());
        self.redo_stack.push(Snapshot {
            doc: current.clone(),
            image,
        });
        Some(previous)
    }

    /// Step forward one redo step.
    pub fn redo(
        &mut self,
        current: &Document,
        current_image: Option<&RgbaImage>,
    ) -> Option<Snapshot> {
        let next = self.redo_stack.pop()?;
        let image = next.image.as_ref().and_then(|_| current_image.cloned());
        self.undo_stack.push(Snapshot {
            doc: current.clone(),
            image,
        });
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::model::{Rect, RectShape, Shape};

    fn document_with_rects(count: usize) -> Document {
        let mut document = Document::new(100, 100);
        for index in 0..count {
            document.shapes.push(Shape::Rectangle(RectShape {
                id: format!("r{index}"),
                x: index as f32 * 10.0,
                y: 0.0,
                w: 5.0,
                h: 5.0,
                color: "#e03131".to_string(),
                stroke_width: 2.0,
                ..Default::default()
            }));
        }
        document
    }

    fn add_rect(document: &mut Document, id: &str) {
        document.shapes.push(Shape::Rectangle(RectShape {
            id: id.to_string(),
            x: 0.0,
            y: 0.0,
            w: 5.0,
            h: 5.0,
            color: "#e03131".to_string(),
            stroke_width: 2.0,
            ..Default::default()
        }));
    }

    fn test_image(byte: u8) -> RgbaImage {
        image::RgbaImage::from_pixel(10, 10, image::Rgba([byte, byte, byte, 255]))
    }

    #[test]
    fn an_interaction_collapses_into_one_undo_step() {
        let mut document = document_with_rects(0);
        let mut history = History::new();

        history.begin(&document, None);
        add_rect(&mut document, "a");
        add_rect(&mut document, "b");
        document.delete(&"a".to_string());
        history.commit(&document, None);

        assert!(history.can_undo());
        assert!(!history.can_redo());
        let restored = history.undo(&document, None).unwrap();
        assert!(restored.doc.shapes.is_empty());
        assert!(restored.image.is_none());
    }

    #[test]
    fn an_interaction_without_changes_commits_nothing() {
        let document = document_with_rects(1);
        let mut history = History::new();

        history.begin(&document, None);
        history.commit(&document, None); // e.g. a mouse press with no movement
        assert!(!history.can_undo());
    }

    #[test]
    fn undo_redo_round_trips_through_every_state() {
        let mut document = document_with_rects(0);
        let mut history = History::new();

        for name in ["a", "b", "c"] {
            history.begin(&document, None);
            add_rect(&mut document, name);
            history.commit(&document, None);
        }
        assert_eq!(document.shapes.len(), 3);

        document = history.undo(&document, None).unwrap().doc;
        assert_eq!(document.shapes.len(), 2);
        document = history.undo(&document, None).unwrap().doc;
        assert_eq!(document.shapes.len(), 1);
        assert!(history.can_undo());
        assert!(history.can_redo());

        document = history.undo(&document, None).unwrap().doc;
        assert_eq!(document.shapes.len(), 0);
        assert!(!history.can_undo());

        document = history.redo(&document, None).unwrap().doc;
        assert_eq!(document.shapes.len(), 1);
        document = history.redo(&document, None).unwrap().doc;
        assert_eq!(document.shapes.len(), 2);
        assert!(history.can_redo());
    }

    #[test]
    fn a_new_edit_clears_the_redo_stack() {
        let mut document = document_with_rects(0);
        let mut history = History::new();
        history.begin(&document, None);
        add_rect(&mut document, "a");
        history.commit(&document, None);
        document = history.undo(&document, None).unwrap().doc;
        assert!(history.can_redo());

        history.begin(&document, None);
        add_rect(&mut document, "b");
        history.commit(&document, None);
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_commits_the_open_interaction_first() {
        let mut document = document_with_rects(1);
        let mut history = History::new();
        history.begin(&document, None);
        add_rect(&mut document, "live");
        // User hits ctrl+z mid-drag: the open interaction lands first.
        let restored = history.undo(&document, None).unwrap();
        assert_eq!(restored.doc.shapes.len(), 1);
        assert!(!history.can_undo());
    }

    #[test]
    fn undo_during_an_uncommitted_shape_edit_ignores_the_untracked_image() {
        let document = document_with_rects(0);
        let image = test_image(1);
        let mut history = History::new();
        history.begin(&document, None);
        let mut live = document.clone();
        add_rect(&mut live, "live");

        let restored = history.undo(&live, Some(&image)).unwrap();
        assert_eq!(restored.doc, document);
        assert!(restored.image.is_none());
        assert!(!history.can_undo());

        let redone = history.redo(&restored.doc, Some(&image)).unwrap();
        assert_eq!(redone.doc, live);
        assert!(redone.image.is_none());
    }

    #[test]
    fn undo_without_edits_does_not_create_a_phantom_image_step() {
        let document = document_with_rects(0);
        let image = test_image(1);
        let mut history = History::new();
        history.begin(&document, None);

        assert!(history.undo(&document, Some(&image)).is_none());
        assert!(!history.can_undo());
    }

    #[test]
    fn cancel_drops_the_pending_interaction() {
        let mut document = document_with_rects(1);
        let mut history = History::new();
        history.begin(&document, None);
        add_rect(&mut document, "live");
        let cancelled = history.cancel().unwrap();
        assert_eq!(cancelled.doc.shapes.len(), 1);
        assert!(!history.can_undo());
    }

    #[test]
    fn undo_on_fresh_history_returns_none() {
        let mut history = History::new();
        let document = document_with_rects(0);
        assert!(history.undo(&document, None).is_none());
        assert!(history.redo(&document, None).is_none());
    }

    #[test]
    fn image_changes_make_a_snapshot_different() {
        let document = document_with_rects(1);
        let mut history = History::new();
        let before = test_image(1);

        history.begin(&document, Some(&before));
        let after = test_image(2);
        history.commit(&document, Some(&after));
        assert!(history.can_undo(), "a pixel change must count as a change");

        let restored = history.undo(&document, Some(&after)).unwrap();
        assert_eq!(restored.image.as_ref().unwrap().as_raw(), before.as_raw());

        let redone = history.redo(&restored.doc, Some(&before)).unwrap();
        assert_eq!(redone.image.as_ref().unwrap().as_raw(), after.as_raw());
    }

    #[test]
    fn snapshots_are_independent_of_live_mutation() {
        let document = document_with_rects(1);
        let mut history = History::new();
        history.begin(&document, None);
        let mut live = document.clone();
        live.crop(&Rect::new(0.0, 0.0, 5.0, 5.0));
        history.commit(&live, None);
        let restored = history.undo(&live, None).unwrap();
        assert_eq!(restored.doc.shapes.len(), 1);
    }
}
