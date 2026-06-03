use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicU64},
};
use tokio::sync::mpsc;

use crate::core::types::{NormalizedSpan, SpanId, Trace, TraceId};

pub struct StoreVersion(pub AtomicU64);

#[derive(Clone, Debug, Default)]
pub struct TraceListQuery {
    pub limit: usize,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TraceListSnapshot {
    pub version: u64,
    pub rows: Vec<TraceSummary>,
    pub total_traces: usize,
    pub total_spans: usize,
}

#[derive(Clone, Debug)]
pub struct TraceSummary {
    pub trace_id: TraceId,
    pub root_name: Option<String>,
    pub start_unix_nano: u64,
    pub duration_nano: u64,
    pub span_count: usize,
    pub error_count: usize,
}

#[derive(Clone, Debug)]
pub struct TraceDetailSnapshot {
    pub version: u64,
    pub trace_id: TraceId,
    pub rows: Vec<SpanRow>,
    pub selected_span: Option<SpanDetails>,
}

#[derive(Clone, Debug)]
pub struct SpanRow {
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub depth: usize,
    pub name: String,
    pub start_unix_nano: u64,
    pub end_unix_nano: u64,
}

#[derive(Clone, Debug)]
pub struct SpanDetails {
    pub span_id: SpanId,
    pub name: String,
    pub start_unix_nano: u64,
    pub end_unix_nano: u64,
    pub attributes: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct AggregateQuery {
    pub span_name_search: Option<String>,
    pub group_by_attribute: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AggregateSnapshot {
    pub version: u64,
    pub rows: Vec<AggregateRow>,
}

#[derive(Clone, Debug)]
pub struct AggregateRow {
    pub span_name: String,
    pub group: Option<String>,
    pub calls: usize,
    pub mean_nano: u64,
    pub p50_nano: u64,
    pub p95_nano: u64,
    pub max_nano: u64,
    pub error_count: usize,
}

/// Events that the store can receive from producers and consumers.
pub enum StoreReceiver {
    /// Receive a list of spans from a resource.
    ///
    /// This is received over a receiver channel from the server which gets
    /// and parses trace events.
    ReceivedSpans(Vec<NormalizedSpan>),
}

/// Events emitted by the store to notify downstream consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreEvent {
    Updated { store_version: u64 },
}

/// A simple key value store that stores all traces received over the server.
/// The store is responsible for storing and serving all the traces received to
/// any downstream consumers.
///
/// This applies all the policies wrt the long term storage, filtering and eviction
/// of received traces to maintain the store policies.
///
/// Store policies can include things like:
/// - maximum number of traces to store.
/// - maximum size of the store (e.g. hashmap can grow to max 4gb)
pub struct Store {
    traces: Arc<Mutex<HashMap<TraceId, Trace>>>,
    version: StoreVersion,
}

impl Store {
    pub fn new() -> Self {
        Self {
            version: StoreVersion(0.into()),
            traces: Default::default(),
        }
    }

    pub fn recent_traces(&self, _query: TraceListQuery) -> TraceListSnapshot {
        unimplemented!("store recent_traces query will be implemented by store owner")
    }

    pub fn trace_detail(
        &self,
        _trace_id: &TraceId,
        _selected_span: Option<&SpanId>,
    ) -> Option<TraceDetailSnapshot> {
        unimplemented!("store trace_detail query will be implemented by store owner")
    }

    pub fn aggregate(&self, _query: AggregateQuery) -> AggregateSnapshot {
        unimplemented!("store aggregate query will be implemented by store owner")
    }

    /// Takes a vector of `NormalizedSpan` received from the server and inserts it into
    /// the internal store.
    ///
    /// Also emits events based on the received spans for downstream consumers.
    pub fn insert_spans(&self, spans: Vec<NormalizedSpan>) -> Vec<StoreEvent> {
        let mut store = self.traces.lock();
        let changed = !spans.is_empty();
        for span in spans.into_iter() {
            let trace_id = span.trace_id.clone();

            if let Some(trace) = store.get_mut(&trace_id) {
                let _ = trace.insert_span(span);
            } else {
                let trace = Trace::new_from_normalized_span(span);
                store.insert(trace_id, trace.clone());
            }
        }
        if changed {
            let version = self
                .version
                .0
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                + 1;
            vec![StoreEvent::Updated {
                store_version: version,
            }]
        } else {
            Vec::new()
        }
    }

    pub async fn run(
        self: Arc<Self>,
        mut ingest_rx: mpsc::Receiver<StoreReceiver>,
        tui_tx: mpsc::Sender<Vec<StoreEvent>>,
    ) {
        loop {
            tokio::select! {
                event = ingest_rx.recv() => {
                    match event {
                        Some(StoreReceiver::ReceivedSpans(spans)) => {

                            let events = self.insert_spans(spans);
                            if !events.is_empty() {
                                let _ = tui_tx.send(events).await;
                            }
                        }
                        None => break,
                    }
                }
                else => break,
            }
        }
    }
}
