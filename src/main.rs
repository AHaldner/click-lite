use click_lite::app::ClickLiteApp;
use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_component::Root;
use gpui_component::input::InputState;

fn main() {
    let _ = dotenvy::dotenv();

    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(980.), px(640.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| {
                    let team_id = std::env::var("CLICKUP_WORKSPACE_ID")
                        .or_else(|_| std::env::var("CLICKUP_TEAM_ID"))
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok());

                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window);

                    let message_input = cx.new(|cx| {
                        InputState::new(window, cx)
                            .auto_grow(1, 6)
                            .placeholder("Select a chat to start messaging...")
                    });

                    let mut app = ClickLiteApp::new(
                        team_id,
                        focus_handle,
                        window.window_handle(),
                        message_input,
                        cx,
                    );
                    app.fetch_clickup_user(cx);
                    app.start_message_refresh(cx);
                    app
                });

                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
