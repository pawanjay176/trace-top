mod core;
mod tui;

use std::{env, error::Error, sync::Arc};
use tokio::sync::{mpsc, oneshot};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4317".to_string());
    let store = Arc::new(core::store::Store::new());
    let (ingest_tx, ingest_rx) = mpsc::channel(1024);
    let (root_span_tx, root_span_rx) = mpsc::channel(1024);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let store_task = tokio::spawn({
        let store = store.clone();
        async move {
            store.run(ingest_rx, root_span_tx).await;
        }
    });

    let server_addr = addr.clone();
    let server_task = tokio::spawn(async move {
        if let Err(err) = core::server::serve(&server_addr, ingest_tx, shutdown_rx).await {
            eprintln!("otlp server exited with error: {err}");
        }
    });

    let tui_result = tui::run(store.clone(), root_span_rx);
    drop(shutdown_tx);
    let _ = server_task.await;
    let _ = store_task.await;
    tui_result
}
