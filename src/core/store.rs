use parking_lot::Mutex;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use crate::core::types::{NormalizedSpan, SpanId, Trace, TraceId};

#[derive(Clone, Debug, Default)]
pub struct TraceListQuery {
    pub limit: usize,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TraceListSnapshot {
    pub rows: Vec<TraceSummary>,
    pub total_traces: usize,
    pub total_spans: usize,
}

#[derive(Clone, Debug)]
pub struct TraceSummary {
    pub trace_id: TraceId,
    pub root_name: Option<String>,
    /// Absolute wall-clock start timestamp in since unix epoch time
    /// in nanoseconds.
    pub start_unix_nano: u64,
    pub duration: Duration,
    pub span_count: usize,
}

#[derive(Clone, Debug)]
pub struct TraceDetailSnapshot {
    pub rows: Vec<SpanRow>,
    pub selected_span: Option<SpanDetails>,
}

#[derive(Clone, Debug)]
pub struct SpanRow {
    pub span_id: SpanId,
    pub depth: usize,
    pub name: String,
    /// Absolute wall-clock start timestamp for this span, in Unix epoch nanoseconds.
    pub start_unix_nano: u64,
    /// Absolute wall-clock end timestamp for this span, in Unix epoch nanoseconds.
    pub end_unix_nano: u64,
}

#[derive(Clone, Debug)]
pub struct SpanDetails {
    pub span_id: SpanId,
    pub name: String,
    /// Absolute wall-clock start timestamp for this span, in Unix epoch nanoseconds.
    pub start_unix_nano: u64,
    /// Absolute wall-clock end timestamp for this span, in Unix epoch nanoseconds.
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
#[derive(Debug)]
pub struct Store {
    traces: Mutex<HashMap<TraceId, Trace>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            traces: Default::default(),
        }
    }

    /// Returns a list of most recent traces based on the query.
    pub fn recent_traces(&self, query: TraceListQuery) -> TraceListSnapshot {
        let store = self.traces.lock();
        let mut rows = store
            .values()
            .filter(|trace| {
                query
                    .search
                    .as_ref()
                    .is_none_or(|search| trace.spans().any(|span| span.name.contains(search)))
            })
            .map(|trace| TraceSummary {
                trace_id: trace.trace_id().clone(),
                root_name: trace.root_name().map(str::to_owned),
                start_unix_nano: trace.start_unix_nano(),
                duration: trace.duration(),
                span_count: trace.span_count(),
            })
            .collect::<Vec<_>>();

        rows.sort_by_key(|row| {
            store
                .get(&row.trace_id)
                .map(|trace| std::cmp::Reverse(trace.latest_span_time()))
        });
        rows.truncate(query.limit);

        TraceListSnapshot {
            rows,
            total_traces: store.len(),
            total_spans: store.values().map(Trace::span_count).sum(),
        }
    }

    pub fn trace_detail(
        &self,
        trace_id: &TraceId,
        selected_span: Option<&SpanId>,
    ) -> Option<TraceDetailSnapshot> {
        let store = self.traces.lock();
        let trace = store.get(trace_id)?;
        let rows = span_rows(trace);
        let selected_span = selected_span
            .and_then(|span_id| trace.span(span_id))
            .or_else(|| rows.first().and_then(|row| trace.span(&row.span_id)))
            .map(|span| SpanDetails {
                span_id: span.span_id.clone(),
                name: span.name.clone(),
                start_unix_nano: span.start_unix_nano,
                end_unix_nano: span.end_unix_nano,
                attributes: span
                    .attributes
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            });

        Some(TraceDetailSnapshot {
            rows,
            selected_span,
        })
    }

    pub fn aggregate(&self, query: AggregateQuery) -> AggregateSnapshot {
        let store = self.traces.lock();
        let mut groups: HashMap<(String, Option<String>), Vec<u64>> = HashMap::new();

        for span in store.values().flat_map(Trace::spans) {
            if query
                .span_name_search
                .as_ref()
                .is_some_and(|search| !span.name.contains(search))
            {
                continue;
            }

            let group = query
                .group_by_attribute
                .as_ref()
                .and_then(|key| span.attributes.get(key).cloned());
            let duration_nano = span.end_unix_nano.saturating_sub(span.start_unix_nano);

            groups
                .entry((span.name.clone(), group))
                .or_default()
                .push(duration_nano);
        }

        let mut rows = groups
            .into_iter()
            .map(|((span_name, group), mut durations)| {
                durations.sort_unstable();
                let calls = durations.len();
                let total: u128 = durations.iter().map(|duration| u128::from(*duration)).sum();

                AggregateRow {
                    span_name,
                    group,
                    calls,
                    mean_nano: (total / calls as u128).min(u128::from(u64::MAX)) as u64,
                    p50_nano: percentile_nearest_rank(&durations, 50),
                    p95_nano: percentile_nearest_rank(&durations, 95),
                    max_nano: durations.last().copied().unwrap_or(0),
                    error_count: 0,
                }
            })
            .collect::<Vec<_>>();

        rows.sort_by(|left, right| {
            left.span_name
                .cmp(&right.span_name)
                .then_with(|| left.group.cmp(&right.group))
        });

        AggregateSnapshot { rows }
    }

    /// Takes a vector of `NormalizedSpan` received from the server and inserts it into
    /// the internal store.
    ///
    /// This is intentionally silent. Consumers pull snapshots from the store when
    /// they want a refreshed view.
    pub fn insert_spans(&self, spans: Vec<NormalizedSpan>) {
        let mut store = self.traces.lock();
        for span in spans.into_iter() {
            let trace_id = span.trace_id.clone();

            if let Some(trace) = store.get_mut(&trace_id) {
                let _ = trace.insert_span(span);
            } else {
                let trace = Trace::new_from_normalized_span(span);
                store.insert(trace_id, trace.clone());
            }
        }
    }
}

fn span_rows(trace: &Trace) -> Vec<SpanRow> {
    let mut children: HashMap<Option<SpanId>, Vec<_>> = HashMap::new();
    for span in trace.spans() {
        children
            .entry(span.parent_span_id.clone())
            .or_default()
            .push(span);
    }

    for spans in children.values_mut() {
        spans.sort_by_key(|span| span.start_unix_nano);
    }

    let mut rows = Vec::new();
    let mut visited = HashSet::new();
    append_children(None, 0, &children, &mut visited, &mut rows);

    for span in trace.spans() {
        if visited.insert(span.span_id.clone()) {
            rows.push(SpanRow {
                span_id: span.span_id.clone(),
                depth: 0,
                name: span.name.clone(),
                start_unix_nano: span.start_unix_nano,
                end_unix_nano: span.end_unix_nano,
            });
        }
    }

    rows
}

fn percentile_nearest_rank(sorted_values: &[u64], percentile: usize) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }

    let index = (sorted_values.len() * percentile).div_ceil(100) - 1;
    sorted_values[index.min(sorted_values.len() - 1)]
}

fn append_children(
    parent: Option<SpanId>,
    depth: usize,
    children: &HashMap<Option<SpanId>, Vec<&NormalizedSpan>>,
    visited: &mut HashSet<SpanId>,
    rows: &mut Vec<SpanRow>,
) {
    let Some(spans) = children.get(&parent) else {
        return;
    };

    for span in spans {
        if !visited.insert(span.span_id.clone()) {
            continue;
        }

        rows.push(SpanRow {
            span_id: span.span_id.clone(),
            depth,
            name: span.name.clone(),
            start_unix_nano: span.start_unix_nano,
            end_unix_nano: span.end_unix_nano,
        });
        append_children(
            Some(span.span_id.clone()),
            depth + 1,
            children,
            visited,
            rows,
        );
    }
}
