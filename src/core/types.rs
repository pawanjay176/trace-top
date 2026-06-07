use std::{collections::HashMap, time::Duration};

use opentelemetry_proto::tonic::trace::v1::Span;

/// 16 byte trace id represented as a hex value.
pub type TraceId = String;
/// 8 byte span id represented as a hex value.
pub type SpanId = String;

/// Represents the useful components of a span that we care about
/// for our downstream consumers.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NormalizedSpan {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub name: String,
    /// Absolute wall-clock start timestamp from OTLP, in Unix epoch nanoseconds.
    pub start_unix_nano: u64,
    /// Absolute wall-clock end timestamp from OTLP, in Unix epoch nanoseconds.
    pub end_unix_nano: u64,
    /// TODO: store useful attributes here later.
    pub attributes: HashMap<String, String>,
}

impl From<Span> for NormalizedSpan {
    fn from(span: Span) -> Self {
        let attributes = span
            .attributes
            .into_iter()
            .filter_map(|kv| {
                if let Some(value) = kv.value.and_then(|v| v.value) {
                    Some((kv.key, format!("{:?}", value)))
                } else {
                    None
                }
            })
            .collect();
        Self {
            name: span.name,
            start_unix_nano: span.start_time_unix_nano,
            end_unix_nano: span.end_time_unix_nano,
            parent_span_id: if span.parent_span_id.is_empty() {
                None
            } else {
                Some(hex::encode(span.parent_span_id))
            },
            span_id: hex::encode(span.span_id),
            trace_id: hex::encode(span.trace_id),
            // TODO: fill in useful params like line number later.
            attributes,
        }
    }
}

/// Represents a trace with all of the spans it created.
#[derive(Debug, Clone)]
pub struct Trace {
    trace_id: TraceId,
    /// The root span id that created every other trace. Useful for display.
    /// TODO: claude tells me there can be multiple roots but I don't see any reason
    /// to support that as the use cases seem very pathological. Can change to a Vec later
    /// if required.
    root: Option<SpanId>,
    spans_by_id: HashMap<SpanId, NormalizedSpan>,
    latest_span_time: Duration,
}

impl Trace {
    pub fn new_from_normalized_span(span: NormalizedSpan) -> Self {
        let root = if span.parent_span_id.is_none() {
            Some(span.span_id.clone())
        } else {
            None
        };
        let latest_span_time = Duration::from_nanos(span.start_unix_nano);
        Self {
            trace_id: span.trace_id.clone(),
            spans_by_id: HashMap::from_iter([(span.span_id.clone(), span)]),
            root,
            latest_span_time,
        }
    }

    pub fn insert_span(&mut self, span: NormalizedSpan) -> bool {
        if span.trace_id != self.trace_id {
            return false;
        }
        // If the root span came in later, then update it
        if self.root.is_none() && span.parent_span_id.is_none() {
            self.root = Some(span.span_id.clone());
        }

        self.latest_span_time = self
            .latest_span_time
            .max(Duration::from_nanos(span.start_unix_nano));
        self.spans_by_id
            .insert(span.span_id.clone(), span)
            .is_some()
    }

    pub fn trace_id(&self) -> &TraceId {
        &self.trace_id
    }

    pub fn root_name(&self) -> Option<&str> {
        self.root
            .as_ref()
            .and_then(|root| self.spans_by_id.get(root))
            .map(|span| span.name.as_str())
    }

    /// Returns the start time of the earliest span within the trace.
    /// Unless root_span is None, this should be the start time for the root span.
    pub fn start_time(&self) -> Duration {
        Duration::from_nanos(self.start_unix_nano())
    }

    /// Returns the earliest absolute wall-clock timestamp in this trace, in
    /// Unix epoch nanoseconds.
    pub fn start_unix_nano(&self) -> u64 {
        self.spans_by_id
            .values()
            .map(|span| span.start_unix_nano)
            .min()
            .unwrap_or(0)
    }

    /// Returns the end time of the latest span within the trace.
    ///
    /// Note: this could be incomplete as a trace might not be complete as we are receiving
    /// more spans.
    pub fn end_time(&self) -> Duration {
        Duration::from_nanos(
            self.spans_by_id
                .values()
                .map(|span| span.end_unix_nano)
                .max()
                .unwrap_or(0),
        )
    }

    /// Returns the time difference between the first registered span under this
    /// trace and the last registered span under this trace.
    pub fn duration(&self) -> Duration {
        self.end_time().saturating_sub(self.start_time())
    }

    pub fn latest_span_time(&self) -> Duration {
        self.latest_span_time
    }

    pub fn span_count(&self) -> usize {
        self.spans_by_id.len()
    }

    pub fn spans(&self) -> impl Iterator<Item = &NormalizedSpan> {
        self.spans_by_id.values()
    }
}
