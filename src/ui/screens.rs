use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use uuid::Uuid;

use crate::csv::{export_state_csv, import_state_csv};
use crate::currencies::CURRENCY_CODES;
use crate::state::{
    AppState, Expense, ExpenseIcon, NewExpenseInput, NewSettlementInput, Settlement, Share,
    equal_split, format_cents, normalize_currency, parse_money_to_cents,
};
use crate::storage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Participants,
    AddExpense,
    AddSettlement,
    History,
    ImportExport,
    Settings,
}

impl Screen {
    pub const ALL: [Screen; 7] = [
        Screen::Dashboard,
        Screen::Participants,
        Screen::AddExpense,
        Screen::AddSettlement,
        Screen::History,
        Screen::ImportExport,
        Screen::Settings,
    ];

    pub fn short_label(self) -> &'static str {
        match self {
            Screen::Dashboard => "Home",
            Screen::Participants => "People",
            Screen::AddExpense => "Expense",
            Screen::AddSettlement => "Settle",
            Screen::History => "History",
            Screen::ImportExport => "CSV",
            Screen::Settings => "Settings",
        }
    }
}

#[component]
pub fn AppLayout(
    mut state: Signal<AppState>,
    mut screen: Signal<Screen>,
    mut feedback: Signal<Option<String>>,
) -> Element {
    let state_snapshot = state();
    let mut expense_editor = use_signal(|| ExpenseEditorState::create(&state_snapshot));
    let mut settlement_editor = use_signal(|| SettlementEditorState::create(&state_snapshot));

    rsx! {
        main { class: "app-shell",
            header { class: "app-header",
                div {
                    h1 { "SplitMoney Lite" }
                    p { "Browser-only shared expense tracker" }
                }
                div { class: "header-stats",
                    span { "Participants: {state_snapshot.participants.len()}" }
                    span { "Expenses: {state_snapshot.expenses.len()}" }
                    span { "Settlements: {state_snapshot.settlements.len()}" }
                }
            }

            if let Some(message) = feedback() {
                div { class: "feedback-banner",
                    span { "{message}" }
                    button {
                        class: "link-btn",
                        onclick: move |_| feedback.set(None),
                        "Dismiss"
                    }
                }
            }

            section { class: "screen-content",
                {
                    match screen() {
                        Screen::Dashboard => rsx! {
                            DashboardScreen { state }
                        },
                        Screen::Participants => rsx! {
                            ParticipantsScreen { state, feedback }
                        },
                        Screen::AddExpense => rsx! {
                            AddExpenseScreen { state, feedback, expense_editor }
                        },
                        Screen::AddSettlement => rsx! {
                            AddSettlementScreen { state, feedback, settlement_editor }
                        },
                        Screen::History => rsx! {
                            HistoryScreen { state, screen, feedback, expense_editor, settlement_editor }
                        },
                        Screen::ImportExport => rsx! {
                            ImportExportScreen { state, feedback }
                        },
                        Screen::Settings => rsx! {
                            SettingsScreen { state, feedback }
                        },
                    }
                }
            }

            nav { class: "bottom-nav",
                for nav_screen in Screen::ALL {
                    button {
                        class: if nav_screen == screen() { "nav-btn active" } else { "nav-btn" },
                        onclick: move |_| {
                            if nav_screen == Screen::AddExpense {
                                expense_editor.set(ExpenseEditorState::create(&state()));
                            }
                            if nav_screen == Screen::AddSettlement {
                                settlement_editor.set(SettlementEditorState::create(&state()));
                            }
                            screen.set(nav_screen);
                        },
                        "{nav_screen.short_label()}"
                    }
                }
            }
        }
    }
}

#[component]
fn DashboardScreen(state: Signal<AppState>) -> Element {
    let snapshot = state();
    let balance_groups = snapshot.compute_balances_by_currency();
    let suggestions = snapshot.settlement_suggestions_by_currency();

    rsx! {
        div { class: "panel",
            h2 { "Balances" }
            if balance_groups.is_empty() {
                p { class: "muted", "Add participants first to get started." }
            }
            for group in balance_groups {
                div { class: "history-item",
                    p { class: "meta", "Currency: {group.currency}" }
                    for balance in group.balances {
                        div { class: "balance-row",
                            div { class: "row-left",
                                span { class: if balance.participant.is_active { "dot active" } else { "dot" } }
                                strong { "{balance.participant.name}" }
                            }
                            span {
                                class: if balance.net_cents >= 0 { "money positive" } else { "money negative" },
                                {format_signed_money(balance.net_cents, &group.currency)}
                            }
                        }
                    }
                }
            }
        }

        div { class: "panel",
            h2 { "Suggested Settlements" }
            if suggestions.is_empty() {
                p { class: "muted", "No transfer suggestions yet." }
            }
            for suggestion in suggestions {
                p {
                    class: "suggestion",
                    {format!(
                        "{} pays {} {}",
                        participant_name(&snapshot, suggestion.from_id),
                        participant_name(&snapshot, suggestion.to_id),
                        format_money(suggestion.amount_cents, &suggestion.currency)
                    )}
                }
            }
        }
    }
}

#[component]
fn ParticipantsScreen(
    mut state: Signal<AppState>,
    mut feedback: Signal<Option<String>>,
) -> Element {
    let snapshot = state();
    let mut new_name = use_signal(String::new);
    let mut rename_cache = use_signal(HashMap::<Uuid, String>::new);

    rsx! {
        div { class: "panel",
            h2 { "Participants" }
            form {
                class: "row-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    match state.with_mut(|store| store.add_participant(new_name().clone())) {
                        Ok(_) => {
                            persist_state(state);
                            feedback.set(Some("Participant added.".to_string()));
                            new_name.set(String::new());
                        }
                        Err(err) => feedback.set(Some(err)),
                    }
                },
                input {
                    value: new_name(),
                    placeholder: "Add participant name",
                    oninput: move |event| new_name.set(event.value()),
                }
                button { class: "primary", r#type: "submit", "Add" }
            }
        }

        div { class: "panel",
            if snapshot.participants.is_empty() {
                p { class: "muted", "No participants yet." }
            }
            for participant in snapshot.participants {
                {
                    let participant_id = participant.id;
                    let participant_name = participant.name.clone();
                    let participant_name_for_blur = participant_name.clone();
                    let participant_name_for_enter = participant_name.clone();
                    let is_active = participant.is_active;
                    rsx! {
                        div { class: "participant-row",
                            input {
                                value: rename_cache().get(&participant_id).cloned().unwrap_or_else(|| participant_name.clone()),
                                oninput: move |event| {
                                    rename_cache.with_mut(|cache| {
                                        cache.insert(participant_id, event.value());
                                    });
                                },
                                onblur: move |_| {
                                    commit_participant_rename(
                                        state,
                                        feedback,
                                        rename_cache,
                                        participant_id,
                                        participant_name_for_blur.clone(),
                                    );
                                },
                                onkeydown: move |event| {
                                    if event.key() == Key::Enter {
                                        commit_participant_rename(
                                            state,
                                            feedback,
                                            rename_cache,
                                            participant_id,
                                            participant_name_for_enter.clone(),
                                        );
                                    }
                                },
                            }
                            button {
                                class: if is_active { "warn" } else { "secondary" },
                                onclick: move |_| {
                                    let next_active = !is_active;
                                    let used = state().participant_is_used(participant_id);
                                    if used && !next_active {
                                        feedback.set(Some("Participant has records and is now marked inactive instead of removed.".to_string()));
                                    }

                                    match state.with_mut(|store| store.set_participant_active(participant_id, next_active)) {
                                        Ok(_) => {
                                            persist_state(state);
                                        }
                                        Err(err) => feedback.set(Some(err)),
                                    }
                                },
                                if is_active { "Deactivate" } else { "Activate" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ShareDraft {
    participant_id: Uuid,
    selected: bool,
    amount_input: String,
}

#[derive(Clone, Debug, PartialEq)]
struct ExpenseDraft {
    title: String,
    total_input: String,
    currency: String,
    payer_id: Option<Uuid>,
    icon: ExpenseIcon,
    one_owes_id: Option<Uuid>,
    shares: Vec<ShareDraft>,
}

impl ExpenseDraft {
    fn from_state(state: &AppState) -> Self {
        let mut share_drafts = Vec::new();
        let mut payer = None;

        for participant in &state.participants {
            if participant.is_active {
                if payer.is_none() {
                    payer = Some(participant.id);
                }
                share_drafts.push(ShareDraft {
                    participant_id: participant.id,
                    selected: true,
                    amount_input: "0.00".to_string(),
                });
            }
        }
        let one_owes_id = share_drafts
            .iter()
            .map(|share| share.participant_id)
            .find(|participant_id| Some(*participant_id) != payer);

        Self {
            title: String::new(),
            total_input: String::new(),
            currency: state.last_currency.clone(),
            payer_id: payer,
            icon: ExpenseIcon::Other,
            one_owes_id,
            shares: share_drafts,
        }
    }

    fn from_expense(state: &AppState, expense: &Expense) -> Self {
        let mut include_ids = HashSet::<Uuid>::new();
        include_ids.insert(expense.payer_id);
        for share in &expense.shares {
            include_ids.insert(share.participant_id);
        }

        let mut share_map = HashMap::<Uuid, i64>::new();
        for share in &expense.shares {
            share_map.insert(share.participant_id, share.amount_cents);
        }

        let mut share_drafts = Vec::new();
        for participant in &state.participants {
            if participant.is_active || include_ids.contains(&participant.id) {
                let share_amount = share_map.get(&participant.id).copied().unwrap_or(0);
                share_drafts.push(ShareDraft {
                    participant_id: participant.id,
                    selected: include_ids.contains(&participant.id),
                    amount_input: format_cents(share_amount),
                });
            }
        }
        let one_owes_id = expense
            .shares
            .iter()
            .map(|share| share.participant_id)
            .find(|participant_id| *participant_id != expense.payer_id)
            .or_else(|| {
                share_drafts
                    .iter()
                    .map(|share| share.participant_id)
                    .find(|participant_id| *participant_id != expense.payer_id)
            });

        Self {
            title: expense.title.clone(),
            total_input: format_cents(expense.total_cents),
            currency: expense.currency.clone(),
            payer_id: Some(expense.payer_id),
            icon: expense.icon,
            one_owes_id,
            shares: share_drafts,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ExpenseEditorState {
    editing_id: Option<Uuid>,
    draft: ExpenseDraft,
}

impl ExpenseEditorState {
    fn create(state: &AppState) -> Self {
        Self {
            editing_id: None,
            draft: ExpenseDraft::from_state(state),
        }
    }

    fn edit(state: &AppState, expense: &Expense) -> Self {
        Self {
            editing_id: Some(expense.id),
            draft: ExpenseDraft::from_expense(state, expense),
        }
    }
}

#[component]
fn AddExpenseScreen(
    mut state: Signal<AppState>,
    mut feedback: Signal<Option<String>>,
    mut expense_editor: Signal<ExpenseEditorState>,
) -> Element {
    let snapshot = state();
    let is_edit_mode = expense_editor().editing_id.is_some();
    let mut referenced_ids = HashSet::<Uuid>::new();
    if let Some(payer_id) = expense_editor().draft.payer_id {
        referenced_ids.insert(payer_id);
    }
    for share in &expense_editor().draft.shares {
        referenced_ids.insert(share.participant_id);
    }

    let selectable_participants = snapshot
        .participants
        .iter()
        .filter(|participant| participant.is_active || referenced_ids.contains(&participant.id))
        .cloned()
        .collect::<Vec<_>>();
    let payer_id = expense_editor().draft.payer_id;
    let one_owes_candidates = selectable_participants
        .iter()
        .filter(|participant| Some(participant.id) != payer_id)
        .cloned()
        .collect::<Vec<_>>();
    let one_owes_value = expense_editor()
        .draft
        .one_owes_id
        .filter(|id| {
            one_owes_candidates
                .iter()
                .any(|participant| participant.id == *id)
        })
        .map(|id| id.to_string())
        .unwrap_or_default();
    let can_select_one_owes = !one_owes_candidates.is_empty();

    rsx! {
        div { class: "panel",
            h2 { if is_edit_mode { "Edit Expense" } else { "Add Expense" } }
            if selectable_participants.is_empty() {
                p { class: "muted", "Add active participants before creating expenses." }
            } else {
                form {
                    class: "expense-form",
                    onsubmit: move |event| {
                        event.prevent_default();

                        let editor = expense_editor();
                        let current = editor.draft;
                        let total_cents = match parse_money_to_cents(&current.total_input) {
                            Ok(value) => value,
                            Err(err) => {
                                feedback.set(Some(err));
                                return;
                            }
                        };

                        let mut shares = Vec::new();
                        for share in &current.shares {
                            if !share.selected {
                                continue;
                            }

                            let share_cents = if share.amount_input.trim().is_empty() {
                                0
                            } else {
                                match parse_money_to_cents(&share.amount_input) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        feedback.set(Some(format!(
                                            "Share for {} is invalid: {err}",
                                            participant_name(&state(), share.participant_id)
                                        )));
                                        return;
                                    }
                                }
                            };

                            shares.push(Share {
                                participant_id: share.participant_id,
                                amount_cents: share_cents,
                            });
                        }

                        if shares.is_empty() {
                            feedback.set(Some("Select at least one participant share.".to_string()));
                            return;
                        }

                        let Some(payer_id) = current.payer_id else {
                            feedback.set(Some("Select a payer participant.".to_string()));
                            return;
                        };

                        let payload = NewExpenseInput {
                            title: current.title,
                            icon: current.icon,
                            payer_id,
                            total_cents,
                            currency: normalize_currency(&current.currency),
                            created_at: None,
                            shares,
                        };

                        let result = match editor.editing_id {
                            Some(expense_id) => {
                                state.with_mut(|store| store.update_expense(expense_id, payload))
                            }
                            None => state.with_mut(|store| store.add_expense(payload).map(|_| ())),
                        };

                        match result {
                            Ok(_) => {
                                persist_state(state);
                                feedback.set(Some(if editor.editing_id.is_some() {
                                    "Expense updated.".to_string()
                                } else {
                                    "Expense added.".to_string()
                                }));
                                expense_editor.set(ExpenseEditorState::create(&state()));
                            }
                            Err(err) => feedback.set(Some(err)),
                        }
                    },
                    label { "Title (optional)" }
                    input {
                        value: expense_editor().draft.title,
                        placeholder: "Dinner, Taxi, Rent...",
                        oninput: move |event| {
                            expense_editor.with_mut(|editor| editor.draft.title = event.value());
                        },
                    }

                    label { "Amount" }
                    div { class: "amount-row",
                        input {
                            class: "currency-code-input",
                            value: expense_editor().draft.currency,
                            list: "currency-codes",
                            placeholder: "USD",
                            oninput: move |event| {
                                expense_editor.with_mut(|editor| editor.draft.currency = event.value());
                            },
                            onblur: move |_| {
                                let next_currency = normalize_currency(&expense_editor().draft.currency);
                                expense_editor.with_mut(|editor| editor.draft.currency = next_currency.clone());
                                state.with_mut(|store| store.last_currency = next_currency);
                                persist_state(state);
                            },
                        }
                        input {
                            class: "amount-input",
                            value: expense_editor().draft.total_input,
                            placeholder: "0.00",
                            inputmode: "decimal",
                            oninput: move |event| {
                                expense_editor.with_mut(|editor| editor.draft.total_input = event.value());
                            },
                        }
                    }
                    datalist { id: "currency-codes",
                        for code in CURRENCY_CODES {
                            option { value: *code }
                        }
                    }
                    p { class: "meta", "Choose an ISO currency code or type your own code." }

                    div {
                        label { "Payer" }
                        select {
                            value: expense_editor().draft.payer_id.map(|id| id.to_string()).unwrap_or_default(),
                            onchange: move |event| {
                                if let Ok(id) = Uuid::parse_str(&event.value()) {
                                    expense_editor.with_mut(|editor| {
                                        editor.draft.payer_id = Some(id);
                                        let has_valid_one_owes = editor
                                            .draft
                                            .one_owes_id
                                            .is_some_and(|one_owes_id| {
                                                one_owes_id != id
                                                    && editor
                                                        .draft
                                                        .shares
                                                        .iter()
                                                        .any(|share| share.participant_id == one_owes_id)
                                            });
                                        if !has_valid_one_owes {
                                            editor.draft.one_owes_id = editor
                                                .draft
                                                .shares
                                                .iter()
                                                .map(|share| share.participant_id)
                                                .find(|participant_id| *participant_id != id);
                                        }
                                    });
                                }
                            },
                            for participant in &selectable_participants {
                                option { value: participant.id.to_string(), "{participant.name}" }
                            }
                        }
                    }

                    label { "Type" }
                    div { class: "icon-grid",
                        for icon in ExpenseIcon::ALL {
                            button {
                                r#type: "button",
                                class: if icon == expense_editor().draft.icon { "icon-choice active" } else { "icon-choice" },
                                onclick: move |_| {
                                    expense_editor.with_mut(|editor| editor.draft.icon = icon);
                                },
                                IconBadge { icon }
                                span { "{icon.label()}" }
                            }
                        }
                    }

                    div { class: "split-tools",
                        h3 { "Split" }
                        button {
                            r#type: "button",
                            class: "secondary",
                            onclick: move |_| {
                                let total_cents = match parse_money_to_cents(&expense_editor().draft.total_input) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        feedback.set(Some(err));
                                        return;
                                    }
                                };

                                let selected_ids = expense_editor()
                                    .draft
                                    .shares
                                    .iter()
                                    .filter(|share| share.selected)
                                    .map(|share| share.participant_id)
                                    .collect::<Vec<_>>();

                                let split = equal_split(total_cents, &selected_ids);
                                let mut split_map = HashMap::<Uuid, i64>::new();
                                for share in split {
                                    split_map.insert(share.participant_id, share.amount_cents);
                                }

                                expense_editor.with_mut(|editor| {
                                    for share in &mut editor.draft.shares {
                                        if let Some(amount) = split_map.get(&share.participant_id) {
                                            share.amount_input = format_cents(*amount);
                                        }
                                    }
                                });
                            },
                            "Split equally (selected)"
                        }

                        div { class: "one-owes-row",
                            select {
                                value: one_owes_value,
                                disabled: !can_select_one_owes,
                                onchange: move |event| {
                                    if let Ok(id) = Uuid::parse_str(&event.value()) {
                                        expense_editor.with_mut(|editor| editor.draft.one_owes_id = Some(id));
                                    }
                                },
                                for participant in &one_owes_candidates {
                                    option { value: participant.id.to_string(), "{participant.name}" }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "secondary",
                                disabled: !can_select_one_owes,
                                onclick: move |_| {
                                    let total_cents = match parse_money_to_cents(&expense_editor().draft.total_input) {
                                        Ok(value) => value,
                                        Err(err) => {
                                            feedback.set(Some(err));
                                            return;
                                        }
                                    };

                                    let Some(payer_id) = expense_editor().draft.payer_id else {
                                        feedback.set(Some("Select a payer participant.".to_string()));
                                        return;
                                    };

                                    let Some(one_owes) = expense_editor().draft.one_owes_id else {
                                        feedback.set(Some("Select who owes all.".to_string()));
                                        return;
                                    };

                                    if one_owes == payer_id {
                                        feedback.set(Some("Who owes all must be different from the payer.".to_string()));
                                        return;
                                    }

                                    expense_editor.with_mut(|editor| {
                                        for share in &mut editor.draft.shares {
                                            share.selected = share.participant_id == one_owes;
                                            share.amount_input = if share.participant_id == one_owes {
                                                format_cents(total_cents)
                                            } else {
                                                "0.00".to_string()
                                            };
                                        }
                                    });
                                },
                                "One person owes all"
                            }
                        }
                    }

                    div { class: "share-list",
                        for share in expense_editor().draft.shares {
                            div { class: "share-row",
                                button {
                                    r#type: "button",
                                    class: if share.selected { "tag active" } else { "tag" },
                                    onclick: move |_| {
                                        expense_editor.with_mut(|editor| {
                                            if let Some(target) = editor
                                                .draft
                                                .shares
                                                .iter_mut()
                                                .find(|item| item.participant_id == share.participant_id)
                                            {
                                                target.selected = !target.selected;
                                            }
                                        });
                                    },
                                    if share.selected { "Included" } else { "Excluded" }
                                }
                                span { "{participant_name(&state(), share.participant_id)}" }
                                input {
                                    value: share.amount_input,
                                    inputmode: "decimal",
                                    oninput: move |event| {
                                        expense_editor.with_mut(|editor| {
                                            if let Some(target) = editor
                                                .draft
                                                .shares
                                                .iter_mut()
                                                .find(|item| item.participant_id == share.participant_id)
                                            {
                                                target.amount_input = event.value();
                                            }
                                        });
                                    },
                                }
                            }
                        }
                    }

                    div { class: "history-actions",
                        button { class: "primary", r#type: "submit", if is_edit_mode { "Update Expense" } else { "Save Expense" } }
                        if is_edit_mode {
                            button {
                                class: "secondary",
                                r#type: "button",
                                onclick: move |_| {
                                    expense_editor.set(ExpenseEditorState::create(&state()));
                                },
                                "Back to Add"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SettlementDraft {
    from_id: Option<Uuid>,
    to_id: Option<Uuid>,
    amount_input: String,
    currency: String,
    note: String,
}

impl SettlementDraft {
    fn from_state(state: &AppState) -> Self {
        let mut ids = state
            .participants
            .iter()
            .filter(|participant| participant.is_active)
            .map(|participant| participant.id)
            .collect::<Vec<_>>();

        ids.truncate(2);

        Self {
            from_id: ids.first().copied(),
            to_id: ids.get(1).copied().or_else(|| ids.first().copied()),
            amount_input: String::new(),
            currency: state.last_currency.clone(),
            note: String::new(),
        }
    }

    fn from_settlement(settlement: &Settlement) -> Self {
        Self {
            from_id: Some(settlement.from_id),
            to_id: Some(settlement.to_id),
            amount_input: format_cents(settlement.amount_cents),
            currency: settlement.currency.clone(),
            note: settlement.note.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SettlementEditorState {
    editing_id: Option<Uuid>,
    draft: SettlementDraft,
}

impl SettlementEditorState {
    fn create(state: &AppState) -> Self {
        Self {
            editing_id: None,
            draft: SettlementDraft::from_state(state),
        }
    }

    fn edit(settlement: &Settlement) -> Self {
        Self {
            editing_id: Some(settlement.id),
            draft: SettlementDraft::from_settlement(settlement),
        }
    }
}

#[component]
fn AddSettlementScreen(
    mut state: Signal<AppState>,
    mut feedback: Signal<Option<String>>,
    mut settlement_editor: Signal<SettlementEditorState>,
) -> Element {
    let snapshot = state();
    let is_edit_mode = settlement_editor().editing_id.is_some();
    let mut include_ids = HashSet::<Uuid>::new();
    if let Some(from_id) = settlement_editor().draft.from_id {
        include_ids.insert(from_id);
    }
    if let Some(to_id) = settlement_editor().draft.to_id {
        include_ids.insert(to_id);
    }

    let participants = snapshot
        .participants
        .iter()
        .filter(|participant| participant.is_active || include_ids.contains(&participant.id))
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        div { class: "panel",
            h2 { if is_edit_mode { "Edit Settlement" } else { "Add Settlement" } }
            if participants.len() < 2 {
                p { class: "muted", "At least two active participants are needed." }
            } else {
                form {
                    class: "expense-form",
                    onsubmit: move |event| {
                        event.prevent_default();

                        let editor = settlement_editor();
                        let current = editor.draft;
                        let Some(from_id) = current.from_id else {
                            feedback.set(Some("Select who paid.".to_string()));
                            return;
                        };
                        let Some(to_id) = current.to_id else {
                            feedback.set(Some("Select receiver participant.".to_string()));
                            return;
                        };

                        let amount_cents = match parse_money_to_cents(&current.amount_input) {
                            Ok(value) => value,
                            Err(err) => {
                                feedback.set(Some(err));
                                return;
                            }
                        };

                        let payload = NewSettlementInput {
                            from_id,
                            to_id,
                            amount_cents,
                            currency: normalize_currency(&current.currency),
                            created_at: None,
                            note: current.note,
                        };

                        let result = match editor.editing_id {
                            Some(settlement_id) => {
                                state.with_mut(|store| store.update_settlement(settlement_id, payload))
                            }
                            None => state.with_mut(|store| store.add_settlement(payload).map(|_| ())),
                        };

                        match result {
                            Ok(_) => {
                                persist_state(state);
                                feedback.set(Some(if editor.editing_id.is_some() {
                                    "Settlement updated.".to_string()
                                } else {
                                    "Settlement saved.".to_string()
                                }));
                                settlement_editor.set(SettlementEditorState::create(&state()));
                            }
                            Err(err) => feedback.set(Some(err)),
                        }
                    },
                    label { "From (debtor)" }
                    select {
                        value: settlement_editor().draft.from_id.map(|id| id.to_string()).unwrap_or_default(),
                        onchange: move |event| {
                            if let Ok(id) = Uuid::parse_str(&event.value()) {
                                let next_to_id =
                                    participants.iter().find(|participant| participant.id != id).map(|participant| participant.id);
                                settlement_editor.with_mut(|editor| {
                                    editor.draft.from_id = Some(id);
                                    editor.draft.to_id = next_to_id;
                                });
                            }
                        },
                        for participant in &participants {
                            option { value: participant.id.to_string(), "{participant.name}" }
                        }
                    }

                    label { "To (creditor)" }
                    select {
                        value: settlement_editor().draft.to_id.map(|id| id.to_string()).unwrap_or_default(),
                        onchange: move |event| {
                            if let Ok(id) = Uuid::parse_str(&event.value()) {
                                settlement_editor.with_mut(|editor| editor.draft.to_id = Some(id));
                            }
                        },
                        for participant in participants.iter().filter(|participant| {
                            Some(participant.id) != settlement_editor().draft.from_id
                        }) {
                            option { value: participant.id.to_string(), "{participant.name}" }
                        }
                    }

                    label { "Amount" }
                    input {
                        value: settlement_editor().draft.amount_input,
                        inputmode: "decimal",
                        placeholder: "0.00",
                        oninput: move |event| {
                            settlement_editor.with_mut(|editor| editor.draft.amount_input = event.value());
                        },
                    }

                    label { "Currency" }
                    input {
                        value: settlement_editor().draft.currency,
                        list: "currency-codes",
                        placeholder: "USD or custom code",
                        oninput: move |event| {
                            settlement_editor.with_mut(|editor| editor.draft.currency = event.value());
                        },
                        onblur: move |_| {
                            let next_currency = normalize_currency(&settlement_editor().draft.currency);
                            settlement_editor.with_mut(|editor| editor.draft.currency = next_currency.clone());
                            state.with_mut(|store| store.last_currency = next_currency);
                            persist_state(state);
                        },
                    }

                    label { "Note" }
                    input {
                        value: settlement_editor().draft.note,
                        placeholder: "Optional",
                        oninput: move |event| {
                            settlement_editor.with_mut(|editor| editor.draft.note = event.value());
                        },
                    }

                    div { class: "history-actions",
                        button { class: "primary", r#type: "submit", if is_edit_mode { "Update Settlement" } else { "Save Settlement" } }
                        if is_edit_mode {
                            button {
                                class: "secondary",
                                r#type: "button",
                                onclick: move |_| {
                                    settlement_editor.set(SettlementEditorState::create(&state()));
                                },
                                "Back to Add"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum HistoryItem {
    Expense(Expense),
    Settlement(Settlement),
}

impl HistoryItem {
    fn timestamp(&self) -> i64 {
        match self {
            HistoryItem::Expense(expense) => expense.created_at,
            HistoryItem::Settlement(settlement) => settlement.created_at,
        }
    }
}

#[component]
fn HistoryScreen(
    mut state: Signal<AppState>,
    mut screen: Signal<Screen>,
    mut feedback: Signal<Option<String>>,
    mut expense_editor: Signal<ExpenseEditorState>,
    mut settlement_editor: Signal<SettlementEditorState>,
) -> Element {
    let snapshot = state();
    let mut query = use_signal(String::new);

    let mut rows = Vec::new();
    for expense in &snapshot.expenses {
        rows.push(HistoryItem::Expense(expense.clone()));
    }

    for settlement in &snapshot.settlements {
        rows.push(HistoryItem::Settlement(settlement.clone()));
    }

    rows.sort_by_key(|item| -item.timestamp());
    let lower_query = query().to_lowercase();

    rsx! {
        div { class: "panel",
            h2 { "History" }
            input {
                value: query(),
                placeholder: "Filter by title, participant, or note",
                oninput: move |event| query.set(event.value()),
            }
        }

        div { class: "panel",
            if rows.is_empty() {
                p { class: "muted", "No records yet." }
            }
            for row in rows {
                if history_matches(&row, &snapshot, &lower_query) {
                    {
                        match row {
                            HistoryItem::Expense(expense) => {
                                let expense_id = expense.id;
                                rsx! {
                                article { class: "history-item",
                                    div { class: "history-top",
                                        div { class: "history-title",
                                            IconBadge { icon: expense.icon }
                                            strong { "{expense_title_for_history(&expense)}" }
                                        }
                                        span { class: "money", "{format_money(expense.total_cents, &expense.currency)}" }
                                    }
                                    p { class: "muted", "Paid by {participant_name(&snapshot, expense.payer_id)}" }
                                    p { class: "meta", "Type: {expense.icon.label()}" }
                                    p { class: "meta", "{format_timestamp(expense.created_at)}" }
                                    p { class: "meta", "Expense ID: {expense_id}" }

                                    div { class: "history-actions",
                                        button {
                                            class: "secondary",
                                            onclick: move |_| {
                                                expense_editor.set(ExpenseEditorState::edit(&state(), &expense));
                                                screen.set(Screen::AddExpense);
                                            },
                                            "Edit"
                                        }
                                        button {
                                            class: "warn",
                                            onclick: move |_| {
                                                if confirm_dialog("Delete this expense? This cannot be undone.") {
                                                    match state.with_mut(|store| store.delete_expense(expense_id)) {
                                                        Ok(_) => {
                                                            persist_state(state);
                                                            feedback.set(Some("Expense deleted.".to_string()));
                                                        }
                                                        Err(err) => feedback.set(Some(err)),
                                                    }
                                                }
                                            },
                                            "Delete"
                                        }
                                    }
                                }
                            }
                            },
                            HistoryItem::Settlement(settlement) => {
                                let settlement_id = settlement.id;
                                rsx! {
                                article { class: "history-item",
                                    div { class: "history-top",
                                        strong { "Settlement" }
                                        span { class: "money", {format_money(settlement.amount_cents, &settlement.currency)} }
                                    }
                                    p { class: "muted", "{participant_name(&snapshot, settlement.from_id)} paid {participant_name(&snapshot, settlement.to_id)}" }
                                    if !settlement.note.is_empty() {
                                        p { class: "meta", "Note: {settlement.note}" }
                                    }
                                    p { class: "meta", "{format_timestamp(settlement.created_at)}" }
                                    p { class: "meta", "Settlement ID: {settlement_id}" }

                                    div { class: "history-actions",
                                        button {
                                            class: "secondary",
                                            onclick: move |_| {
                                                settlement_editor.set(SettlementEditorState::edit(&settlement));
                                                screen.set(Screen::AddSettlement);
                                            },
                                            "Edit"
                                        }
                                        button {
                                            class: "warn",
                                            onclick: move |_| {
                                                if confirm_dialog("Delete this settlement? This cannot be undone.") {
                                                    match state.with_mut(|store| store.delete_settlement(settlement_id)) {
                                                        Ok(_) => {
                                                            persist_state(state);
                                                            feedback.set(Some("Settlement deleted.".to_string()));
                                                        }
                                                        Err(err) => feedback.set(Some(err)),
                                                    }
                                                }
                                            },
                                            "Delete"
                                        }
                                    }
                                }
                            }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ImportExportScreen(
    mut state: Signal<AppState>,
    mut feedback: Signal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "panel",
            h2 { "Import / Export" }
            p { class: "muted", "CSV includes participant, expense, and settlement rows." }

            button {
                class: "primary",
                onclick: move |_| {
                    match export_state_csv(&state()) {
                        Ok(csv_payload) => {
                            if download_csv_file("splitmoney-export.csv", &csv_payload).is_ok() {
                                feedback.set(Some("CSV export started.".to_string()));
                            } else {
                                feedback.set(Some("CSV export failed in the browser.".to_string()));
                            }
                        }
                        Err(err) => feedback.set(Some(err)),
                    }
                },
                "Export CSV"
            }

            label { class: "file-label",
                "Import CSV"
                input {
                    r#type: "file",
                    accept: ".csv,text/csv",
                    onchange: move |event| {
                        let files = event.files();
                        let Some(first_file) = files.first().cloned() else {
                            feedback.set(Some("No file selected.".to_string()));
                            return;
                        };

                        let mut state_signal = state;
                        let mut feedback_signal = feedback;
                        spawn(async move {
                            let payload = match first_file.read_string().await {
                                Ok(contents) => contents,
                                Err(_) => {
                                    feedback_signal
                                        .set(Some("Failed to read selected file.".to_string()));
                                    return;
                                }
                            };

                            match import_state_csv(&payload) {
                                Ok((imported, summary)) => {
                                    state_signal.set(imported.clone());
                                    storage::save_state(&imported);
                                    feedback_signal.set(Some(format!(
                                        "Imported {} participants, {} expenses, {} settlements.",
                                        summary.participants, summary.expenses, summary.settlements
                                    )));
                                }
                                Err(err) => feedback_signal.set(Some(err)),
                            }
                        });
                    },
                }
            }
        }
    }
}

#[component]
fn SettingsScreen(mut state: Signal<AppState>, mut feedback: Signal<Option<String>>) -> Element {
    rsx! {
        div { class: "panel",
            h2 { "Settings" }
            p { class: "muted", "All app data is stored only in this browser's LocalStorage." }

            button {
                class: "warn",
                onclick: move |_| {
                    if confirm_dialog("Reset all data? This cannot be undone.") {
                        state.set(AppState::default());
                        storage::reset_state();
                        feedback.set(Some("All data was reset.".to_string()));
                    }
                },
                "Reset data"
            }
        }
    }
}

#[component]
fn IconBadge(icon: ExpenseIcon) -> Element {
    rsx! {
        svg {
            class: "icon-svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            {
                match icon {
                    ExpenseIcon::Food => rsx! {
                        path { d: "M5 3v8" }
                        path { d: "M9 3v8" }
                        path { d: "M7 3v8" }
                        path { d: "M7 11v10" }
                        path { d: "M14 3c0 5 5 5 5 10v8" }
                    },
                    ExpenseIcon::Transport => rsx! {
                        rect { x: "3", y: "6", width: "18", height: "10", rx: "2" }
                        path { d: "M7 16v3" }
                        path { d: "M17 16v3" }
                        circle { cx: "7", cy: "19", r: "1" }
                        circle { cx: "17", cy: "19", r: "1" }
                    },
                    ExpenseIcon::Home => rsx! {
                        path { d: "M3 11l9-7 9 7" }
                        path { d: "M5 10v10h14V10" }
                        path { d: "M10 20v-6h4v6" }
                    },
                    ExpenseIcon::Drinks => rsx! {
                        path { d: "M5 4h14" }
                        path { d: "M7 4l2 13h6l2-13" }
                        path { d: "M12 17v3" }
                    },
                    ExpenseIcon::Shopping => rsx! {
                        path { d: "M6 7h14l-1.5 8h-11z" }
                        path { d: "M6 7L5 4H3" }
                        circle { cx: "10", cy: "19", r: "1" }
                        circle { cx: "17", cy: "19", r: "1" }
                    },
                    ExpenseIcon::Entertainment => rsx! {
                        circle { cx: "12", cy: "12", r: "8" }
                        path { d: "M10 9l5 3-5 3z" }
                    },
                    ExpenseIcon::Other => rsx! {
                        circle { cx: "12", cy: "12", r: "9" }
                        path { d: "M12 8v4" }
                        path { d: "M12 16h.01" }
                    },
                }
            }
        }
    }
}

fn history_matches(item: &HistoryItem, state: &AppState, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let haystack = match item {
        HistoryItem::Expense(expense) => format!(
            "{} {} {} {}",
            expense.title,
            expense.icon.label(),
            participant_name(state, expense.payer_id),
            expense.currency
        ),
        HistoryItem::Settlement(settlement) => format!(
            "{} {} {} {}",
            participant_name(state, settlement.from_id),
            participant_name(state, settlement.to_id),
            settlement.note,
            settlement.currency
        ),
    };

    haystack.to_lowercase().contains(query)
}

fn expense_title_for_history(expense: &Expense) -> String {
    if expense.title.trim().is_empty() {
        "Untitled expense".to_string()
    } else {
        expense.title.clone()
    }
}

fn persist_state(state: Signal<AppState>) {
    storage::save_state(&state());
}

fn commit_participant_rename(
    mut state: Signal<AppState>,
    mut feedback: Signal<Option<String>>,
    mut rename_cache: Signal<HashMap<Uuid, String>>,
    participant_id: Uuid,
    current_name: String,
) {
    let next_name = rename_cache()
        .get(&participant_id)
        .cloned()
        .unwrap_or_else(|| current_name.clone());
    let trimmed = next_name.trim().to_string();

    if trimmed.is_empty() {
        rename_cache.with_mut(|cache| {
            cache.insert(participant_id, current_name);
        });
        feedback.set(Some("Participant name is required.".to_string()));
        return;
    }

    if trimmed == current_name {
        rename_cache.with_mut(|cache| {
            cache.remove(&participant_id);
        });
        return;
    }

    match state.with_mut(|store| store.rename_participant(participant_id, trimmed)) {
        Ok(_) => {
            persist_state(state);
            rename_cache.with_mut(|cache| {
                cache.remove(&participant_id);
            });
        }
        Err(err) => feedback.set(Some(err)),
    }
}

fn participant_name(state: &AppState, participant_id: Uuid) -> String {
    state
        .participant_by_id(participant_id)
        .map(|participant| participant.name.clone())
        .unwrap_or_else(|| format!("Unknown ({})", &participant_id.to_string()[..8]))
}

fn format_money(cents: i64, currency: &str) -> String {
    format!("{} {}", currency, format_cents(cents))
}

fn format_signed_money(cents: i64, currency: &str) -> String {
    if cents >= 0 {
        format!("+{} {}", currency, format_cents(cents))
    } else {
        format!("-{} {}", currency, format_cents(-cents))
    }
}

fn format_timestamp(ts: i64) -> String {
    if ts == 0 {
        return "Unknown date".to_string();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ts as f64));
        date.to_locale_string("en-US", &wasm_bindgen::JsValue::UNDEFINED)
            .as_string()
            .unwrap_or_else(|| ts.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        ts.to_string()
    }
}

fn confirm_dialog(message: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.confirm_with_message(message).ok())
            .unwrap_or(false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        false
    }
}

fn download_csv_file(file_name: &str, payload: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        let Some(window) = web_sys::window() else {
            return Err("window is not available".to_string());
        };
        let Some(document) = window.document() else {
            return Err("document is not available".to_string());
        };

        let parts = js_sys::Array::new();
        parts.push(&wasm_bindgen::JsValue::from_str(payload));

        let options = web_sys::BlobPropertyBag::new();
        options.set_type("text/csv;charset=utf-8");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
            .map_err(|_| "failed to create Blob".to_string())?;

        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|_| "failed to create object URL".to_string())?;

        let anchor = document
            .create_element("a")
            .map_err(|_| "failed to create anchor element".to_string())?
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .map_err(|_| "failed to cast anchor element".to_string())?;

        anchor.set_href(&url);
        anchor.set_download(file_name);
        anchor.click();

        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (file_name, payload);
        Err("CSV download is only available in wasm32 builds".to_string())
    }
}
