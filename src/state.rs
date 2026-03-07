use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MONEY_TOLERANCE_CENTS: i64 = 1;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseIcon {
    Food,
    Transport,
    Home,
    Drinks,
    Shopping,
    Entertainment,
    Other,
}

impl ExpenseIcon {
    pub const ALL: [ExpenseIcon; 7] = [
        ExpenseIcon::Food,
        ExpenseIcon::Transport,
        ExpenseIcon::Home,
        ExpenseIcon::Drinks,
        ExpenseIcon::Shopping,
        ExpenseIcon::Entertainment,
        ExpenseIcon::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExpenseIcon::Food => "Food",
            ExpenseIcon::Transport => "Transport",
            ExpenseIcon::Home => "Home",
            ExpenseIcon::Drinks => "Drinks",
            ExpenseIcon::Shopping => "Shopping",
            ExpenseIcon::Entertainment => "Entertainment",
            ExpenseIcon::Other => "Other",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Participant {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Share {
    pub participant_id: Uuid,
    pub amount_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Expense {
    pub id: Uuid,
    pub title: String,
    pub icon: ExpenseIcon,
    pub payer_id: Uuid,
    pub total_cents: i64,
    pub currency: String,
    pub created_at: i64,
    pub shares: Vec<Share>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settlement {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub amount_cents: i64,
    #[serde(default = "default_currency_code")]
    pub currency: String,
    pub created_at: i64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppState {
    pub participants: Vec<Participant>,
    pub expenses: Vec<Expense>,
    pub settlements: Vec<Settlement>,
    #[serde(default = "default_currency_code")]
    pub last_currency: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            participants: Vec::new(),
            expenses: Vec::new(),
            settlements: Vec::new(),
            last_currency: default_currency_code(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewExpenseInput {
    pub title: String,
    pub icon: ExpenseIcon,
    pub payer_id: Uuid,
    pub total_cents: i64,
    pub currency: String,
    pub created_at: Option<i64>,
    pub shares: Vec<Share>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewSettlementInput {
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub amount_cents: i64,
    pub currency: String,
    pub created_at: Option<i64>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParticipantBalance {
    pub participant: Participant,
    pub net_cents: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettlementSuggestion {
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub amount_cents: i64,
    pub currency: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurrencyBalanceGroup {
    pub currency: String,
    pub balances: Vec<ParticipantBalance>,
}

impl AppState {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn with_defaults(mut self) -> Self {
        for expense in &mut self.expenses {
            expense.currency = normalize_currency(&expense.currency);
        }
        for settlement in &mut self.settlements {
            settlement.currency = normalize_currency(&settlement.currency);
        }
        self.last_currency = normalize_currency(&self.last_currency);
        self
    }

    pub fn participant_by_id(&self, id: Uuid) -> Option<&Participant> {
        self.participants
            .iter()
            .find(|participant| participant.id == id)
    }

    pub fn add_participant(&mut self, name: String) -> Result<Participant, String> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err("Participant name is required.".to_string());
        }

        let participant = Participant {
            id: Uuid::new_v4(),
            name: clean_name.to_string(),
            is_active: true,
        };

        self.participants.push(participant.clone());
        Ok(participant)
    }

    pub fn rename_participant(
        &mut self,
        participant_id: Uuid,
        new_name: String,
    ) -> Result<(), String> {
        let clean_name = new_name.trim();
        if clean_name.is_empty() {
            return Err("Participant name is required.".to_string());
        }

        let Some(participant) = self
            .participants
            .iter_mut()
            .find(|participant| participant.id == participant_id)
        else {
            return Err("Participant was not found.".to_string());
        };

        participant.name = clean_name.to_string();
        Ok(())
    }

    pub fn set_participant_active(
        &mut self,
        participant_id: Uuid,
        is_active: bool,
    ) -> Result<(), String> {
        let Some(participant) = self
            .participants
            .iter_mut()
            .find(|participant| participant.id == participant_id)
        else {
            return Err("Participant was not found.".to_string());
        };

        participant.is_active = is_active;
        Ok(())
    }

    pub fn participant_is_used(&self, participant_id: Uuid) -> bool {
        self.expenses.iter().any(|expense| {
            expense.payer_id == participant_id
                || expense
                    .shares
                    .iter()
                    .any(|share| share.participant_id == participant_id)
        }) || self.settlements.iter().any(|settlement| {
            settlement.from_id == participant_id || settlement.to_id == participant_id
        })
    }

    pub fn add_expense(&mut self, input: NewExpenseInput) -> Result<Expense, String> {
        validate_new_expense(self, &input)?;

        let mut shares = input.shares;
        adjust_shares_to_total(&mut shares, input.total_cents);

        let expense = Expense {
            id: Uuid::new_v4(),
            title: normalize_expense_title(&input.title),
            icon: input.icon,
            payer_id: input.payer_id,
            total_cents: input.total_cents,
            currency: normalize_currency(&input.currency),
            created_at: input.created_at.unwrap_or_else(now_timestamp_ms),
            shares,
        };

        self.last_currency = expense.currency.clone();
        self.expenses.push(expense.clone());
        Ok(expense)
    }

    pub fn add_settlement(&mut self, input: NewSettlementInput) -> Result<Settlement, String> {
        validate_new_settlement(self, &input)?;

        let settlement = Settlement {
            id: Uuid::new_v4(),
            from_id: input.from_id,
            to_id: input.to_id,
            amount_cents: input.amount_cents,
            currency: normalize_currency(&input.currency),
            created_at: input.created_at.unwrap_or_else(now_timestamp_ms),
            note: input.note.trim().to_string(),
        };

        self.last_currency = settlement.currency.clone();
        self.settlements.push(settlement.clone());
        Ok(settlement)
    }

    pub fn update_expense(
        &mut self,
        expense_id: Uuid,
        input: NewExpenseInput,
    ) -> Result<(), String> {
        validate_new_expense(self, &input)?;

        let Some(index) = self
            .expenses
            .iter()
            .position(|expense| expense.id == expense_id)
        else {
            return Err("Expense was not found.".to_string());
        };

        let previous = &self.expenses[index];
        let mut shares = input.shares;
        adjust_shares_to_total(&mut shares, input.total_cents);

        self.expenses[index] = Expense {
            id: previous.id,
            title: normalize_expense_title(&input.title),
            icon: input.icon,
            payer_id: input.payer_id,
            total_cents: input.total_cents,
            currency: normalize_currency(&input.currency),
            created_at: input.created_at.unwrap_or(previous.created_at),
            shares,
        };

        self.last_currency = self.expenses[index].currency.clone();
        Ok(())
    }

    pub fn delete_expense(&mut self, expense_id: Uuid) -> Result<(), String> {
        let before = self.expenses.len();
        self.expenses.retain(|expense| expense.id != expense_id);
        if self.expenses.len() == before {
            return Err("Expense was not found.".to_string());
        }
        Ok(())
    }

    pub fn update_settlement(
        &mut self,
        settlement_id: Uuid,
        input: NewSettlementInput,
    ) -> Result<(), String> {
        validate_new_settlement(self, &input)?;

        let Some(index) = self
            .settlements
            .iter()
            .position(|settlement| settlement.id == settlement_id)
        else {
            return Err("Settlement was not found.".to_string());
        };

        let previous = &self.settlements[index];
        self.settlements[index] = Settlement {
            id: previous.id,
            from_id: input.from_id,
            to_id: input.to_id,
            amount_cents: input.amount_cents,
            currency: normalize_currency(&input.currency),
            created_at: input.created_at.unwrap_or(previous.created_at),
            note: input.note.trim().to_string(),
        };

        self.last_currency = self.settlements[index].currency.clone();
        Ok(())
    }

    pub fn delete_settlement(&mut self, settlement_id: Uuid) -> Result<(), String> {
        let before = self.settlements.len();
        self.settlements
            .retain(|settlement| settlement.id != settlement_id);
        if self.settlements.len() == before {
            return Err("Settlement was not found.".to_string());
        }
        Ok(())
    }

    pub fn compute_balances_by_currency(&self) -> Vec<CurrencyBalanceGroup> {
        let mut nets_by_currency = HashMap::<String, HashMap<Uuid, i64>>::new();

        for expense in &self.expenses {
            let currency = normalize_currency(&expense.currency);
            let nets = nets_by_currency.entry(currency).or_default();
            *nets.entry(expense.payer_id).or_insert(0) += expense.total_cents;
            for share in &expense.shares {
                *nets.entry(share.participant_id).or_insert(0) -= share.amount_cents;
            }
        }

        for settlement in &self.settlements {
            let currency = normalize_currency(&settlement.currency);
            let nets = nets_by_currency.entry(currency).or_default();
            *nets.entry(settlement.from_id).or_insert(0) += settlement.amount_cents;
            *nets.entry(settlement.to_id).or_insert(0) -= settlement.amount_cents;
        }

        let mut groups = Vec::new();
        for (currency, nets) in nets_by_currency {
            let mut balances = Vec::new();
            for participant in &self.participants {
                balances.push(ParticipantBalance {
                    participant: participant.clone(),
                    net_cents: *nets.get(&participant.id).unwrap_or(&0),
                });
            }

            balances.sort_by(|left, right| left.participant.name.cmp(&right.participant.name));
            groups.push(CurrencyBalanceGroup { currency, balances });
        }

        groups.sort_by(|left, right| left.currency.cmp(&right.currency));
        groups
    }

    pub fn settlement_suggestions_by_currency(&self) -> Vec<SettlementSuggestion> {
        let groups = self.compute_balances_by_currency();
        let mut suggestions = Vec::new();

        for group in groups {
            let mut debtors: Vec<(Uuid, i64)> = group
                .balances
                .iter()
                .filter_map(|balance| {
                    if balance.net_cents < 0 {
                        Some((balance.participant.id, -balance.net_cents))
                    } else {
                        None
                    }
                })
                .collect();

            let mut creditors: Vec<(Uuid, i64)> = group
                .balances
                .iter()
                .filter_map(|balance| {
                    if balance.net_cents > 0 {
                        Some((balance.participant.id, balance.net_cents))
                    } else {
                        None
                    }
                })
                .collect();

            debtors.sort_by_key(|(_, cents)| -(*cents));
            creditors.sort_by_key(|(_, cents)| -(*cents));

            let mut debtor_index = 0;
            let mut creditor_index = 0;
            while debtor_index < debtors.len() && creditor_index < creditors.len() {
                let (debtor_id, debtor_left) = debtors[debtor_index];
                let (creditor_id, creditor_left) = creditors[creditor_index];

                let transfer = debtor_left.min(creditor_left);
                if transfer > 0 {
                    suggestions.push(SettlementSuggestion {
                        from_id: debtor_id,
                        to_id: creditor_id,
                        amount_cents: transfer,
                        currency: group.currency.clone(),
                    });
                }

                debtors[debtor_index].1 -= transfer;
                creditors[creditor_index].1 -= transfer;

                if debtors[debtor_index].1 == 0 {
                    debtor_index += 1;
                }
                if creditors[creditor_index].1 == 0 {
                    creditor_index += 1;
                }
            }
        }

        suggestions
    }

    pub fn normalize_after_import(&mut self) {
        let participant_ids = self
            .participants
            .iter()
            .map(|participant| participant.id)
            .collect::<HashSet<_>>();

        for expense in &mut self.expenses {
            expense.title = normalize_expense_title(&expense.title);
            expense.currency = normalize_currency(&expense.currency);
            expense
                .shares
                .retain(|share| participant_ids.contains(&share.participant_id));
            adjust_shares_to_total(&mut expense.shares, expense.total_cents);
        }

        self.expenses.retain(|expense| {
            participant_ids.contains(&expense.payer_id)
                && expense.total_cents > 0
                && !expense.shares.is_empty()
                && !expense.shares.iter().any(|share| share.amount_cents < 0)
        });

        self.settlements.retain(|settlement| {
            settlement.amount_cents > 0
                && settlement.from_id != settlement.to_id
                && participant_ids.contains(&settlement.from_id)
                && participant_ids.contains(&settlement.to_id)
        });

        for settlement in &mut self.settlements {
            settlement.currency = normalize_currency(&settlement.currency);
        }

        self.participants
            .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        self.expenses
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));
        self.settlements
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));

        self.last_currency = if self.last_currency.trim().is_empty() {
            self.expenses
                .last()
                .map(|expense| expense.currency.clone())
                .or_else(|| {
                    self.settlements
                        .last()
                        .map(|settlement| settlement.currency.clone())
                })
                .unwrap_or_else(default_currency_code)
        } else {
            normalize_currency(&self.last_currency)
        };
    }
}

pub fn validate_new_expense(state: &AppState, input: &NewExpenseInput) -> Result<(), String> {
    if state.participant_by_id(input.payer_id).is_none() {
        return Err("Payer participant was not found.".to_string());
    }

    if input.total_cents <= 0 {
        return Err("Total amount must be greater than zero.".to_string());
    }

    if input.shares.is_empty() {
        return Err("Add at least one participant share.".to_string());
    }

    let mut sum_cents = 0_i64;
    for share in &input.shares {
        if share.amount_cents < 0 {
            return Err("Shares cannot be negative.".to_string());
        }
        if state.participant_by_id(share.participant_id).is_none() {
            return Err("One of the selected participants was not found.".to_string());
        }
        sum_cents += share.amount_cents;
    }

    let delta = (sum_cents - input.total_cents).abs();
    if delta > MONEY_TOLERANCE_CENTS {
        return Err(format!(
            "Shares must match total. Difference is {} cents.",
            sum_cents - input.total_cents
        ));
    }

    if normalize_expense_title(&input.title).is_empty() {
        return Err("Expense title is required.".to_string());
    }

    Ok(())
}

pub fn validate_new_settlement(state: &AppState, input: &NewSettlementInput) -> Result<(), String> {
    if input.from_id == input.to_id {
        return Err("Settlement sender and receiver must be different participants.".to_string());
    }

    if state.participant_by_id(input.from_id).is_none()
        || state.participant_by_id(input.to_id).is_none()
    {
        return Err("Settlement participant was not found.".to_string());
    }

    if input.amount_cents <= 0 {
        return Err("Settlement amount must be greater than zero.".to_string());
    }

    if input.currency.trim().is_empty() {
        return Err("Settlement currency is required.".to_string());
    }

    Ok(())
}

pub fn equal_split(total_cents: i64, participant_ids: &[Uuid]) -> Vec<Share> {
    if total_cents <= 0 || participant_ids.is_empty() {
        return Vec::new();
    }

    let base_share = total_cents / participant_ids.len() as i64;
    let mut remainder = total_cents % participant_ids.len() as i64;

    let mut shares = Vec::with_capacity(participant_ids.len());
    for participant_id in participant_ids {
        let mut amount = base_share;
        if remainder > 0 {
            amount += 1;
            remainder -= 1;
        }
        shares.push(Share {
            participant_id: *participant_id,
            amount_cents: amount,
        });
    }

    shares
}

pub fn adjust_shares_to_total(shares: &mut [Share], total_cents: i64) {
    if shares.is_empty() {
        return;
    }

    let current_total: i64 = shares.iter().map(|share| share.amount_cents).sum();
    let delta = total_cents - current_total;
    if delta == 0 {
        return;
    }

    if let Some(target_share) = shares.iter_mut().max_by_key(|share| share.amount_cents) {
        target_share.amount_cents += delta;
    }
}

pub fn now_timestamp_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let now = std::time::SystemTime::now();
        let since_epoch = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_millis(0));
        since_epoch.as_millis() as i64
    }
}

pub fn parse_money_to_cents(raw: &str) -> Result<i64, String> {
    let trimmed = raw.trim().replace(',', ".");
    if trimmed.is_empty() {
        return Err("Amount is required.".to_string());
    }

    if trimmed.starts_with('-') {
        return Err("Amount cannot be negative.".to_string());
    }

    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() > 2 {
        return Err("Amount format is invalid.".to_string());
    }

    let whole_part = parts[0]
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();

    if whole_part.is_empty() {
        return Err("Amount format is invalid.".to_string());
    }

    let whole = whole_part
        .parse::<i64>()
        .map_err(|_| "Amount is too large.".to_string())?;

    let frac = if parts.len() == 2 {
        let decimals = parts[1]
            .chars()
            .filter(|character| character.is_ascii_digit())
            .collect::<String>();

        if decimals.len() > 2 {
            return Err("Use at most two decimal places.".to_string());
        }

        match decimals.len() {
            0 => 0,
            1 => {
                decimals
                    .parse::<i64>()
                    .map_err(|_| "Amount format is invalid.".to_string())?
                    * 10
            }
            _ => decimals
                .parse::<i64>()
                .map_err(|_| "Amount format is invalid.".to_string())?,
        }
    } else {
        0
    };

    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(frac))
        .ok_or_else(|| "Amount is too large.".to_string())
}

pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    let whole = abs / 100;
    let frac = abs % 100;
    format!("{}{}.{:02}", sign, whole, frac)
}

pub fn normalize_currency(currency: &str) -> String {
    let normalized = currency
        .trim()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();

    if normalized.is_empty() {
        default_currency_code()
    } else {
        normalized
    }
}

pub fn default_currency_code() -> String {
    "USD".to_string()
}

pub fn normalize_expense_title(title: &str) -> String {
    title.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_money_to_cents() {
        assert_eq!(parse_money_to_cents("12").unwrap(), 1200);
        assert_eq!(parse_money_to_cents("12.3").unwrap(), 1230);
        assert_eq!(parse_money_to_cents("12.34").unwrap(), 1234);
        assert!(parse_money_to_cents("12.345").is_err());
    }

    #[test]
    fn equal_split_handles_remainder() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let shares = equal_split(100, &[a, b, c]);
        assert_eq!(
            shares.iter().map(|share| share.amount_cents).sum::<i64>(),
            100
        );
    }
}
