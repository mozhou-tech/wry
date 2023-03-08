// Copyright 2020-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use once_cell::sync::Lazy;
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::platform::macos::WindowBuilderExtMacOS;

static mut tabbing_window_size: PhysicalSize<u32> = PhysicalSize::new(2400, 1000);
static mut tabbing_window_position: PhysicalPosition<i32> = PhysicalPosition::new(300, 100);

fn main() -> wry::Result<()> {
    use std::collections::HashMap;
    use wry::{
        application::{
            event::{Event, StartCause, WindowEvent},
            event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
            window::{Window, WindowBuilder, WindowId},
        },
        webview::{WebView, WebViewBuilder},
    };

    const HTML: &str = r#"
  <html>
  <head>
      <style>
          html {
            font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
          }

          * {
              padding: 0;
              margin: 0;
              box-sizing: border-box;
          }

          main {
            display: grid;
            place-items: center;
            height: calc(100vh - 30px);
          }

          .titlebar {
              height: 30px;
              padding-left: 5px;
              display: grid;
              grid-auto-flow: column;
              grid-template-columns: 1fr max-content max-content max-content;
              align-items: center;
              background: #1F1F1F;
              color: white;
              user-select: none;
          }

          .titlebar-button {
              display: inline-flex;
              justify-content: center;
              align-items: center;
              width: 30px;
              height: 30px;
          }

          .titlebar-button:hover {
              background: #3b3b3b;
          }

          .titlebar-button#close:hover {
              background: #da3d3d;
          }

          .titlebar-button img {
              filter: invert(100%);
          }
      </style>
  </head>

  <body>
      <div class="titlebar">
          <div class="drag-region">Custom Titlebar</div>
          <div>
            <div class="titlebar-button" onclick="window.ipc.postMessage('new_window')">
                  <button>+</button>
              </div>
              <div class="titlebar-button" onclick="window.ipc.postMessage('minimize')">
                  <img src="https://api.iconify.design/codicon:chrome-minimize.svg" />
              </div>
              <div class="titlebar-button" onclick="window.ipc.postMessage('maximize')">
                  <img src="https://api.iconify.design/codicon:chrome-maximize.svg" />
              </div>

              <div class="titlebar-button" id="close" onclick="window.ipc.postMessage('close_all')">
                  <img src="https://api.iconify.design/codicon:close.svg" />
              </div>
          </div>
      </div>
      <main>
          <h4> WRYYYYYYYYYYYYYYYYYYYYYY! </h4>
      </main>
      <script>
          document.addEventListener('mousedown', (e) => {
              if (e.target.classList.contains('drag-region') && e.buttons === 1) {
                  e.detail === 2
                      ? window.ipc.postMessage('maximize')
                      : window.ipc.postMessage('drag_window');
              }
          })
          document.addEventListener('touchstart', (e) => {
              if (e.target.classList.contains('drag-region')) {
                  window.ipc.postMessage('drag_window');
              }
          })
      </script>
  </body>
  </html>
"#;

    enum UserEvents {
        CloseWindow(WindowId),
        NewWindow(),
        CloseAllWindow
    }

    fn create_new_window(
        title: String,
        event_loop: &EventLoopWindowTarget<UserEvents>,
        proxy: EventLoopProxy<UserEvents>,
    ) -> (WindowId, WebView) {
        let mut window: Window;
        unsafe {
            window = WindowBuilder::new()
                .with_title(title)
                .with_automatic_window_tabbing(false)
                .with_decorations(false)
                .with_position(tabbing_window_position.clone())
                .with_inner_size(tabbing_window_size.clone())
                .build(event_loop)
                .unwrap();
        }

        let window_id = window.id();

        let handler = move |window: &Window, req: String| {
            if req == "minimize" {
                window.set_minimized(true);
            }
            if req == "maximize" {
                window.set_maximized(!window.is_maximized());
            }
            if req == "close" {
                let _ = proxy.send_event(UserEvents::CloseWindow(window_id));
            }
            if req == "close_all" {
                let _ = proxy.send_event(UserEvents::CloseAllWindow);
            }
            if req == "drag_window" {
                let _ = window.drag_window();
            }
            if req == "new_window" {
                let _ = proxy.send_event(UserEvents::NewWindow());
            }
        };
        let webview = WebViewBuilder::new(window)
            .unwrap()
            .with_html(HTML)
            .unwrap()
            .with_ipc_handler(handler)
            .build()
            .unwrap();
        (window_id, webview)
    }

    let event_loop = EventLoop::<UserEvents>::with_user_event();
    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    let new_window = create_new_window(
        format!("Window {}", webviews.len() + 1),
        &event_loop,
        proxy.clone(),
    );
    webviews.insert(new_window.0, new_window.1);

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => {
                    webviews.remove(&window_id);
                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit
                    }
                }
                WindowEvent::Moved(p) => {
                    unsafe {
                        tabbing_window_position = p;
                    }
                    for (_window_id, webview) in &webviews {
                        webview.window().set_outer_position(p)
                    }
                }
                WindowEvent::Resized(_) => {
                    let current_window = &webviews[&window_id];
                    for (window_id, webview) in &webviews {
                        if *window_id != current_window.window().id() {
                            webview.window().set_inner_size(current_window.inner_size());
                            unsafe {
                                tabbing_window_size = current_window.inner_size();
                            }
                        }
                    }
                }
                _ => (),
            },
            Event::UserEvent(UserEvents::NewWindow()) => {
                let new_window = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    &event_loop,
                    proxy.clone(),
                );
                webviews.insert(new_window.0, new_window.1);
            }
            Event::UserEvent(UserEvents::CloseAllWindow) => {
                webviews.clear();
            }
            Event::UserEvent(UserEvents::CloseWindow(id)) => {
                webviews.remove(&id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            _ => (),
        }
    });
}
