mod core;
mod tui;

use std::{env, error::Error, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4317".to_string());
    let store = Arc::new(core::store::Store::new());

    let server_addr = addr.clone();
    let server_store = store.clone();
    let server_task = tokio::spawn(async move {
        if let Err(err) = core::server::serve(&server_addr, server_store).await {
            eprintln!("otlp server exited with error: {err}");
        }
    });

    let tui_result = tui::run(store);
    // Kill the server once the tui is quit
    server_task.abort();
    let _ = server_task.await;
    tui_result
}
