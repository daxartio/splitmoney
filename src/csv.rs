use std::collections::{HashMap, HashSet};

use crate::state::{
    AppState, Expense, ExpenseIcon, Participant, Settlement, Share, adjust_shares_to_total,
    normalize_currency, normalize_expense_title,
};
use csv::{StringRecord, Trim};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CSV_HEADERS: [&str; 16] = [
    "record_type",
    "record_id",
    "created_at",
    "participant_id",
    "participant_name",
    "participant_active",
    "title",
    "icon",
    "payer_id",
    "total_cents",
    "currency",
    "shares_json",
    "from_id",
    "to_id",
    "amount_cents",
    "note",
];

#[derive(Clone, Debug, PartialEq)]
pub struct ImportSummary {
    pub participants: usize,
    pub expenses: usize,
    pub settlements: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CsvShare {
    participant_id: Uuid,
    amount_cents: i64,
}

pub fn export_state_csv(state: &AppState) -> Result<String, String> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(Vec::new());

    writer
        .write_record(CSV_HEADERS)
        .map_err(|err| format!("Failed to write CSV header: {err}"))?;

    for participant in &state.participants {
        writer
            .write_record([
                "participant",
                "",
                "",
                &participant.id.to_string(),
                &participant.name,
                if participant.is_active {
                    "true"
                } else {
                    "false"
                },
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ])
            .map_err(|err| format!("Failed to export participant: {err}"))?;
    }

    for expense in &state.expenses {
        let shares = expense
            .shares
            .iter()
            .map(|share| CsvShare {
                participant_id: share.participant_id,
                amount_cents: share.amount_cents,
            })
            .collect::<Vec<_>>();
        let shares_payload = serde_json::to_string(&shares)
            .map_err(|err| format!("Failed to serialize expense shares: {err}"))?;

        writer
            .write_record([
                "expense",
                &expense.id.to_string(),
                &expense.created_at.to_string(),
                "",
                "",
                "",
                &expense.title,
                &serde_json::to_string(&expense.icon)
                    .map_err(|err| format!("Failed to serialize expense icon: {err}"))?
                    .replace('"', ""),
                &expense.payer_id.to_string(),
                &expense.total_cents.to_string(),
                &expense.currency,
                &shares_payload,
                "",
                "",
                "",
                "",
            ])
            .map_err(|err| format!("Failed to export expense: {err}"))?;
    }

    for settlement in &state.settlements {
        writer
            .write_record([
                "settlement",
                &settlement.id.to_string(),
                &settlement.created_at.to_string(),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &settlement.currency,
                "",
                &settlement.from_id.to_string(),
                &settlement.to_id.to_string(),
                &settlement.amount_cents.to_string(),
                &settlement.note,
            ])
            .map_err(|err| format!("Failed to export settlement: {err}"))?;
    }

    let bytes = writer
        .into_inner()
        .map_err(|err| format!("Failed to finalize CSV: {err}"))?;

    String::from_utf8(bytes).map_err(|err| format!("Exported CSV is not valid UTF-8: {err}"))
}

pub fn import_state_csv(payload: &str) -> Result<(AppState, ImportSummary), String> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .from_reader(payload.as_bytes());

    let headers = reader
        .headers()
        .map_err(|err| format!("Failed to read CSV header: {err}"))?
        .clone();

    let Some(record_type_index) = headers.iter().position(|header| header == "record_type") else {
        return Err("CSV header must include 'record_type'.".to_string());
    };

    let mut participants_map = HashMap::<Uuid, Participant>::new();
    let mut expenses = Vec::<Expense>::new();
    let mut settlements = Vec::<Settlement>::new();
    let mut seen_ids = HashSet::<Uuid>::new();

    for (row_offset, row_result) in reader.records().enumerate() {
        let row_number = row_offset + 2;
        let row = row_result.map_err(|err| format!("CSV row {row_number} is invalid: {err}"))?;
        let record_type = row
            .get(record_type_index)
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        if record_type.is_empty() {
            continue;
        }

        match record_type.as_str() {
            "participant" => {
                let participant_id =
                    parse_uuid_field(&headers, &row, "participant_id", row_number)?;
                let name = get_field(&headers, &row, "participant_name")
                    .trim()
                    .to_string();
                if name.is_empty() {
                    return Err(format!("CSV row {row_number} has empty participant_name."));
                }

                let is_active = match get_field(&headers, &row, "participant_active")
                    .trim()
                    .to_lowercase()
                    .as_str()
                {
                    "" | "1" | "true" | "yes" => true,
                    "0" | "false" | "no" => false,
                    _ => {
                        return Err(format!(
                            "CSV row {row_number} has invalid participant_active value."
                        ));
                    }
                };

                participants_map.insert(
                    participant_id,
                    Participant {
                        id: participant_id,
                        name,
                        is_active,
                    },
                );
            }
            "expense" => {
                let expense_id = parse_uuid_field(&headers, &row, "record_id", row_number)?;
                if !seen_ids.insert(expense_id) {
                    return Err(format!("CSV row {row_number} has duplicate record_id."));
                }

                let title = normalize_expense_title(get_field(&headers, &row, "title"));
                if title.is_empty() {
                    return Err(format!("CSV row {row_number} has empty expense title."));
                }

                let icon = parse_icon_field(&headers, &row, "icon", row_number)?;
                let payer_id = parse_uuid_field(&headers, &row, "payer_id", row_number)?;
                let total_cents = parse_i64_field(&headers, &row, "total_cents", row_number)?;
                if total_cents <= 0 {
                    return Err(format!(
                        "CSV row {row_number} total_cents must be positive."
                    ));
                }

                let currency = normalize_currency(get_field(&headers, &row, "currency"));
                let created_at = parse_i64_or_default(&headers, &row, "created_at", 0)?;
                let shares_json = get_field(&headers, &row, "shares_json");

                let shares = parse_shares(shares_json, row_number)?;
                if shares.is_empty() {
                    return Err(format!("CSV row {row_number} has no shares."));
                }

                for share in &shares {
                    if share.amount_cents < 0 {
                        return Err(format!("CSV row {row_number} has a negative share amount."));
                    }
                }

                participants_map.entry(payer_id).or_insert(Participant {
                    id: payer_id,
                    name: format!("Imported {}", &payer_id.to_string()[..8]),
                    is_active: true,
                });
                for share in &shares {
                    participants_map
                        .entry(share.participant_id)
                        .or_insert(Participant {
                            id: share.participant_id,
                            name: format!("Imported {}", &share.participant_id.to_string()[..8]),
                            is_active: true,
                        });
                }

                expenses.push(Expense {
                    id: expense_id,
                    title,
                    icon,
                    payer_id,
                    total_cents,
                    currency,
                    created_at,
                    shares,
                });
            }
            "settlement" => {
                let settlement_id = parse_uuid_field(&headers, &row, "record_id", row_number)?;
                if !seen_ids.insert(settlement_id) {
                    return Err(format!("CSV row {row_number} has duplicate record_id."));
                }

                let from_id = parse_uuid_field(&headers, &row, "from_id", row_number)?;
                let to_id = parse_uuid_field(&headers, &row, "to_id", row_number)?;
                let amount_cents = parse_i64_field(&headers, &row, "amount_cents", row_number)?;
                let currency = normalize_currency(get_field(&headers, &row, "currency"));
                if amount_cents <= 0 {
                    return Err(format!(
                        "CSV row {row_number} amount_cents must be positive."
                    ));
                }

                if from_id == to_id {
                    return Err(format!(
                        "CSV row {row_number} has identical from_id and to_id values."
                    ));
                }

                participants_map.entry(from_id).or_insert(Participant {
                    id: from_id,
                    name: format!("Imported {}", &from_id.to_string()[..8]),
                    is_active: true,
                });
                participants_map.entry(to_id).or_insert(Participant {
                    id: to_id,
                    name: format!("Imported {}", &to_id.to_string()[..8]),
                    is_active: true,
                });

                settlements.push(Settlement {
                    id: settlement_id,
                    from_id,
                    to_id,
                    amount_cents,
                    currency,
                    created_at: parse_i64_or_default(&headers, &row, "created_at", 0)?,
                    note: get_field(&headers, &row, "note").trim().to_string(),
                });
            }
            _ => {}
        }
    }

    let mut participants = participants_map.into_values().collect::<Vec<_>>();
    participants.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    for expense in &mut expenses {
        adjust_shares_to_total(&mut expense.shares, expense.total_cents);
    }

    let mut state = AppState {
        participants,
        expenses,
        settlements,
        last_currency: String::new(),
    };
    state.normalize_after_import();

    let summary = ImportSummary {
        participants: state.participants.len(),
        expenses: state.expenses.len(),
        settlements: state.settlements.len(),
    };

    Ok((state, summary))
}

fn parse_shares(payload: &str, row_number: usize) -> Result<Vec<Share>, String> {
    if payload.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed: Vec<CsvShare> = serde_json::from_str(payload)
        .map_err(|err| format!("CSV row {row_number} has invalid shares_json: {err}"))?;

    Ok(parsed
        .into_iter()
        .map(|share| Share {
            participant_id: share.participant_id,
            amount_cents: share.amount_cents,
        })
        .collect())
}

fn parse_icon_field(
    headers: &StringRecord,
    row: &StringRecord,
    field: &str,
    row_number: usize,
) -> Result<ExpenseIcon, String> {
    let raw = get_field(headers, row, field);
    if raw.trim().is_empty() {
        return Ok(ExpenseIcon::Other);
    }

    serde_json::from_str(&format!("\"{}\"", raw.trim().to_lowercase()))
        .map_err(|_| format!("CSV row {row_number} has invalid icon value '{raw}'."))
}

fn parse_uuid_field(
    headers: &StringRecord,
    row: &StringRecord,
    field: &str,
    row_number: usize,
) -> Result<Uuid, String> {
    let raw = get_field(headers, row, field);
    if raw.trim().is_empty() {
        return Err(format!("CSV row {row_number} is missing '{field}'."));
    }

    Uuid::parse_str(raw.trim())
        .map_err(|_| format!("CSV row {row_number} has invalid UUID in '{field}'."))
}

fn parse_i64_field(
    headers: &StringRecord,
    row: &StringRecord,
    field: &str,
    row_number: usize,
) -> Result<i64, String> {
    let raw = get_field(headers, row, field);
    raw.trim()
        .parse::<i64>()
        .map_err(|_| format!("CSV row {row_number} has invalid integer in '{field}'."))
}

fn parse_i64_or_default(
    headers: &StringRecord,
    row: &StringRecord,
    field: &str,
    default: i64,
) -> Result<i64, String> {
    let raw = get_field(headers, row, field).trim().to_string();
    if raw.is_empty() {
        return Ok(default);
    }

    raw.parse::<i64>()
        .map_err(|_| format!("CSV has invalid integer in '{field}'."))
}

fn get_field<'a>(headers: &'a StringRecord, row: &'a StringRecord, field: &str) -> &'a str {
    headers
        .iter()
        .position(|header| header == field)
        .and_then(|index| row.get(index))
        .unwrap_or_default()
}
