#![allow(unused_imports)]
use super::*;
use crate::model::*;
use crossbeam_channel::unbounded;
use std::path::PathBuf;

pub(crate) fn create_test_app() -> App {
    let (_tx, rx) = unbounded();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tmp = std::env::temp_dir().join(format!("lgtm-test-{}", unique));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = App::new(tmp, rx, None, None, None);
    app.comments.clear();
    app
}
