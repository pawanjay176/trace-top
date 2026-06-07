use crate::core::{
    store::{
        AggregateQuery, AggregateSnapshot, AggregateSpansQuery, AggregateSpansSnapshot, Store,
        TraceDetailSnapshot, TraceListQuery, TraceListSnapshot,
    },
    types::{SpanId, TraceId},
};

const DEFAULT_TRACE_LIST_LIMIT: usize = 250;

#[derive(Clone, Debug)]
pub enum Action {
    Noop,
    Quit,
    Resize(u16, u16),
    RefreshCurrentScreen,
    RefreshTraceList,
    ShowTraceList,
    ShowTraceDetail,
    ShowAggregates,
    ShowAggregateSpans,
    OpenAggregateSpanTrace,
    MoveSelectionDown,
    MoveSelectionUp,
    MoveSpanDown,
    MoveSpanUp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    TraceList,
    TraceDetail,
    Aggregates,
    AggregateSpans,
}

#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    pub should_quit: bool,
    pub terminal_size: Option<(u16, u16)>,
    pub trace_list_query: TraceListQuery,
    pub trace_list: TraceListSnapshot,
    pub selected_trace_index: usize,
    pub selected_trace_id: Option<TraceId>,
    pub selected_trace: Option<TraceDetailSnapshot>,
    pub selected_span_index: usize,
    pub aggregate_query: AggregateQuery,
    pub aggregate: AggregateSnapshot,
    pub selected_aggregate_index: usize,
    pub aggregate_spans_query: Option<AggregateSpansQuery>,
    pub aggregate_spans: AggregateSpansSnapshot,
    pub selected_aggregate_span_index: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            screen: Screen::TraceList,
            should_quit: false,
            terminal_size: None,
            trace_list_query: TraceListQuery {
                limit: DEFAULT_TRACE_LIST_LIMIT,
                search: None,
            },
            trace_list: TraceListSnapshot::default(),
            selected_trace_index: 0,
            selected_trace_id: None,
            selected_trace: None,
            selected_span_index: 0,
            aggregate_query: AggregateQuery::default(),
            aggregate: AggregateSnapshot::default(),
            selected_aggregate_index: 0,
            aggregate_spans_query: None,
            aggregate_spans: AggregateSpansSnapshot::default(),
            selected_aggregate_span_index: 0,
        }
    }

    pub fn update(&mut self, action: Action, store: &Store) {
        match action {
            Action::Noop => {}
            Action::Quit => self.should_quit = true,
            Action::Resize(width, height) => self.terminal_size = Some((width, height)),
            Action::RefreshCurrentScreen => self.refresh_current_screen(store),
            Action::RefreshTraceList => self.refresh_trace_list(store),
            Action::ShowTraceList => {
                self.screen = Screen::TraceList;
                self.refresh_trace_list(store);
            }
            Action::ShowTraceDetail => self.open_selected_trace(store),
            Action::ShowAggregates => {
                self.screen = Screen::Aggregates;
                self.refresh_aggregates(store);
            }
            Action::ShowAggregateSpans => self.open_selected_aggregate_row(store),
            Action::OpenAggregateSpanTrace => self.open_selected_aggregate_span_trace(store),
            Action::MoveSelectionDown => self.move_trace_selection_down(store),
            Action::MoveSelectionUp => self.move_trace_selection_up(store),
            Action::MoveSpanDown => self.move_span_selection_down(store),
            Action::MoveSpanUp => self.move_span_selection_up(store),
        }
    }

    pub fn selected_trace_id_text(&self) -> &str {
        self.selected_trace_id.as_deref().unwrap_or("<none>")
    }

    fn refresh_current_screen(&mut self, store: &Store) {
        match self.screen {
            Screen::TraceList => self.refresh_trace_list(store),
            Screen::TraceDetail => self.refresh_selected_trace(store),
            Screen::Aggregates => self.refresh_aggregates(store),
            Screen::AggregateSpans => self.refresh_aggregate_spans(store),
        }
    }

    fn refresh_trace_list(&mut self, store: &Store) {
        self.trace_list = store.recent_traces(self.trace_list_query.clone());
        self.clamp_trace_selection();
        self.selected_trace_id = self
            .trace_list
            .rows
            .get(self.selected_trace_index)
            .map(|row| row.trace_id.clone());
    }

    fn refresh_selected_trace(&mut self, store: &Store) {
        let Some(trace_id) = self.selected_trace_id.as_ref() else {
            self.selected_trace = None;
            return;
        };

        let selected_span_id = self
            .selected_trace
            .as_ref()
            .and_then(|trace| trace.rows.get(self.selected_span_index))
            .map(|row| &row.span_id);

        self.selected_trace = store.trace_detail(trace_id, selected_span_id);
        self.clamp_span_selection();
    }

    fn refresh_aggregates(&mut self, store: &Store) {
        self.aggregate = store.aggregate(self.aggregate_query.clone());
        self.clamp_aggregate_selection();
    }

    fn refresh_aggregate_spans(&mut self, store: &Store) {
        let Some(query) = self.aggregate_spans_query.clone() else {
            self.aggregate_spans = AggregateSpansSnapshot::default();
            self.selected_aggregate_span_index = 0;
            return;
        };

        self.aggregate_spans = store.aggregate_spans(query);
        self.clamp_aggregate_span_selection();
    }

    fn open_selected_trace(&mut self, store: &Store) {
        self.selected_trace_id = self
            .trace_list
            .rows
            .get(self.selected_trace_index)
            .map(|row| row.trace_id.clone());
        self.selected_span_index = 0;
        self.screen = Screen::TraceDetail;
        self.refresh_selected_trace(store);
    }

    fn open_selected_aggregate_row(&mut self, store: &Store) {
        let Some(row) = self.aggregate.rows.get(self.selected_aggregate_index) else {
            return;
        };

        self.aggregate_spans_query = Some(AggregateSpansQuery {
            span_name: row.span_name.clone(),
            group_by_attribute: self.aggregate_query.group_by_attribute.clone(),
            group: row.group.clone(),
        });
        self.selected_aggregate_span_index = 0;
        self.screen = Screen::AggregateSpans;
        self.refresh_aggregate_spans(store);
    }

    fn open_selected_aggregate_span_trace(&mut self, store: &Store) {
        let Some(row) = self
            .aggregate_spans
            .rows
            .get(self.selected_aggregate_span_index)
        else {
            return;
        };

        self.open_trace_span(store, row.trace_id.clone(), row.span_id.clone());
    }

    fn open_trace_span(&mut self, store: &Store, trace_id: TraceId, span_id: SpanId) {
        self.selected_trace_id = Some(trace_id.clone());
        self.selected_trace = store.trace_detail(&trace_id, Some(&span_id));
        self.selected_span_index = self
            .selected_trace
            .as_ref()
            .and_then(|trace| trace.rows.iter().position(|row| row.span_id == span_id))
            .unwrap_or(0);
        self.screen = Screen::TraceDetail;
        self.clamp_span_selection();
    }

    fn move_trace_selection_down(&mut self, store: &Store) {
        match self.screen {
            Screen::TraceList => {
                if self.selected_trace_index + 1 < self.trace_list.rows.len() {
                    self.selected_trace_index += 1;
                    self.selected_trace_id = self
                        .trace_list
                        .rows
                        .get(self.selected_trace_index)
                        .map(|row| row.trace_id.clone());
                } else {
                    self.refresh_trace_list(store);
                }
            }
            Screen::Aggregates => {
                if self.selected_aggregate_index + 1 < self.aggregate.rows.len() {
                    self.selected_aggregate_index += 1;
                }
            }
            Screen::AggregateSpans => {
                if self.selected_aggregate_span_index + 1 < self.aggregate_spans.rows.len() {
                    self.selected_aggregate_span_index += 1;
                }
            }
            Screen::TraceDetail => {}
        }
    }

    fn move_trace_selection_up(&mut self, _store: &Store) {
        match self.screen {
            Screen::TraceList if self.selected_trace_index > 0 => {
                self.selected_trace_index -= 1;
                self.selected_trace_id = self
                    .trace_list
                    .rows
                    .get(self.selected_trace_index)
                    .map(|row| row.trace_id.clone());
            }
            Screen::Aggregates if self.selected_aggregate_index > 0 => {
                self.selected_aggregate_index -= 1;
            }
            Screen::AggregateSpans if self.selected_aggregate_span_index > 0 => {
                self.selected_aggregate_span_index -= 1;
            }
            _ => {}
        }
    }

    fn move_span_selection_down(&mut self, _store: &Store) {
        if self.screen != Screen::TraceDetail {
            return;
        }
        let Some(trace) = self.selected_trace.as_ref() else {
            return;
        };
        if self.selected_span_index + 1 < trace.rows.len() {
            self.selected_span_index += 1;
        }
    }

    fn move_span_selection_up(&mut self, _store: &Store) {
        if self.screen == Screen::TraceDetail && self.selected_span_index > 0 {
            self.selected_span_index -= 1;
        }
    }

    fn clamp_trace_selection(&mut self) {
        if self.trace_list.rows.is_empty() {
            self.selected_trace_index = 0;
            return;
        }
        if self.selected_trace_index >= self.trace_list.rows.len() {
            self.selected_trace_index = self.trace_list.rows.len() - 1;
        }
    }

    fn clamp_span_selection(&mut self) {
        let Some(trace) = self.selected_trace.as_ref() else {
            self.selected_span_index = 0;
            return;
        };
        if trace.rows.is_empty() {
            self.selected_span_index = 0;
        } else if self.selected_span_index >= trace.rows.len() {
            self.selected_span_index = trace.rows.len() - 1;
        }
    }

    fn clamp_aggregate_selection(&mut self) {
        if self.aggregate.rows.is_empty() {
            self.selected_aggregate_index = 0;
        } else if self.selected_aggregate_index >= self.aggregate.rows.len() {
            self.selected_aggregate_index = self.aggregate.rows.len() - 1;
        }
    }

    fn clamp_aggregate_span_selection(&mut self) {
        if self.aggregate_spans.rows.is_empty() {
            self.selected_aggregate_span_index = 0;
        } else if self.selected_aggregate_span_index >= self.aggregate_spans.rows.len() {
            self.selected_aggregate_span_index = self.aggregate_spans.rows.len() - 1;
        }
    }
}
