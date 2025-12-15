use click_lite::actions::{Backspace, SendMessage};
use click_lite::app::ClickLiteApp;
use gpui::{
    App, Application, Bounds, KeyBinding, WindowBounds, WindowOptions, prelude::*, px, size,
};

fn main() {
    let _ = dotenvy::dotenv();

    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("enter", SendMessage, None),
            KeyBinding::new("backspace", Backspace, None),
        ]);

        let bounds = Bounds::centered(None, size(px(980.), px(640.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let team_id = std::env::var("CLICKUP_WORKSPACE_ID")
                        .or_else(|_| std::env::var("CLICKUP_TEAM_ID"))
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok());

                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window);

                    let mut app = ClickLiteApp::new(team_id, focus_handle);
                    app.fetch_clickup_user(cx);
                    app.start_message_refresh(cx);
                    app
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
