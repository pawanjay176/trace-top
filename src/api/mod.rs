use std::{collections::HashMap, error::Error, net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::core::{
    store::Store,
    types::{NormalizedSpan, Trace, TraceId},
};

const DEFAULT_SLOWEST_LIMIT: usize = 25;
const MAX_SLOWEST_LIMIT: usize = 500;

#[derive(Clone)]
struct ApiState {
    store: Arc<Store>,
}

#[derive(Debug, Deserialize)]
struct SpanQuery {
    name: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SpansResponse {
    List(SpanListResponse),
    Report(SpanReportResponse),
}

#[derive(Debug, Serialize)]
struct SpanListResponse {
    spans: Vec<SpanSummaryResponse>,
}

#[derive(Debug, Serialize)]
struct SpanSummaryResponse {
    name: String,
    stats: SpanStatsResponse,
}

#[derive(Debug, Serialize)]
struct SpanReportResponse {
    span_name: String,
    stats: SpanStatsResponse,
    slowest: Vec<SpanResponse>,
}

#[derive(Debug, Serialize)]
struct SpanStatsResponse {
    calls: usize,
    total_duration_nano: u64,
    mean_nano: u64,
    p50_nano: u64,
    p95_nano: u64,
    p99_nano: u64,
    max_nano: u64,
}

#[derive(Debug, Serialize)]
struct TraceResponse {
    trace_id: String,
    root_name: Option<String>,
    start_time_unix_nano: u64,
    duration_nano: u64,
    span_count: usize,
    spans: Vec<SpanResponse>,
}

#[derive(Debug, Serialize)]
struct SpanResponse {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    name: String,
    start_time_unix_nano: u64,
    duration_nano: u64,
    attributes: HashMap<String, String>,
    children: Vec<String>,
}

pub async fn serve(addr: &str, store: Arc<Store>) -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = addr.parse()?;
    let state = ApiState { store };
    let app = Router::new()
        .route("/api/spans", get(span_report))
        .route("/api/traces/{trace_id}", get(trace_detail))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn span_report(
    State(state): State<ApiState>,
    Query(query): Query<SpanQuery>,
) -> Result<Json<SpansResponse>, (StatusCode, Json<ApiError>)> {
    let Some(span_name) = query.name else {
        let response = state.store.read_traces(span_list_response);
        return Ok(Json(SpansResponse::List(response)));
    };
    if span_name.is_empty() {
        return Err(api_error(StatusCode::BAD_REQUEST, "name cannot be empty"));
    }

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SLOWEST_LIMIT)
        .min(MAX_SLOWEST_LIMIT);

    let response = state
        .store
        .read_traces(|traces| span_report_response(traces, span_name, limit));

    Ok(Json(SpansResponse::Report(response)))
}

fn span_list_response(traces: &HashMap<TraceId, Trace>) -> SpanListResponse {
    let mut durations_by_name: HashMap<String, Vec<u64>> = HashMap::new();

    for span in traces.values().flat_map(Trace::spans) {
        durations_by_name
            .entry(span.name.clone())
            .or_default()
            .push(span_duration_nano(span));
    }

    let mut spans = durations_by_name
        .into_iter()
        .map(|(name, durations)| SpanSummaryResponse {
            name,
            stats: span_stats(durations),
        })
        .collect::<Vec<_>>();

    spans.sort_by(|left, right| {
        right
            .stats
            .calls
            .cmp(&left.stats.calls)
            .then_with(|| left.name.cmp(&right.name))
    });

    SpanListResponse { spans }
}

fn span_report_response(
    traces: &HashMap<TraceId, Trace>,
    span_name: String,
    limit: usize,
) -> SpanReportResponse {
    let mut durations = Vec::new();
    let mut slowest = Vec::new();

    for trace in traces.values() {
        let children = children_by_parent(trace);
        for span in trace.spans().filter(|span| span.name == span_name) {
            durations.push(span_duration_nano(span));
            slowest.push(span_response(
                span,
                children.get(&span.span_id).cloned().unwrap_or_default(),
            ));
        }
    }

    slowest.sort_by_key(|span| std::cmp::Reverse(span.duration_nano));
    slowest.truncate(limit);

    SpanReportResponse {
        span_name,
        stats: span_stats(durations),
        slowest,
    }
}

async fn trace_detail(
    State(state): State<ApiState>,
    Path(trace_id): Path<String>,
) -> Result<Json<TraceResponse>, (StatusCode, Json<ApiError>)> {
    let response = state
        .store
        .read_trace(&trace_id, trace_response)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "trace not found"))?;

    Ok(Json(response))
}

fn trace_response(trace: &Trace) -> TraceResponse {
    let children = children_by_parent(trace);
    let mut spans = trace
        .spans()
        .map(|span| {
            span_response(
                span,
                children.get(&span.span_id).cloned().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();

    spans.sort_by_key(|span| span.start_time_unix_nano);

    TraceResponse {
        trace_id: trace.trace_id().clone(),
        root_name: trace.root_name().map(str::to_owned),
        start_time_unix_nano: trace.start_unix_nano(),
        duration_nano: trace.duration().as_nanos().min(u128::from(u64::MAX)) as u64,
        span_count: trace.span_count(),
        spans,
    }
}

fn span_response(span: &NormalizedSpan, mut children: Vec<String>) -> SpanResponse {
    children.sort();
    SpanResponse {
        trace_id: span.trace_id.clone(),
        span_id: span.span_id.clone(),
        parent_span_id: span.parent_span_id.clone(),
        name: span.name.clone(),
        start_time_unix_nano: span.start_unix_nano,
        duration_nano: span_duration_nano(span),
        attributes: span.attributes.clone(),
        children,
    }
}

fn children_by_parent(trace: &Trace) -> HashMap<String, Vec<String>> {
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for span in trace.spans() {
        if let Some(parent_span_id) = span.parent_span_id.as_ref() {
            children
                .entry(parent_span_id.clone())
                .or_default()
                .push(span.span_id.clone());
        }
    }
    children
}

fn span_stats(mut durations: Vec<u64>) -> SpanStatsResponse {
    durations.sort_unstable();
    let calls = durations.len();
    let total: u128 = durations.iter().map(|duration| u128::from(*duration)).sum();

    SpanStatsResponse {
        calls,
        total_duration_nano: total.min(u128::from(u64::MAX)) as u64,
        mean_nano: if calls == 0 {
            0
        } else {
            (total / calls as u128).min(u128::from(u64::MAX)) as u64
        },
        p50_nano: percentile_nearest_rank(&durations, 50),
        p95_nano: percentile_nearest_rank(&durations, 95),
        p99_nano: percentile_nearest_rank(&durations, 99),
        max_nano: durations.last().copied().unwrap_or(0),
    }
}

fn percentile_nearest_rank(sorted_values: &[u64], percentile: usize) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }

    let index = (sorted_values.len() * percentile).div_ceil(100) - 1;
    sorted_values[index.min(sorted_values.len() - 1)]
}

fn span_duration_nano(span: &NormalizedSpan) -> u64 {
    span.end_unix_nano.saturating_sub(span.start_unix_nano)
}

fn api_error(status: StatusCode, error: &str) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: error.to_string(),
        }),
    )
}
