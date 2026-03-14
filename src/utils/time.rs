//! Utilitaires pour le formatage des dates en format relatif.

use crate::i18n::text_owned;
use chrono::{DateTime, Duration, Local, TimeZone, Utc};

/// Formate un timestamp unix en date relative ("il y a 2h", "hier", etc.)
pub fn format_relative_time(timestamp: i64) -> String {
    let datetime = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);
    let now = Utc::now();
    let diff = now.signed_duration_since(datetime);

    format_duration(diff)
}

/// Formate une durée en texte relatif français.
fn format_duration(diff: Duration) -> String {
    let seconds = diff.num_seconds();
    let minutes = diff.num_minutes();
    let hours = diff.num_hours();
    let days = diff.num_days();
    let weeks = days / 7;
    let months = days / 30;
    let years = days / 365;

    if seconds < 60 {
        text_owned("a l'instant", "just now")
    } else if minutes < 60 {
        if minutes == 1 {
            text_owned("il y a 1 minute", "1 minute ago")
        } else {
            text_owned(
                format!("il y a {} minutes", minutes),
                format!("{} minutes ago", minutes),
            )
        }
    } else if hours < 24 {
        if hours == 1 {
            text_owned("il y a 1 heure", "1 hour ago")
        } else {
            text_owned(
                format!("il y a {} heures", hours),
                format!("{} hours ago", hours),
            )
        }
    } else if days == 1 {
        text_owned("hier", "yesterday")
    } else if days < 7 {
        text_owned(
            format!("il y a {} jours", days),
            format!("{} days ago", days),
        )
    } else if weeks < 4 {
        if weeks == 1 {
            text_owned("il y a 1 semaine", "1 week ago")
        } else {
            text_owned(
                format!("il y a {} semaines", weeks),
                format!("{} weeks ago", weeks),
            )
        }
    } else if months < 12 {
        if months == 1 {
            text_owned("il y a 1 mois", "1 month ago")
        } else {
            text_owned(
                format!("il y a {} mois", months),
                format!("{} months ago", months),
            )
        }
    } else if years == 1 {
        text_owned("il y a 1 an", "1 year ago")
    } else {
        text_owned(
            format!("il y a {} ans", years),
            format!("{} years ago", years),
        )
    }
}

/// Formate une date en format absolu (pour le panneau de détail).
pub fn format_absolute_time(timestamp: i64) -> String {
    let datetime: DateTime<Local> = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now);

    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{with_language, Language};

    #[test]
    fn test_format_relative_time_now() {
        with_language(Language::Fr, || {
            let now = Utc::now().timestamp();
            let result = format_relative_time(now);
            assert_eq!(result, "a l'instant");
        });
    }

    #[test]
    fn test_format_relative_time_minutes() {
        with_language(Language::Fr, || {
            let five_min_ago = (Utc::now() - Duration::minutes(5)).timestamp();
            let result = format_relative_time(five_min_ago);
            assert_eq!(result, "il y a 5 minutes");
        });
    }

    #[test]
    fn test_format_relative_time_hours() {
        with_language(Language::Fr, || {
            let two_hours_ago = (Utc::now() - Duration::hours(2)).timestamp();
            let result = format_relative_time(two_hours_ago);
            assert_eq!(result, "il y a 2 heures");
        });
    }

    #[test]
    fn test_format_relative_time_yesterday() {
        with_language(Language::Fr, || {
            let yesterday = (Utc::now() - Duration::days(1)).timestamp();
            let result = format_relative_time(yesterday);
            assert_eq!(result, "hier");
        });
    }

    #[test]
    fn test_format_relative_time_days() {
        with_language(Language::Fr, || {
            let three_days_ago = (Utc::now() - Duration::days(3)).timestamp();
            let result = format_relative_time(three_days_ago);
            assert_eq!(result, "il y a 3 jours");
        });
    }

    #[test]
    fn test_format_relative_time_weeks() {
        with_language(Language::Fr, || {
            let two_weeks_ago = (Utc::now() - Duration::weeks(2)).timestamp();
            let result = format_relative_time(two_weeks_ago);
            assert_eq!(result, "il y a 2 semaines");
        });
    }

    #[test]
    fn test_format_relative_time_months() {
        with_language(Language::Fr, || {
            let two_months_ago = (Utc::now() - Duration::days(60)).timestamp();
            let result = format_relative_time(two_months_ago);
            assert_eq!(result, "il y a 2 mois");
        });
    }

    #[test]
    fn test_format_relative_time_years() {
        with_language(Language::Fr, || {
            let two_years_ago = (Utc::now() - Duration::days(730)).timestamp();
            let result = format_relative_time(two_years_ago);
            assert_eq!(result, "il y a 2 ans");
        });
    }

    #[test]
    fn test_format_absolute_time_valid() {
        let timestamp = 1_700_000_000;
        let expected = Local
            .timestamp_opt(timestamp, 0)
            .single()
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let result = format_absolute_time(timestamp);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_absolute_time_epoch() {
        let timestamp = 0;
        let expected = Local
            .timestamp_opt(timestamp, 0)
            .single()
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let result = format_absolute_time(timestamp);
        assert_eq!(result, expected);
    }
}
