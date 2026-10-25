use super::*;
use crossbeam_channel::unbounded;

pub(crate) fn create_test_app() -> App {
    static CNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (_tx, rx) = unbounded();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tmp = std::env::temp_dir().join(format!("lgtm-test-{}-{}", unique, id));
    let _ = std::fs::create_dir_all(&tmp);
    let mut app = App::new(tmp, rx, None, None, None);
    app.forge_factory = |_| Box::new(lgtm_core::forge::Unsupported("test"));
    app.comments.clear();
    app.agent_sink_kind = Some(crate::ipc::SinkKind::Dummy);
    app
}

pub(crate) fn wait_for_local_refresh(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while app.local_refresh_pending() {
        app.handle_background_events();
        assert!(
            std::time::Instant::now() < deadline,
            "workspace refresh did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

pub(crate) fn wait_for_audit(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while app.audit_pending() {
        app.handle_background_events();
        assert!(
            std::time::Instant::now() < deadline,
            "audit did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

pub(crate) fn wait_for_view_data(app: &mut App) {
    app.prepare_view_data();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while app.view_data_pending() {
        app.handle_background_events();
        assert!(
            std::time::Instant::now() < deadline,
            "view data did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
