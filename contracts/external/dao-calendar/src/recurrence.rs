// ═══════════════════════════ Recurrence Engine ═══════════════════════════
// Computes recurring event instances from a RecurrenceRule.
// All functions are pure — no storage access, suitable for query handlers.

use cosmwasm_std::Timestamp;

use crate::msg::{
    ComputedInstance, DayOfWeek, RecurrenceEnd, RecurrenceFrequency, RecurrenceRule,
};

const DEFAULT_MAX_INSTANCES: u32 = 30;
const MAX_INSTANCES: u32 = 100;

/// Compute future instances of a recurring event within the given time range.
/// Returns up to `limit` instances (default 30, max 100).
/// Sets `has_more` if additional instances exist beyond the limit.
pub fn compute_instances(
    rule: &RecurrenceRule,
    event_start: Timestamp,
    event_end: Timestamp,
    from: Option<Timestamp>,
    to: Option<Timestamp>,
    limit: Option<u32>,
) -> (Vec<ComputedInstance>, bool) {
    let limit = limit
        .unwrap_or(DEFAULT_MAX_INSTANCES)
        .min(MAX_INSTANCES);
    let from_secs = from.unwrap_or(event_start).seconds() as i64;
    let to_secs = to
        .map(|t| t.seconds() as i64)
        .unwrap_or(i64::MAX);
    let duration = event_end.seconds() as i64 - event_start.seconds() as i64;
    let origin_secs = event_start.seconds() as i64;

    let mut instances: Vec<ComputedInstance> = Vec::new();
    let mut occ = 0u32;

    loop {
        occ += 1;
        if let Some(max_count) = match &rule.end_condition {
            RecurrenceEnd::AfterCount(n) => Some(*n),
            _ => None,
        } {
            if occ > max_count {
                break;
            }
        }

        let occ_start = match compute_occurrence_time(rule, origin_secs, occ) {
            Some(t) => t,
            None => continue,
        };

        let occ_end = occ_start + duration;

        // Check end condition by date
        if let RecurrenceEnd::AtDate(end_date) = &rule.end_condition {
            if occ_start > end_date.seconds() as i64 {
                break;
            }
        }

        // Skip if entirely before the range
        if occ_end < from_secs {
            continue;
        }

        // Stop if past the range
        if occ_start > to_secs {
            break;
        }

        instances.push(ComputedInstance {
            occurrence_number: occ,
            start_time: Timestamp::from_seconds(occ_start as u64),
            end_time: Timestamp::from_seconds(occ_end as u64),
        });

        if instances.len() >= limit as usize {
            // Check if there would be more
            let has_more = has_more_instances(rule, occ + 1, occ_start, occ_end, to_secs);
            return (instances, has_more);
        }
    }

    (instances, false)
}

/// Determine if at least one more instance exists after occ `next_occ`.
fn has_more_instances(
    rule: &RecurrenceRule,
    next_occ: u32,
    _last_start: i64,
    _last_end: i64,
    to_secs: i64,
) -> bool {
    if let RecurrenceEnd::AfterCount(n) = &rule.end_condition {
        return next_occ <= *n;
    }
    // For Never or AtDate, there's always more instances (we just didn't compute them)
    // But check if end condition AtDate is before our range start
    if let RecurrenceEnd::AtDate(d) = &rule.end_condition {
        if (d.seconds() as i64) < to_secs {
            // end date is within range, so there may be more
            return next_occ as u64 <= rule.max_possible_occ();
        }
        return true;
    }
    true
}

impl RecurrenceRule {
    fn max_possible_occ(&self) -> u64 {
        match &self.end_condition {
            RecurrenceEnd::Never => u64::MAX,
            RecurrenceEnd::AfterCount(n) => *n as u64,
            // Approximate: at most 100 years of daily events
            RecurrenceEnd::AtDate(_) => 36500,
        }
    }
}

/// Compute the start time (Unix seconds) of occurrence number `occ` (1-indexed).
fn compute_occurrence_time(rule: &RecurrenceRule, origin_secs: i64, occ: u32) -> Option<i64> {
    let occ = occ as i64;
    let interval = rule.interval.max(1) as i64;

    match rule.frequency {
        RecurrenceFrequency::Daily => Some(origin_secs + (occ - 1) * interval * 86400),

        RecurrenceFrequency::Weekly => {
            if let Some(ref days) = rule.days_of_week {
                if days.is_empty() {
                    // No days specified, fall back to interval weeks
                    return Some(origin_secs + (occ - 1) * interval * 7 * 86400);
                }

                // We need to find the Nth matching weekday
                // origin_secs is the first occurrence's start
                // occ=1 -> origin_secs (already a matching day)
                // occ>1 -> find the next matching weekday

                if occ == 1 {
                    return Some(origin_secs);
                }

                // Convert origin_secs to a date to compute weekday
                let origin_weekday = days_since_unix_epoch(origin_secs) % 7; // Mon=0 ... Sun=6
                let mut days_to_add = 0i64;
                let mut matches_found = 1i64; // occ 1 already accounted for

                // Sort the days of week
                let mut sorted_days: Vec<u8> =
                    days.iter().map(|d| d.weekday_num()).collect();
                sorted_days.sort();
                sorted_days.dedup();

                // Optimization: iterate per-week intervals
                let week_span = 7 * interval;
                // Number of days per week that match
                let days_per_week = sorted_days.len() as i64;

                if days_per_week == 0 {
                    return None;
                }

                // How many complete weeks to skip
                let complete_weeks = (occ - 1) / days_per_week;
                let remainder = (occ - 1) % days_per_week;

                // Within the first complete_weeks, we have complete_weeks * week_span days
                // But the first week might be partial (only days after origin_weekday)
                // Let's use a simpler approach: iterate until we fill occ

                // Actually, let's find the correct occurrence by walking forward
                let target_occ = occ;
                let mut current_occ = 1i64;
                let mut current_secs = origin_secs;

                loop {
                    let current_weekday = days_since_unix_epoch(current_secs) % 7;
                    // Find the next matching day in this week
                    let mut found_next = false;

                    for &dow in &sorted_days {
                        let dow_i64 = dow as i64;
                        if dow_i64 > current_weekday {
                            let diff = dow_i64 - current_weekday;
                            current_secs += diff * 86400;
                            current_occ += 1;
                            if current_occ == target_occ {
                                return Some(current_secs);
                            }
                            found_next = true;
                            break;
                        }
                    }

                    if !found_next {
                        // Go to next interval's week, first matching day
                        current_secs += (week_span - (current_weekday - sorted_days[0] as i64)) * 86400;
                        current_occ += 1;
                        if current_occ == target_occ {
                            return Some(current_secs);
                        }
                        // Reset to beginning of the week for proper weekday tracking
                        // Actually, we jumped to the first matching day of next interval week
                    }
                }
            } else {
                // No specific days: interval weeks from origin
                Some(origin_secs + (occ - 1) * interval * 7 * 86400)
            }
        }

        RecurrenceFrequency::Monthly => {
            // Every `interval` months on the same day of month as origin
            let origin_days = days_since_unix_epoch(origin_secs);
            let origin_day_of_month = {
                // Approximate: day of month from Unix epoch days
                let year_len = 365;
                let leap_years = |y: i64| y / 4 - y / 100 + y / 400;
                let months_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

                let mut remaining = origin_days;
                let mut year = 1970i64;
                loop {
                    let ylen = if is_leap(year) { 366 } else { 365 };
                    if remaining < ylen {
                        break;
                    }
                    remaining -= ylen;
                    year += 1;
                }
                let mut month = 0usize;
                while month < 12 {
                    let mdays = if month == 1 && is_leap(year) {
                        29
                    } else {
                        months_days[month]
                    };
                    if remaining < mdays {
                        break;
                    }
                    remaining -= mdays;
                    month += 1;
                }
                (remaining + 1, year, month) // day_of_month (1-indexed), year, month (0-indexed)
            };

            let (day_of_month, origin_year, origin_month) = origin_day_of_month;
            let total_months = (origin_year * 12 + origin_month as i64) + (occ - 1) * interval;
            let target_year = total_months / 12;
            let target_month = (total_months % 12) as usize;

            let days_in_target_month = if target_month == 1 && is_leap(target_year) {
                29
            } else {
                [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][target_month]
            };

            let clamped_day = day_of_month.min(days_in_target_month);
            let target_days = days_from_ymd(target_year, target_month, clamped_day);

            let time_of_day = origin_secs - days_since_unix_epoch(origin_secs) * 86400;
            Some(target_days * 86400 + time_of_day)
        }

        RecurrenceFrequency::Yearly => {
            let origin_days = days_since_unix_epoch(origin_secs);
            let time_of_day = origin_secs - origin_days * 86400;

            let (origin_day_of_year, origin_year) = {
                let mut remaining = origin_days;
                let mut y = 1970i64;
                loop {
                    let ylen = if is_leap(y) { 366 } else { 365 };
                    if remaining < ylen {
                        break;
                    }
                    remaining -= ylen;
                    y += 1;
                }
                (remaining, y)
            };

            let target_year = origin_year + (occ - 1) * interval;
            let target_days = days_from_ymd(target_year, 0, 1) + origin_day_of_year;

            // Handle Feb 29 → Feb 28 in non-leap years
            let days_in_year = if is_leap(target_year) { 366 } else { 365 };
            let clamped_days = if origin_day_of_year >= days_in_year {
                days_in_year - 1
            } else {
                origin_day_of_year
            };

            let target_secs = (days_from_ymd(target_year, 0, 1) + clamped_days) * 86400 + time_of_day;
            Some(target_secs)
        }
    }
}

// ═══════════════════════════ Date Helpers ═══════════════════════════

/// Days since Unix epoch (1970-01-01).
fn days_since_unix_epoch(secs: i64) -> i64 {
    secs / 86400
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days from Unix epoch to (year, month, day).
/// month: 0-indexed (0=January), day: 1-indexed.
fn days_from_ymd(year: i64, month: usize, day: i64) -> i64 {
    let months_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut total = 0i64;
    // Days from 1970 to year
    for y in 1970..year {
        total += if is_leap(y) { 366 } else { 365 };
    }
    // Days from start of year to month
    for m in 0..month {
        total += if m == 1 && is_leap(year) {
            29
        } else {
            months_days[m]
        };
    }
    // Days from start of month to day
    total += day - 1;
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::DayOfWeek::*;

    fn inst(occ: u32, start: u64, end: u64) -> ComputedInstance {
        ComputedInstance {
            occurrence_number: occ,
            start_time: Timestamp::from_seconds(start),
            end_time: Timestamp::from_seconds(end),
        }
    }

    #[test]
    fn daily_recurrence() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            days_of_week: None,
            end_condition: RecurrenceEnd::AfterCount(5),
        };
        let instances = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000),
            Timestamp::from_seconds(1003600), // 1 hour later
            None,
            None,
            Some(10),
        );
        assert_eq!(instances.0.len(), 5);
        assert!(!instances.1); // no more
        assert_eq!(instances.0[0].start_time.seconds(), 1000000);
        assert_eq!(instances.0[1].start_time.seconds(), 1000000 + 86400);
        assert_eq!(instances.0[2].start_time.seconds(), 1000000 + 2 * 86400);
    }

    #[test]
    fn daily_interval_3() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 3,
            days_of_week: None,
            end_condition: RecurrenceEnd::AfterCount(4),
        };
        let (instances, _) = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000),
            Timestamp::from_seconds(1003600),
            None,
            None,
            Some(10),
        );
        assert_eq!(instances.len(), 4);
        assert_eq!(instances[1].start_time.seconds(), 1000000 + 3 * 86400);
        assert_eq!(instances[2].start_time.seconds(), 1000000 + 6 * 86400);
    }

    #[test]
    fn weekly_no_days() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            days_of_week: None,
            end_condition: RecurrenceEnd::AfterCount(3),
        };
        let (instances, _) = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000),
            Timestamp::from_seconds(1003600),
            None,
            None,
            Some(10),
        );
        assert_eq!(instances.len(), 3);
        assert_eq!(instances[1].start_time.seconds(), 1000000 + 7 * 86400);
    }

    #[test]
    fn weekly_tues_thurs() {
        // Origin is a Monday (Jan 12, 1970). 1000000 / 86400 = 11.57...
        // Jan 12, 1970 = Monday
        // So we want Tuesday (1) and Thursday (3) of that week and beyond
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            days_of_week: Some(vec![Tuesday, Thursday]),
            end_condition: RecurrenceEnd::AfterCount(6),
        };
        let (instances, _) = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000), // Monday
            Timestamp::from_seconds(1003600),
            None,
            None,
            Some(10),
        );
        assert_eq!(instances.len(), 6);
        // occ 1 = Monday (origin)
        // Let's just check we got 6 instances
    }

    #[test]
    fn monthly_recurrence() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Monthly,
            interval: 2,
            days_of_week: None,
            end_condition: RecurrenceEnd::AfterCount(3),
        };
        let (instances, _) = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000),  // Jan 12, 1970 approx
            Timestamp::from_seconds(1003600),
            None,
            None,
            Some(10),
        );
        assert_eq!(instances.len(), 3);
    }

    #[test]
    fn yearly_recurrence() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Yearly,
            interval: 1,
            days_of_week: None,
            end_condition: RecurrenceEnd::AfterCount(3),
        };
        let (instances, _) = compute_instances(
            &rule,
            Timestamp::from_seconds(100000000), // ~Mar 3, 1973
            Timestamp::from_seconds(100003600),
            None,
            None,
            Some(10),
        );
        assert_eq!(instances.len(), 3);
        // Each should be ~365 days apart
        assert!(
            instances[1].start_time.seconds() as i64 - instances[0].start_time.seconds() as i64
                >= 365 * 86400 - 86400
        );
    }

    #[test]
    fn never_ends() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            days_of_week: None,
            end_condition: RecurrenceEnd::Never,
        };
        let (instances, has_more) = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000),
            Timestamp::from_seconds(1003600),
            None,
            Some(Timestamp::from_seconds(1000000 + 3 * 86400)),
            Some(2),
        );
        assert_eq!(instances.len(), 2);
        assert!(has_more);
    }

    #[test]
    fn duration_preserved() {
        let rule = RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            days_of_week: None,
            end_condition: RecurrenceEnd::AfterCount(3),
        };
        let (instances, _) = compute_instances(
            &rule,
            Timestamp::from_seconds(1000000),
            Timestamp::from_seconds(1007200), // 2 hours
            None,
            None,
            Some(10),
        );
        for inst in &instances {
            assert_eq!(
                inst.end_time.seconds() - inst.start_time.seconds(),
                7200
            );
        }
    }
}