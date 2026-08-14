//! Closing deliberately.
//!
//! Luna is meant to stay running, so quitting is a decision rather than an accident.
//! Two things stand between a click on Quit and the process ending.
//!
//! **Tools get a say.** A service in the middle of something that cannot be resumed,
//! such as a file conversion writing its output, can ask for a moment or ask the user
//! to confirm. It cannot refuse outright: a tool that could veto shutdown would be a
//! tool that can trap the user in the app.
//!
//! **The user is told what stops.** Everything scheduled runs through one scheduler, so
//! the prompt can say what will not happen while Luna is closed. That is the one piece
//! of information that actually changes the decision, which is why it is worth
//! assembling properly rather than showing a generic confirmation.

use chrono::{DateTime, Duration, Utc};

use crate::scheduler::Upcoming;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// How long a tool may hold up shutdown before it is closed anyway.
///
/// Long enough to finish writing a file, short enough that a stuck tool cannot make
/// Quit feel broken.
pub const MAX_DELAY: Duration = Duration::seconds(30);

/// What a tool says when asked whether Luna may close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownVote {
    /// Nothing in progress. The default.
    Allow,

    /// Closing now would lose something the user would want to know about.
    ///
    /// Surfaced in the prompt. The user may still choose to quit.
    NeedsConfirmation { message: String },

    /// Something is mid-flight and would be corrupted by stopping now.
    ///
    /// Holds the door for up to [`MAX_DELAY`], then closes regardless. A tool that
    /// could block indefinitely would be a tool that can trap the user.
    RequestDelay {
        reason: String,
        /// Roughly how much longer, if the tool knows.
        estimate: Option<Duration>,
    },
}

impl ShutdownVote {
    /// Convenience for the common case.
    pub fn allow() -> Self {
        return ShutdownVote::Allow;
    }

    pub fn is_allow(&self) -> bool {
        return matches!(self, ShutdownVote::Allow);
    }
}

/// One tool's answer, with who gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVote {
    pub tool_id: String,
    pub vote: ShutdownVote,
}

/// Something that will stop happening while Luna is closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PausedWork {
    /// What it is, in the user's words. The rule id if nothing better is known.
    pub label: String,
    /// The tool it belongs to, if any.
    pub tool_id: Option<String>,
    /// When it would next have happened.
    pub next_at: DateTime<Utc>,
}

impl PausedWork {
    /// How far away the next occurrence is, in words.
    pub fn when(&self, now: DateTime<Utc>) -> String {
        return describe_relative(now, self.next_at);
    }
}

/// Everything the user needs in order to decide whether to quit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShutdownReport {
    /// Tools asking to finish something first.
    pub delays: Vec<ToolVote>,
    /// Tools wanting the user to confirm.
    pub confirmations: Vec<ToolVote>,
    /// Scheduled work that stops while closed, soonest first.
    pub paused: Vec<PausedWork>,
}

impl ShutdownReport {
    /// Assembles a report from the tools' votes and what is scheduled.
    ///
    /// `upcoming` comes from [`crate::Scheduler::upcoming`], which reports when each
    /// alert fires rather than when its subject is, so a reminder set a week before a
    /// birthday shows the reminder's date.
    pub fn assemble(votes: Vec<ToolVote>, upcoming: Vec<Upcoming>) -> Self {
        let mut report = Self::default();

        for vote in votes {
            match vote.vote {
                ShutdownVote::Allow => {}
                ShutdownVote::NeedsConfirmation { .. } => report.confirmations.push(vote),
                ShutdownVote::RequestDelay { .. } => report.delays.push(vote),
            }
        }

        report.paused = upcoming
            .into_iter()
            .map(|u| PausedWork {
                label: u.rule_id,
                tool_id: u.tool_id,
                next_at: u.due_at,
            })
            .collect();

        report.paused.sort_by_key(|p| p.next_at);

        return report;
    }

    /// Whether Luna can close without asking anything.
    ///
    /// False when a tool wants confirmation, when one is mid-flight, or when something
    /// scheduled would stop. The last is the usual reason: a user with reminders set
    /// should be told they will not fire.
    pub fn can_close_silently(&self) -> bool {
        return self.delays.is_empty() && self.confirmations.is_empty() && self.paused.is_empty();
    }

    /// Whether anything asked for more time.
    pub fn wants_delay(&self) -> bool {
        return !self.delays.is_empty();
    }

    /// The longest delay any tool asked for, capped at [`MAX_DELAY`].
    pub fn delay_needed(&self) -> Duration {
        let longest = self
            .delays
            .iter()
            .filter_map(|v| match &v.vote {
                ShutdownVote::RequestDelay { estimate, .. } => *estimate,
                _ => None,
            })
            .max()
            .unwrap_or(Duration::zero());

        return longest.min(MAX_DELAY);
    }

    /// Lines for the prompt, in the order they should be shown.
    ///
    /// Reasons a tool gave come first, because they are about work in progress and are
    /// more urgent than a schedule that will resume when Luna reopens.
    pub fn lines(&self, now: DateTime<Utc>) -> Vec<String> {
        let mut lines = Vec::new();

        for vote in &self.delays {
            if let ShutdownVote::RequestDelay { reason, .. } = &vote.vote {
                lines.push(format!("{} is still working: {reason}", vote.tool_id));
            }
        }

        for vote in &self.confirmations {
            if let ShutdownVote::NeedsConfirmation { message } = &vote.vote {
                lines.push(format!("{}: {message}", vote.tool_id));
            }
        }

        for work in &self.paused {
            lines.push(format!(
                "{} will not run, next {}",
                work.label,
                work.when(now)
            ));
        }

        return lines;
    }

    /// A one-line summary for a compact prompt.
    pub fn headline(&self, now: DateTime<Utc>) -> String {
        if self.can_close_silently() {
            return "Nothing is scheduled. Luna can close safely.".to_string();
        }

        if let Some(soonest) = self.paused.first() {
            return format!(
                "{} scheduled item(s) will not run while Luna is closed. The next is {}.",
                self.paused.len(),
                soonest.when(now)
            );
        }

        return "Some tools are still working.".to_string();
    }
}

/// Describes a moment relative to now, in the way a person would say it.
///
/// Deliberately coarse. "In about 3 hours" is what a decision to quit turns on; the
/// exact minute is not.
pub fn describe_relative(now: DateTime<Utc>, at: DateTime<Utc>) -> String {
    let delta = at.signed_duration_since(now);

    if delta < Duration::zero() {
        return "already overdue".to_string();
    }

    let minutes = delta.num_minutes();

    if minutes < 1 {
        return "in under a minute".to_string();
    }

    if minutes < 60 {
        return format!("in {minutes} minute{}", plural(minutes));
    }

    let hours = delta.num_hours();

    if hours < 24 {
        return format!("in {hours} hour{}", plural(hours));
    }

    let days = delta.num_days();

    if days < 7 {
        return format!("in {days} day{}", plural(days));
    }

    let weeks = days / 7;

    return format!("in {weeks} week{}", plural(weeks));
}

fn plural(n: i64) -> &'static str {
    return if n == 1 { "" } else { "s" };
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        return Utc.with_ymd_and_hms(2026, 8, 14, 12, 0, 0).unwrap();
    }

    fn upcoming(rule: &str, at: DateTime<Utc>) -> Upcoming {
        return Upcoming {
            rule_id: rule.to_string(),
            tool_id: Some("luna.reminders".to_string()),
            due_at: at,
            subject_at: at,
        };
    }

    fn delay(tool: &str, reason: &str, estimate: Option<Duration>) -> ToolVote {
        return ToolVote {
            tool_id: tool.to_string(),
            vote: ShutdownVote::RequestDelay {
                reason: reason.to_string(),
                estimate,
            },
        };
    }

    #[test]
    fn an_idle_luna_closes_without_asking() {
        let report = ShutdownReport::assemble(vec![], vec![]);

        assert!(report.can_close_silently());
        assert!(!report.wants_delay());
        assert!(report.lines(now()).is_empty());
    }

    #[test]
    fn allowing_votes_do_not_appear_in_the_report() {
        let votes = vec![
            ToolVote { tool_id: "luna.a".into(), vote: ShutdownVote::Allow },
            ToolVote { tool_id: "luna.b".into(), vote: ShutdownVote::Allow },
        ];

        let report = ShutdownReport::assemble(votes, vec![]);

        assert!(report.can_close_silently());
    }

    #[test]
    fn scheduled_work_alone_is_enough_to_warrant_asking() {
        let report = ShutdownReport::assemble(
            vec![],
            vec![upcoming("standup", now() + Duration::hours(3))],
        );

        assert!(
            !report.can_close_silently(),
            "a user with reminders set should be told they will not fire"
        );
        assert_eq!(report.lines(now()).len(), 1);
    }

    #[test]
    fn paused_work_is_listed_soonest_first() {
        let report = ShutdownReport::assemble(
            vec![],
            vec![
                upcoming("later", now() + Duration::days(2)),
                upcoming("sooner", now() + Duration::minutes(10)),
                upcoming("middle", now() + Duration::hours(5)),
            ],
        );

        let labels: Vec<&str> = report.paused.iter().map(|p| p.label.as_str()).collect();

        assert_eq!(labels, vec!["sooner", "middle", "later"]);
    }

    #[test]
    fn work_in_progress_is_listed_before_schedules() {
        let report = ShutdownReport::assemble(
            vec![delay("luna.converter", "writing output.mp3", None)],
            vec![upcoming("standup", now() + Duration::hours(3))],
        );

        let lines = report.lines(now());

        assert!(lines[0].contains("luna.converter"), "{lines:?}");
        assert!(lines[1].contains("standup"), "{lines:?}");
    }

    #[test]
    fn a_delay_is_capped_so_a_stuck_tool_cannot_trap_the_user() {
        let report = ShutdownReport::assemble(
            vec![delay("luna.stuck", "thinking", Some(Duration::hours(3)))],
            vec![],
        );

        assert!(report.wants_delay());
        assert_eq!(report.delay_needed(), MAX_DELAY);
    }

    #[test]
    fn a_short_delay_is_honoured_as_asked() {
        let report = ShutdownReport::assemble(
            vec![delay("luna.converter", "finishing", Some(Duration::seconds(5)))],
            vec![],
        );

        assert_eq!(report.delay_needed(), Duration::seconds(5));
    }

    #[test]
    fn the_longest_requested_delay_wins() {
        let report = ShutdownReport::assemble(
            vec![
                delay("luna.a", "a", Some(Duration::seconds(3))),
                delay("luna.b", "b", Some(Duration::seconds(9))),
            ],
            vec![],
        );

        assert_eq!(report.delay_needed(), Duration::seconds(9));
    }

    #[test]
    fn a_delay_with_no_estimate_still_registers() {
        let report = ShutdownReport::assemble(
            vec![delay("luna.vague", "busy", None)],
            vec![],
        );

        assert!(report.wants_delay());
        assert_eq!(report.delay_needed(), Duration::zero());
    }

    #[test]
    fn a_confirmation_is_shown_but_does_not_delay() {
        let report = ShutdownReport::assemble(
            vec![ToolVote {
                tool_id: "luna.notes".into(),
                vote: ShutdownVote::NeedsConfirmation {
                    message: "You have unsaved notes".into(),
                },
            }],
            vec![],
        );

        assert!(!report.can_close_silently());
        assert!(!report.wants_delay(), "asking is not the same as holding the door");
        assert!(report.lines(now())[0].contains("unsaved notes"));
    }

    #[test]
    fn the_headline_leads_with_what_stops() {
        let report = ShutdownReport::assemble(
            vec![],
            vec![
                upcoming("standup", now() + Duration::minutes(10)),
                upcoming("backup", now() + Duration::hours(14)),
            ],
        );

        let headline = report.headline(now());

        assert!(headline.contains('2'), "{headline}");
        assert!(headline.contains("10 minutes"), "{headline}");
    }

    #[test]
    fn an_idle_headline_says_so() {
        let report = ShutdownReport::assemble(vec![], vec![]);

        assert!(report.headline(now()).contains("safely"));
    }

    #[test]
    fn relative_times_read_the_way_a_person_would_say_them() {
        let n = now();

        assert_eq!(describe_relative(n, n + Duration::seconds(20)), "in under a minute");
        assert_eq!(describe_relative(n, n + Duration::minutes(1)), "in 1 minute");
        assert_eq!(describe_relative(n, n + Duration::minutes(10)), "in 10 minutes");
        assert_eq!(describe_relative(n, n + Duration::hours(1)), "in 1 hour");
        assert_eq!(describe_relative(n, n + Duration::hours(5)), "in 5 hours");
        assert_eq!(describe_relative(n, n + Duration::days(1)), "in 1 day");
        assert_eq!(describe_relative(n, n + Duration::days(3)), "in 3 days");
        assert_eq!(describe_relative(n, n + Duration::days(14)), "in 2 weeks");
    }

    #[test]
    fn something_already_due_says_so_rather_than_showing_a_negative() {
        let n = now();

        assert_eq!(describe_relative(n, n - Duration::hours(1)), "already overdue");
    }

    #[test]
    fn the_prompt_reads_as_intended() {
        // The shape from the architecture doc, end to end.
        let report = ShutdownReport::assemble(
            vec![delay("luna.converter", "writing output.mp3", Some(Duration::seconds(20)))],
            vec![
                upcoming("standup reminder", now() + Duration::hours(3)),
                upcoming("health sample", now() + Duration::minutes(10)),
            ],
        );

        let lines = report.lines(now());

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("still working"));
        assert!(lines[1].contains("health sample"), "soonest schedule first: {lines:?}");
        assert!(lines[2].contains("standup reminder"));
    }
}
