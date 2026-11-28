//! The Shiftella app (owner, 2026-11-28): a request or a bot button answered in one place is
//! answered everywhere (the inbox, the Requests list, the player-bot), and nothing whose
//! moment has passed can still be pressed — the sim refuses it, not only the UI.

use super::*;
use crate::content::Content;

fn content() -> Content {
    super::tests_t10::content()
}

/// Story day `n` with three helpers (a crew to ask for things).
fn crew_day(c: &Content, n: u32) -> World {
    let mut w = World::new_story_seeded(c, Difficulty::Cozy, 41);
    w.sys.tutorial_off = true;
    bot::finish_cutscenes(&mut w, c);
    w.night = n;
    w.begin_day(c);
    bot::finish_cutscenes(&mut w, c);
    w.debug_set_money(40_000);
    for _ in 0..3 {
        if let Some(id) = w.candidates().first().map(|x| x.id) {
            w.apply(c, Command::Hire { candidate: id }).unwrap();
        }
    }
    w.drain_events();
    w
}

/// The night has begun and the clock shows `minute` since 22:00.
fn night_at(w: &mut World, c: &Content, minute: u32) {
    if w.phase() == Phase::Day {
        w.start_night(c);
        bot::finish_cutscenes(w, c);
    }
    w.prep_left = 0;
    w.tick = minute * TICKS_PER_MINUTE;
    w.drain_events();
}

fn to_next_day(w: &mut World, c: &Content) {
    if w.phase() == Phase::Day {
        w.start_night(c);
    }
    for _ in 0..NIGHT_TICKS * 2 {
        bot::finish_cutscenes(w, c);
        if w.phase() == Phase::Day {
            break;
        }
        w.tick(c);
    }
    bot::finish_cutscenes(w, c);
    w.drain_events();
}

fn args(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// A bot message (and the request it creates, for swap / time-off templates).
fn message(w: &mut World, c: &Content, template: &str, a: &[(&str, &str)]) -> BotMsg {
    app::bot_with(w, c, template, &EffectCtx::default(), args(a));
    w.drain_events();
    w.app.inbox.last().cloned().unwrap()
}

fn press(w: &mut World, c: &Content, msg: u32, action: &str) -> Result<(), CommandError> {
    w.apply(
        c,
        Command::BotAction {
            msg,
            action: action.into(),
        },
    )
}

/// The world as a save, without its pending events (did a refused command change anything?).
fn snapshot(w: &World) -> String {
    let mut x = w.clone();
    x.events.clear();
    x.save_to_string().unwrap()
}

fn answer(w: &mut World, c: &Content, id: u32, approve: bool) -> Result<(), CommandError> {
    if w.app.swaps.iter().any(|s| s.id == id) {
        w.apply(c, Command::AnswerSwap { id, approve })
    } else if w.app.time_off.iter().any(|s| s.id == id) {
        w.apply(c, Command::AnswerTimeOff { id, approve })
    } else {
        w.apply(c, Command::AnswerEarlyLeave { id, approve })
    }
}

/// An early-leave request for tonight and its bot message.
fn early_leave(w: &mut World, c: &Content) -> BotMsg {
    let staff = w.staff.iter().find(|s| !s.owner).unwrap().id;
    let rid = w.alloc_id();
    let night = w.night;
    w.sys.rozpis.early_leave.push(EarlyLeaveRequest {
        id: rid,
        staff,
        night,
        minute: 240,
        status: RequestStatus::Pending,
    });
    let name = w.staff_by_id(staff).unwrap().name.clone();
    app::bot_request(
        w,
        c,
        "early_leave",
        &EffectCtx::default(),
        rid,
        args(&[("time", "02:00"), ("name", &name)]),
    );
    w.drain_events();
    w.app.inbox.last().cloned().unwrap()
}

/// Every kind of request with a bot message: time off, náhradní volno, swap, early leave.
fn requests(w: &mut World, c: &Content) -> Vec<BotMsg> {
    let mut out = vec![
        message(w, c, "time_off_vacation", &[]),
        message(w, c, "time_off_toil", &[]),
        message(w, c, "swap_request", &[]),
    ];
    out.push(early_leave(w, c));
    for m in &out {
        assert!(m.request.is_some(), "{} has its request", m.template);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Answered in one place, answered everywhere
// ---------------------------------------------------------------------------------------------

#[test]
fn a_request_answered_in_the_inbox_has_no_buttons_left_in_the_requests_list() {
    let c = content();
    for approve in [true, false] {
        let mut w = crew_day(&c, 4);
        for m in requests(&mut w, &c) {
            let rid = m.request.unwrap();
            assert!(w.request_open(rid), "{}: open before", m.template);
            assert_eq!(w.bot_msg_state(&c, m.id), BotMsgState::Open);
            let action = if approve { "approve" } else { "decline" };
            press(&mut w, &c, m.id, action).unwrap();
            // the Requests list shows its buttons only for an open request
            assert!(!w.request_open(rid), "{}: closed after", m.template);
            let before = snapshot(&w);
            assert!(
                answer(&mut w, &c, rid, true).is_err(),
                "{}: answered twice",
                m.template
            );
            assert_eq!(
                snapshot(&w),
                before,
                "{}: a refusal changes nothing",
                m.template
            );
            assert_eq!(
                w.bot_msg_state(&c, m.id),
                BotMsgState::Answered(action.into())
            );
        }
    }
}

#[test]
fn a_request_answered_in_the_requests_list_closes_its_inbox_message() {
    let c = content();
    for approve in [true, false] {
        let mut w = crew_day(&c, 4);
        for m in requests(&mut w, &c) {
            let rid = m.request.unwrap();
            answer(&mut w, &c, rid, approve).unwrap();
            let status = if approve {
                RequestStatus::Approved
            } else {
                RequestStatus::Declined
            };
            assert_eq!(
                w.bot_msg_state(&c, m.id),
                BotMsgState::Resolved(status),
                "{}",
                m.template
            );
            // its buttons are gone: pressing one anyway is refused and changes nothing (the
            // message used to close with the wrong answer on it)
            let before = snapshot(&w);
            for a in ["approve", "decline"] {
                assert!(press(&mut w, &c, m.id, a).is_err(), "{}.{a}", m.template);
            }
            assert_eq!(snapshot(&w), before, "{}", m.template);
            let msg = w.app.inbox.iter().find(|x| x.id == m.id).unwrap();
            assert_eq!(msg.answered, None, "{}", m.template);
        }
    }
}

#[test]
fn the_badge_counts_only_what_can_still_be_answered() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let others = w.open_bot_messages(&c);
    let msgs = requests(&mut w, &c);
    let open = w.open_requests();
    answer(&mut w, &c, msgs[0].request.unwrap(), true).unwrap();
    assert_eq!(w.open_requests(), open - 1);
    assert_eq!(
        w.open_bot_messages(&c),
        others + msgs.len() as u32 - 1,
        "the answered request's message isn't waiting any more"
    );
}

#[test]
fn an_unanswered_request_lapses_at_its_night_and_says_so() {
    let c = content();
    let mut w = crew_day(&c, 4);
    // a request the bot makes by itself (sandbox / story beats) is about a night too
    let m = message(&mut w, &c, "time_off_vacation", &[]);
    let rid = m.request.unwrap();
    let night = w.app.time_off.iter().find(|r| r.id == rid).unwrap().night;
    assert_eq!(w.request_meta(rid).map(|x| x.night), Some(night));
    while w.night() < night {
        to_next_day(&mut w, &c);
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    let ev = w.drain_events();
    let r = w.app.time_off.iter().find(|r| r.id == rid).unwrap();
    assert_eq!(r.status, RequestStatus::Lapsed, "no answer is not a no");
    assert!(
        ev.iter().any(
            |e| matches!(e, SimEvent::BotMessage { template, .. } if template == "manage_alone")
        ),
        "the honest line: I'll manage"
    );
    assert_eq!(
        w.bot_msg_state(&c, m.id),
        BotMsgState::Resolved(RequestStatus::Lapsed)
    );
    let before = snapshot(&w);
    assert!(press(&mut w, &c, m.id, "approve").is_err());
    assert!(answer(&mut w, &c, rid, true).is_err());
    assert_eq!(snapshot(&w), before);
}

// ---------------------------------------------------------------------------------------------
// Nothing whose moment has passed
// ---------------------------------------------------------------------------------------------

#[test]
fn a_reservation_whose_time_has_passed_cannot_be_made() {
    let c = content();
    let mut w = crew_day(&c, 9);
    let reg = [
        ("name", "regular.vera.name"),
        ("time", "23:00"),
        ("regular", "vera"),
        ("minute", "60"),
    ];
    let early = message(&mut w, &c, "regular_tonight", &reg);
    let late = message(&mut w, &c, "regular_tonight", &reg);
    // 22:30: still time to keep her seat
    night_at(&mut w, &c, 30);
    assert_eq!(w.bot_msg_state(&c, early.id), BotMsgState::Open);
    press(&mut w, &c, early.id, "reserve").unwrap();
    assert!(w.reserved_seats().contains_key("vera"));
    w.sys.tool.reserved.clear();
    // 01:00: 23:00 has long gone (the owner could still reserve it)
    night_at(&mut w, &c, 180);
    assert_eq!(w.bot_msg_state(&c, late.id), BotMsgState::Expired);
    let before = snapshot(&w);
    assert!(press(&mut w, &c, late.id, "reserve").is_err());
    assert!(press(&mut w, &c, late.id, "noted").is_err());
    assert_eq!(snapshot(&w), before);
    assert!(w.reserved_seats().is_empty());
}

#[test]
fn a_reservation_too_close_to_its_time_or_from_last_night_is_refused() {
    let c = content();
    let mut w = crew_day(&c, 9);
    let reg = [
        ("name", "regular.vera.name"),
        ("time", "23:00"),
        ("regular", "vera"),
        ("minute", "60"),
    ];
    let m = message(&mut w, &c, "regular_tonight", &reg);
    // 22:58: no time to set the card out — she's at the door
    night_at(&mut w, &c, 58);
    assert_eq!(w.bot_msg_state(&c, m.id), BotMsgState::Expired);
    assert!(press(&mut w, &c, m.id, "reserve").is_err());
    // the next day, last night's message is over too
    let m2 = {
        let mut w2 = crew_day(&c, 9);
        let m = message(&mut w2, &c, "regular_tonight", &reg);
        to_next_day(&mut w2, &c);
        assert_eq!(w2.bot_msg_state(&c, m.id), BotMsgState::Expired);
        assert!(press(&mut w2, &c, m.id, "reserve").is_err());
        assert!(w2.reserved_seats().is_empty());
        m
    };
    assert_eq!(m2.night, 9);
}

#[test]
fn the_day_message_about_a_regular_is_for_a_time_inside_opening_hours() {
    let c = content();
    for n in 2..=30 {
        let mut w = crew_day(&c, n);
        for m in w
            .app
            .inbox
            .iter()
            .filter(|m| m.template == "regular_tonight" || m.template == "regular_to_go")
        {
            let at: u32 = m
                .args
                .iter()
                .find(|a| a.0 == "minute")
                .and_then(|a| a.1.parse().ok())
                .unwrap();
            assert!(
                (tool::RESERVE_LEAD_MINUTES..LAST_CALL).contains(&at),
                "night {n}: {at}"
            );
        }
        let _ = &mut w;
    }
}

#[test]
fn a_reminder_for_a_time_already_gone_is_refused() {
    let c = content();
    let mut w = crew_day(&c, 9);
    let reg = [
        ("name", "regular.radek.name"),
        ("time", "23:00"),
        ("regular", "radek"),
        ("minute", "60"),
    ];
    let m = message(&mut w, &c, "regular_to_go", &reg);
    night_at(&mut w, &c, 120);
    assert_eq!(w.bot_msg_state(&c, m.id), BotMsgState::Expired);
    assert!(press(&mut w, &c, m.id, "remind").is_err());
    assert!(w.sys.tool.reminders.is_empty());
}

#[test]
fn a_drone_that_would_land_after_closing_is_not_paid_for() {
    let c = content();
    let mut w = crew_day(&c, 12);
    let placed = (2..14).any(|y| {
        (2..28).any(|x| {
            w.apply(
                &c,
                Command::PlaceFurniture {
                    def: "drone_dock".into(),
                    x,
                    y,
                    rot: 0,
                },
            )
            .is_ok()
        })
    });
    assert!(placed);
    let m = message(
        &mut w,
        &c,
        "low_stock",
        &[("item", "cappuccino"), ("n", "3")],
    );
    // 05:40: a 30-minute drone lands after six
    night_at(&mut w, &c, 460);
    let money = w.money();
    assert!(press(&mut w, &c, m.id, "drone").is_err());
    assert_eq!(w.money(), money);
    assert!(w.sys.tool.drones.is_empty());
    // the other button still works
    press(&mut w, &c, m.id, "menu_off").unwrap();
}

#[test]
fn covering_for_a_late_helper_who_has_arrived_is_over() {
    let c = content();
    let mut w = crew_day(&c, 9);
    night_at(&mut w, &c, 10);
    let i = (0..w.staff.len()).find(|&i| !w.staff[i].owner).unwrap();
    let name = w.staff[i].name.clone();
    w.staff[i].status = StaffStatus::Expected { minute: 40 };
    let m = message(
        &mut w,
        &c,
        "late_alert",
        &[("name", &name), ("minutes", "15")],
    );
    assert_eq!(w.bot_msg_state(&c, m.id), BotMsgState::Open);
    w.staff[i].status = StaffStatus::OnDuty;
    assert_eq!(w.bot_msg_state(&c, m.id), BotMsgState::Expired);
    assert!(press(&mut w, &c, m.id, "cover").is_err());
    assert!(press(&mut w, &c, m.id, "ask_other").is_err());
}

#[test]
fn time_off_or_a_swap_for_a_night_already_begun_cannot_be_answered() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let staff: Vec<u32> = w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect();
    night_at(&mut w, &c, 60);
    // requests without a meta record (old saves, the chaos tests) are about a night too
    let t = w.alloc_id();
    let night = w.night;
    w.app.time_off.push(TimeOffRequest {
        id: t,
        staff: staff[0],
        kind: LeaveKind::Vacation,
        night,
        hours: 8.0,
        status: RequestStatus::Pending,
    });
    let s = w.alloc_id();
    let weekday = w.weekday;
    w.app.swaps.push(SwapRequest {
        id: s,
        from: staff[0],
        to: staff[1],
        weekday,
        status: RequestStatus::Pending,
    });
    for id in [t, s] {
        assert!(!w.request_open(id), "{id}");
        let before = snapshot(&w);
        assert!(answer(&mut w, &c, id, true).is_err(), "{id}");
        assert_eq!(snapshot(&w), before);
    }
    // tomorrow's time off can still be answered tonight
    let t2 = w.alloc_id();
    w.app.time_off.push(TimeOffRequest {
        id: t2,
        staff: staff[0],
        kind: LeaveKind::Vacation,
        night: night + 1,
        hours: 8.0,
        status: RequestStatus::Pending,
    });
    assert!(w.request_open(t2));
    answer(&mut w, &c, t2, true).unwrap();
}

#[test]
fn a_thank_you_is_posted_once_a_night_per_person() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let names: Vec<String> = w
        .staff
        .iter()
        .filter(|s| !s.owner)
        .map(|s| s.name.clone())
        .collect();
    let post = |w: &mut World, name: &str| {
        w.apply(
            &c,
            Command::PostAnnouncement {
                id: "thank_you".into(),
                args: args(&[("name", name)]),
            },
        )
    };
    post(&mut w, &names[0]).unwrap();
    assert!(w.announcement_posted_tonight("thank_you", &args(&[("name", &names[0])])));
    let before = snapshot(&w);
    assert!(post(&mut w, &names[0]).is_err(), "no mood farming");
    assert_eq!(snapshot(&w), before);
    post(&mut w, &names[1]).unwrap();
    to_next_day(&mut w, &c);
    post(&mut w, &names[0]).unwrap();
}

#[test]
fn the_rozpis_publish_lock_is_known_before_pressing() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let id = w.staff.iter().find(|s| !s.owner).unwrap().id;
    let flip = |w: &mut World, wd: u8| {
        let cur = w.app.draft[&id][wd as usize];
        let shift = if cur == Shift::Off {
            Shift::Full
        } else {
            Shift::Off
        };
        w.apply(
            &c,
            Command::SetShift {
                staff: id,
                weekday: wd,
                shift,
            },
        )
        .unwrap();
    };
    flip(&mut w, 2);
    assert!(!w.rozpis_publish_locked());
    w.apply(&c, Command::PublishSchedule).unwrap();
    flip(&mut w, 3);
    assert!(w.rozpis_publish_locked(), "once a week");
    assert_eq!(
        w.apply(&c, Command::PublishSchedule),
        Err(CommandError::Locked)
    );
}
