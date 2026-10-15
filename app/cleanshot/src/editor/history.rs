//! Undo/redo with tldraw's `markHistoryStoppingPoint` semantics: an
//! interaction calls [`History::begin`] when it starts and
//! [`History::commit`] when it ends; all the mutations in between collapse
//! into a single undo step. An interaction that changed nothing commits
//! nothing.
//!
//! History stores full document snapshots — the document is small (a few
//! hundred shapes), which buys command-pattern-level simplicity for free.

use super::model::Document;

#[derive(Debug, Default)]
pub struct History {
    /// Snapshots of past states, most recent last. The document at the top of
    /// this stack is what undo restores.
    undo_stack: Vec<Document>,
    redo_stack: Vec<Document>,
    /// The snapshot taken by the open interaction, waiting for `commit`.
    pending: Option<Document>,
}

impl History {
    pub fn new(initial: Document) -> Self {
        History {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending: Some(initial),
        }
    }

    /// Mark the start of an interaction (a drag, a stroke, a delete…).
    /// Snapshots the current document; pairs with a later `commit`.
    pub fn begin(&mut self, current: &Document) {
        self.pending = Some(current.clone());
    }

    /// Mark the end of an interaction. If the document actually changed since
    /// `begin`, the pre-interaction state becomes one undo step.
    pub fn commit(&mut self, current: &Document) {
        if let Some(before) = self.pending.take() {
            if &before != current {
                self.undo_stack.push(before);
                self.redo_stack.clear();
            }
        }
    }

    /// Discard the open interaction's pending snapshot without committing
    /// (used when an interaction is cancelled).
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// Step back one undo step. Returns the restored document, or None when
    /// there is nothing to undo. An uncommitted interaction commits first.
    pub fn undo(&mut self, current: &Document) -> Option<Document> {
        self.commit(current);
        let previous = self.undo_stack.pop()?;
        self.redo_stack.push(current.clone());
        Some(previous)
    }

    /// Step forward one redo step. Returns the restored document, or None
    /// when there is nothing to redo.
    pub fn redo(&mut self, current: &Document) -> Option<Document> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current.clone());
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Drop all history (e.g. after a crop that replaces the image).
    pub fn clear(&mut self, initial: &Document) {
        *self = History::new(initial.clone());
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
        }));
    }

    #[test]
    fn an_interaction_collapses_into_one_undo_step() {
        let mut document = document_with_rects(0);
        let mut history = History::new(document.clone());

        history.begin(&document);
        add_rect(&mut document, "a");
        add_rect(&mut document, "b");
        document.delete(&"a".to_string());
        history.commit(&document);

        assert!(history.can_undo());
        assert!(!history.can_redo());
        let restored = history.undo(&document).unwrap();
        assert!(restored.shapes.is_empty());
    }

    #[test]
    fn an_interaction_without_changes_commits_nothing() {
        let mut document = document_with_rects(1);
        let mut history = History::new(document.clone());

        history.begin(&document);
        history.commit(&document); // e.g. a mouse press with no movement
        assert!(!history.can_undo());
    }

    #[test]
    fn undo_redo_round_trips_through_every_state() {
        let mut document = document_with_rects(0);
        let mut history = History::new(document.clone());

        for name in ["a", "b", "c"] {
            history.begin(&document);
            add_rect(&mut document, name);
            history.commit(&document);
        }
        assert_eq!(document.shapes.len(), 3);

        document = history.undo(&document).unwrap();
        assert_eq!(document.shapes.len(), 2);
        document = history.undo(&document).unwrap();
        assert_eq!(document.shapes.len(), 1);
        assert!(history.can_undo());
        assert!(history.can_redo());

        document = history.undo(&document).unwrap();
        assert_eq!(document.shapes.len(), 0);
        assert!(!history.can_undo());

        document = history.redo(&document).unwrap();
        assert_eq!(document.shapes.len(), 1);
        document = history.redo(&document).unwrap();
        assert_eq!(document.shapes.len(), 2);
        assert!(history.can_redo());
    }

    #[test]
    fn a_new_edit_clears_the_redo_stack() {
        let mut document = document_with_rects(0);
        let mut history = History::new(document.clone());
        history.begin(&document);
        add_rect(&mut document, "a");
        history.commit(&document);
        document = history.undo(&document).unwrap();
        assert!(history.can_redo());

        history.begin(&document);
        add_rect(&mut document, "b");
        history.commit(&document);
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_commits_the_open_interaction_first() {
        let mut document = document_with_rects(1);
        let mut history = History::new(document.clone());
        history.begin(&document);
        add_rect(&mut document, "live");
        // User hits ctrl+z mid-drag: the open interaction lands first.
        let restored = history.undo(&document).unwrap();
        assert_eq!(restored.shapes.len(), 1);
        assert!(!history.can_undo());
    }

    #[test]
    fn cancel_drops_the_pending_interaction() {
        let mut document = document_with_rects(1);
        let mut history = History::new(document.clone());
        history.begin(&document);
        add_rect(&mut document, "live");
        history.cancel();
        assert!(!history.can_undo());
    }

    #[test]
    fn undo_on_fresh_history_returns_none() {
        let mut history = History::new(document_with_rects(0));
        let document = document_with_rects(0);
        assert!(history.undo(&document).is_none());
        assert!(history.redo(&document).is_none());
    }

    #[test]
    fn clear_resets_to_a_single_state() {
        let mut document = document_with_rects(1);
        let mut history = History::new(document.clone());
        history.begin(&document);
        add_rect(&mut document, "a");
        history.commit(&document);
        history.clear(&document);
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn snapshots_are_independent_of_live_mutation() {
        let document = document_with_rects(1);
        let mut history = History::new(document.clone());
        history.begin(&document);
        let mut live = document.clone();
        live.crop(&Rect::new(0.0, 0.0, 5.0, 5.0));
        history.commit(&live);
        let restored = history.undo(&live).unwrap();
        assert_eq!(restored.shapes.len(), 1);
    }
}
