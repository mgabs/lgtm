use super::*;
use crate::app::rows::Key;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::Rc,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum Anchor {
    Fold {
        raw: usize,
    },
    Unified {
        raw: usize,
        hunk: usize,
        line: usize,
        pair: Option<(usize, bool)>,
    },
    Split {
        hunk: usize,
        old: Option<usize>,
        new: Option<usize>,
        base: usize,
        context: bool,
    },
}

#[derive(Default)]
pub(super) struct Rows {
    pub lines: Vec<Line<'static>>,
    pub anchors: Vec<Option<Anchor>>,
    pub offsets: Vec<usize>,
    pub source_rows: HashMap<usize, usize>,
}

impl<'a> From<Vec<Line<'a>>> for Rows {
    fn from(lines: Vec<Line<'a>>) -> Self {
        let mut rows = Self::default();
        for line in lines {
            rows.push(line);
        }
        rows
    }
}

impl Rows {
    pub fn push(&mut self, line: Line<'_>) {
        let mut owned = Line::from(
            line.spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect::<Vec<_>>(),
        );
        owned.alignment = line.alignment;
        owned.style = line.style;
        self.lines.push(owned);
        self.anchors.push(None);
    }
    pub fn tag(&mut self, anchor: Anchor) {
        let row = self.lines.len() - 1;
        self.anchors[row] = Some(anchor);
        match anchor {
            Anchor::Fold { raw } | Anchor::Unified { raw, .. } => {
                self.source_rows.insert(raw, row);
            }
            Anchor::Split { old, new, base, .. } => {
                for index in old.into_iter().chain(new) {
                    self.source_rows.insert(base + index, row);
                }
            }
        }
    }
    pub fn len(&self) -> usize {
        self.lines.len()
    }
    pub fn wrap(&mut self, width: u16) {
        self.offsets = Vec::with_capacity(self.lines.len() + 1);
        self.offsets.push(0);
        for line in &self.lines {
            let height = Paragraph::new(line.clone())
                .wrap(Wrap { trim: false })
                .line_count(width)
                .max(1);
            self.offsets.push(self.offsets.last().unwrap() + height);
        }
    }
    pub fn total_height(&self) -> usize {
        self.offsets.last().copied().unwrap_or(0)
    }
}

#[derive(Default)]
pub(super) struct Index {
    pub file_alerts: Vec<usize>,
    pub alerts: HashMap<u32, Vec<usize>>,
    pub comments: HashMap<u32, Vec<usize>>,
    pub threads: HashMap<u32, bool>,
    pub alert_counts: HashMap<String, usize>,
}

impl Index {
    fn new(app: &App, name: &str) -> Self {
        let mut index = Self::default();
        for (i, alert) in app.alerts.iter().enumerate() {
            *index
                .alert_counts
                .entry(alert.file_path.clone())
                .or_default() += 1;
            if alert.file_path == name {
                index.file_alerts.push(i);
                if let Some(line) = alert.line_no {
                    index.alerts.entry(line).or_default().push(i);
                }
            }
        }
        let clean = name.trim_start_matches("./");
        for (i, comment) in app.comments.iter().enumerate() {
            let path = comment.file_path.trim_start_matches("./");
            if !comment.orphaned
                && (clean == path || clean.ends_with(path) || path.ends_with(clean))
            {
                index
                    .comments
                    .entry(comment.target_line_no)
                    .or_default()
                    .push(i);
            }
        }
        for thread in &app.threads {
            if thread.path == name {
                if let Some(line) = thread.line {
                    *index.threads.entry(line).or_default() |= !thread.resolved;
                }
            }
        }
        index
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct LayoutKey {
    pub source: Key,
    pub width: u16,
    pub split: bool,
    pub annotations: u64,
    pub git: bool,
}

pub(super) struct Layout {
    pub key: LayoutKey,
    pub title: String,
    pub rows: Rows,
}
type WordPair = (Vec<Span<'static>>, Vec<Span<'static>>);

#[derive(Default)]
pub(crate) struct RenderCache {
    index: Option<(u64, Rc<Index>)>,
    pub(super) layout: Option<Layout>,
    word_source: Option<Key>,
    words: HashMap<(usize, usize, usize), Rc<WordPair>>,
    reviewed_source: Option<(Key, u64)>,
    reviewed: HashMap<usize, usize>,
    pub(crate) layouts_prepared: u64,
    pub(crate) words_prepared: u64,
    pub(crate) viewport_rows: usize,
    pub(crate) drawer_rows: usize,
}

pub(super) fn annotation_signature(app: &App) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    app.snapshot
        .files
        .get(app.selected_file_idx)
        .map(|f| &f.new_path)
        .hash(&mut hash);
    app.comments.len().hash(&mut hash);
    for c in &app.comments {
        (
            &c.file_path,
            c.target_line_no,
            c.orphaned,
            c.is_resolved,
            &c.comment_text,
            &c.replies,
        )
            .hash(&mut hash);
    }
    app.alerts.len().hash(&mut hash);
    for a in &app.alerts {
        (
            &a.file_path,
            a.line_no,
            a.severity as u8,
            &a.title,
            &a.description,
        )
            .hash(&mut hash);
    }
    app.threads.len().hash(&mut hash);
    for t in &app.threads {
        (&t.path, t.line, t.resolved).hash(&mut hash);
    }
    hash.finish()
}

impl RenderCache {
    pub(super) fn prepare_content(&mut self, key: Key) {
        if self.word_source != Some(key) {
            self.word_source = Some(key);
            self.words.clear();
        }
    }
    pub(super) fn index(&mut self, app: &App, signature: u64) -> Rc<Index> {
        if let Some((old, index)) = &self.index {
            if *old == signature {
                return index.clone();
            }
        }
        let _timing = lgtm_core::debug::Span::new("ui.annotations_prepare");
        let name = app
            .snapshot
            .files
            .get(app.selected_file_idx)
            .map(|f| f.display_name())
            .unwrap_or_default();
        let index = Rc::new(Index::new(app, &name));
        self.index = Some((signature, index.clone()));
        index
    }
    pub(super) fn word_spans(
        &mut self,
        app: &App,
        hunk: usize,
        old: usize,
        new: usize,
        deletion: bool,
    ) -> Vec<Span<'static>> {
        let key = Key::of(app).content();
        self.prepare_content(key);
        let pair = self.words.entry((hunk, old, new)).or_insert_with(|| {
            let _timing = lgtm_core::debug::Span::new("ui.words_prepare");
            self.words_prepared += 1;
            let lines = &app.snapshot.files[app.selected_file_idx].hunks[hunk].lines;
            Rc::new(prepare_inline_diff_pair(
                &lines[old].content,
                &lines[new].content,
                None,
            ))
        });
        if deletion {
            pair.0.clone()
        } else {
            pair.1.clone()
        }
    }
    pub(super) fn prepare_reviewed(&mut self, app: &App) {
        if app.marks.is_empty() {
            return;
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        for mark in &app.marks {
            (
                &mark.file,
                &mark.content_hash,
                mark.last_line,
                &mark.reviewed_at,
            )
                .hash(&mut hash);
        }
        // A selected-file change does not alter the document's reviewed counts.
        let mut key = Key::of(app).content();
        key.for_document();
        let source = (key, hash.finish());
        if self.reviewed_source != Some(source) {
            self.reviewed_source = Some(source);
            self.reviewed.clear();
        }
    }
    pub(super) fn reviewed_count(&mut self, app: &App, index: usize) -> usize {
        if app.marks.is_empty() {
            return 0;
        }
        *self.reviewed.entry(index).or_insert_with(|| {
            lgtm_core::review::reviewed_flags(&app.marks, &app.snapshot.files[index])
                .into_iter()
                .filter(|b| *b)
                .count()
        })
    }
}
