use std::collections::HashMap;

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
    pub start_unix_nano: u64,
    pub end_unix_nano: u64,
    /// TODO: store useful attributes here later.
    pub attributes: HashMap<String, String>,
}

impl From<Span> for NormalizedSpan {
    fn from(span: Span) -> Self {
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
            attributes: HashMap::new(),
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
}

impl Trace {
    pub fn new_from_normalized_span(span: NormalizedSpan) -> Self {
        let root = if span.parent_span_id.is_none() {
            Some(span.span_id.clone())
        } else {
            None
        };
        Self {
            trace_id: span.trace_id.clone(),
            spans_by_id: HashMap::from_iter([(span.span_id.clone(), span)]),
            root,
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
        self.spans_by_id
            .insert(span.span_id.clone(), span)
            .is_some()
    }
}
