//! Money: restocking, wages, rent/utilities, story debt payments, nightly ledger.

use super::logic;
use super::types::*;
use super::world::*;
use crate::content::{Content, PaymentRule};

/// Classic: the supplier still delivers this much on credit when money is short.
pub const CLASSIC_OVERDRAFT: i64 = 1500;
/// The most one supply order (and the auto-restock target) may be: well above anything a
/// night uses, far below where the till's numbers would overflow.
pub const MAX_SUPPLY_ORDER: i64 = 100_000;

/// Night start: top up ingredients to the auto-restock target.
pub(crate) fn restock(w: &mut World, _content: &Content) {
    if let Some(target) = w.econ.auto_restock
        && w.econ.supplies < target
    {
        let need = target - w.econ.supplies;
        // Cozy lets you go a little into the red for food; Classic buys what it can afford.
        let afford = if w.difficulty.gentle() {
            need
        } else {
            need.min(w.econ.money.max(0) + CLASSIC_OVERDRAFT)
        };
        if afford > 0 {
            w.econ.money -= afford;
            w.econ.supplies += afford;
            w.tonight.supplies += afford;
        }
    }
    w.tonight.supplies += std::mem::take(&mut w.econ.day_supplies);
}

/// Manual purchase (day phase or night).
pub(crate) fn buy_supplies(w: &mut World, amount: i64) -> Result<(), CommandError> {
    if amount <= 0 || amount > MAX_SUPPLY_ORDER {
        return Err(CommandError::Invalid("amount out of range".into()));
    }
    if w.econ.money < amount && w.difficulty == Difficulty::Classic {
        return Err(CommandError::NotEnoughMoney {
            need: amount,
            have: w.econ.money,
        });
    }
    w.econ.money -= amount;
    w.econ.supplies += amount;
    if w.phase == Phase::Night {
        w.tonight.supplies += amount;
    } else {
        w.econ.day_supplies += amount;
    }
    Ok(())
}

/// `BeatKind::PaymentDue`: collect the story payment for tonight.
pub(crate) fn collect_payment(w: &mut World, content: &Content) {
    let night = w.night;
    let Some(p) = content.story.payment_on(night) else {
        return;
    };
    w.story.payments_processed = w.story.payments_processed.max(night);
    let amount = p.amount.min(w.econ.debt.max(0));
    if amount == 0 {
        // already paid off
        if let Some(x) = w.story.payments.iter_mut().find(|x| x.0 == night) {
            x.2 = true;
        }
        return;
    }
    // Relaxed (no money pressure): the instalment is covered — the debt shrinks, the till
    // keeps its money, and the story counts it as paid
    if content
        .difficulty(w.difficulty.id())
        .is_some_and(|d| d.payments == PaymentRule::Forgiven)
    {
        w.econ.debt -= amount;
        if let Some(x) = w.story.payments.iter_mut().find(|x| x.0 == night) {
            x.2 = true;
        }
        *w.story.counters.entry("payments_made".into()).or_insert(0) += 1;
        w.log_key(
            "access.log.payment_covered",
            vec![("amount".into(), amount.to_string())],
        );
        return;
    }
    if w.econ.money >= amount {
        w.econ.money -= amount;
        w.econ.debt -= amount;
        w.tonight.payment = amount;
        if let Some(x) = w.story.payments.iter_mut().find(|x| x.0 == night) {
            x.2 = true;
        }
        *w.story.counters.entry("payments_made".into()).or_insert(0) += 1;
        let debt = w.econ.debt;
        w.emit(SimEvent::PaymentMade {
            amount,
            debt_left: debt,
        });
        let ctx = EffectCtx::default();
        super::app::bot_with(
            w,
            content,
            "payment_done",
            &ctx,
            vec![
                ("amount".into(), amount.to_string()),
                ("debt".into(), debt.to_string()),
            ],
        );
        w.log_key(
            "app.bot.payment_done",
            vec![("amount".into(), amount.to_string())],
        );
    } else {
        w.tonight.payment_missed = true;
        w.emit(SimEvent::PaymentMissed { amount });
        let diff = w.difficulty.id().to_string();
        if let Some(d) = content.difficulty(&diff) {
            let mut ctx = EffectCtx::default();
            logic::run(
                w,
                content,
                &EffectSrc::MissedPayment(diff.clone()),
                &d.missed_payment_effects,
                &mut ctx,
            );
            if p.is_final && d.payments == PaymentRule::Hard {
                logic::set_flag(w, content, "final_payment_missed");
            }
        }
    }
}

/// 06:00: wages, rent, red-ink streak.
pub(crate) fn on_night_end(w: &mut World, _content: &Content) {
    let night = w.night;
    let mut wages = 0i64;
    for s in w.staff.iter_mut() {
        if s.owner {
            continue;
        }
        let vacation = s
            .leave_nights
            .iter()
            .any(|(n, k)| *n == night && *k == LeaveKind::Vacation);
        let hours = if vacation {
            8.0
        } else {
            (s.minutes_worked as f32 / 60.0).min(s.shift.hours())
        };
        if hours <= 0.0 {
            continue;
        }
        wages += (s.wage as f32 * hours / 8.0).round() as i64;
        s.hours_week += hours;
    }
    w.econ.money -= wages;
    w.tonight.wages = wages;
    let rent = w.econ.rent;
    w.econ.money -= rent;
    w.tonight.rent = rent;
    let t = &mut w.tonight;
    t.net = t.revenue + t.tips + t.other_income
        - t.wages
        - t.supplies
        - t.rent
        - t.other_expenses
        - t.payment;
    if w.econ.money < 0 {
        w.econ.nights_in_red += 1;
    } else {
        w.econ.nights_in_red = 0;
    }
}
