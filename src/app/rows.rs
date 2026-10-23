//! Source-line mapping shared by navigation and rendering; independent of cursor styling.
use super::*;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    revision: u64,
    folds: u64,
    address: usize,
    files: usize,
    selected: usize,
}

impl Key {
    pub(crate) fn of(app: &App) -> Self {
        Self {
            revision: app.diff_revision,
            folds: app.fold_revision,
            address: app.snapshot.files.as_ptr() as usize,
            files: app.snapshot.files.len(),
            selected: app.selected_file_idx,
        }
    }
    pub(crate) fn content(mut self) -> Self {
        self.folds = 0;
        self
    }
    pub(crate) fn for_document(&mut self) {
        self.selected = usize::MAX;
    }
}

#[derive(Default)]
pub(crate) struct Mapping {
    pub(crate) visible: Rc<Vec<usize>>,
    pub(crate) starts: Vec<usize>,
    pub(crate) folds: Vec<Vec<Option<lgtm_core::model::BlockRange>>>,
    pub(crate) representatives: Vec<usize>,
}

#[derive(Default)]
pub(crate) struct Cache {
    value: Option<(Key, Rc<Mapping>)>,
    pub(crate) preparations: u64,
}

impl Cache {
    fn get(&mut self, key: Key, file: Option<&FileDiff>) -> Rc<Mapping> {
        if let Some((old, mapping)) = &self.value {
            if *old == key {
                return mapping.clone();
            }
        }
        let mut timing = lgtm_core::debug::Span::new("ui.rows_prepare");
        let mut mapping = Mapping::default();
        let mut visible = Vec::new();
        if let Some(file) = file {
            let mut offset = 0;
            for hunk in &file.hunks {
                mapping.starts.push(offset);
                let folds = App::fold_map(hunk);
                let mut seen = std::collections::HashMap::new();
                for (index, folded) in folds.iter().enumerate() {
                    let raw = offset + index;
                    let representative = if hunk.is_folded {
                        offset
                    } else if let Some(range) = folded {
                        *seen
                            .entry((range.start_line, range.end_line))
                            .or_insert(raw)
                    } else {
                        raw
                    };
                    if representative == raw {
                        visible.push(raw);
                    }
                    mapping.representatives.push(representative);
                }
                offset += hunk.lines.len();
                mapping.folds.push(folds);
            }
        }
        timing.count("lines", mapping.representatives.len() as u64);
        timing.count("visible", visible.len() as u64);
        mapping.visible = Rc::new(visible);
        let mapping = Rc::new(mapping);
        self.preparations += 1;
        self.value = Some((key, mapping.clone()));
        mapping
    }
}

impl App {
    pub(crate) fn cached_rows(&self) -> Rc<Mapping> {
        self.rows_cache.borrow_mut().get(
            Key::of(self),
            self.snapshot.files.get(self.selected_file_idx),
        )
    }
    pub(crate) fn invalidate_diff_content(&mut self) {
        self.diff_revision = self.diff_revision.wrapping_add(1);
        self.invalidate_audit(true);
        self.invalidate_view_data();
    }
    pub(super) fn invalidate_diff_folds(&mut self) {
        self.fold_revision = self.fold_revision.wrapping_add(1);
    }
}
