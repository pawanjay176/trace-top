use crate::core::{
    store::{
        AggregateQuery, AggregateSnapshot, Store, StoreEvent, TraceDetailSnapshot, TraceListQuery,
        TraceListSnapshot,
    },
    types::TraceId,
};

const DEFAULT_TRACE_LIST_LIMIT: usize = 250;

#[derive(Clone, Debug)]
pub enum Action {
    Noop,
    Quit,
    Resize(u16, u16),
    StoreChanged(StoreEvent),
    ShowTraceList,
    ShowTraceDetail,
    ShowAggregates,
    MoveSelectionDown,
    MoveSelectionUp,
    MoveSpanDown,
    MoveSpanUp,
    ClearTraceSearch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    TraceList,
    TraceDetail,
    Aggregates,
}

#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    pub should_quit: bool,
    pub terminal_size: Option<(u16, u16)>,
    pub store_version_seen: u64,
    pub trace_list_query: TraceListQuery,
    pub trace_list: TraceListSnapshot,
    pub selected_trace_index: usize,
    pub selected_trace_id: Option<TraceId>,
    pub selected_trace: Option<TraceDetailSnapshot>,
    pub selected_span_index: usize,
    pub aggregate_query: AggregateQuery,
    pub aggregate: AggregateSnapshot,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            screen: Screen::TraceList,
            should_quit: false,
            terminal_size: None,
            store_version_seen: 0,
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
        }
    }

    pub fn update(&mut self, action: Action, store: &Store) {
        match action {
            Action::Noop => {}
            Action::Quit => self.should_quit = true,
            Action::Resize(width, height) => self.terminal_size = Some((width, height)),
            Action::StoreChanged(StoreEvent::Updated { store_version }) => {
                self.store_version_seen = store_version;
                self.refresh_current_screen(store);
            }
            Action::ShowTraceList => {
                self.screen = Screen::TraceList;
                self.refresh_trace_list(store);
            }
            Action::ShowTraceDetail => self.open_selected_trace(store),
            Action::ShowAggregates => {
                self.screen = Screen::Aggregates;
                self.refresh_aggregates(store);
            }
            Action::MoveSelectionDown => self.move_trace_selection_down(store),
            Action::MoveSelectionUp => self.move_trace_selection_up(store),
            Action::MoveSpanDown => self.move_span_selection_down(store),
            Action::MoveSpanUp => self.move_span_selection_up(store),
            Action::ClearTraceSearch => {
                self.trace_list_query.search = None;
                self.selected_trace_index = 0;
                self.refresh_trace_list(store);
            }
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

    fn move_trace_selection_down(&mut self, store: &Store) {
        if self.screen != Screen::TraceList {
            return;
        }
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

    fn move_trace_selection_up(&mut self, _store: &Store) {
        if self.screen == Screen::TraceList && self.selected_trace_index > 0 {
            self.selected_trace_index -= 1;
            self.selected_trace_id = self
                .trace_list
                .rows
                .get(self.selected_trace_index)
                .map(|row| row.trace_id.clone());
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
}
