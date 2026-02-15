//! Utilitaires pour le formatage des dates en format relatif.

use chrono::{DateTime, Duration, Local, TimeZone, Utc};

/// Formate un timestamp unix en date relative ("il y a 2h", "hier", etc.)
pub fn format_relative_time(timestamp: i64) -> String {
    let datetime = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now());
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
        "à l'instant".to_string()
    } else if minutes < 60 {
        if minutes == 1 {
            "il y a 1 minute".to_string()
        } else {
            format!("il y a {} minutes", minutes)
        }
    } else if hours < 24 {
        if hours == 1 {
            "il y a 1 heure".to_string()
        } else {
            format!("il y a {} heures", hours)
        }
    } else if days == 1 {
        "hier".to_string()
    } else if days < 7 {
        format!("il y a {} jours", days)
    } else if weeks < 4 {
        if weeks == 1 {
            "il y a 1 semaine".to_string()
        } else {
            format!("il y a {} semaines", weeks)
        }
    } else if months < 12 {
        if months == 1 {
            "il y a 1 mois".to_string()
        } else {
            format!("il y a {} mois", months)
        }
    } else {
        if years == 1 {
            "il y a 1 an".to_string()
        } else {
            format!("il y a {} ans", years)
        }
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

    #[test]
    fn test_format_relative_time_now() {
        let now = Utc::now().timestamp();
        let result = format_relative_time(now);
        assert_eq!(result, "à l'instant");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let five_min_ago = (Utc::now() - Duration::minutes(5)).timestamp();
        let result = format_relative_time(five_min_ago);
        assert_eq!(result, "il y a 5 minutes");
    }

    #[test]
    fn test_format_relative_time_hours() {
        let two_hours_ago = (Utc::now() - Duration::hours(2)).timestamp();
        let result = format_relative_time(two_hours_ago);
        assert_eq!(result, "il y a 2 heures");
    }

    #[test]
    fn test_format_relative_time_yesterday() {
        let yesterday = (Utc::now() - Duration::days(1)).timestamp();
        let result = format_relative_time(yesterday);
        assert_eq!(result, "hier");
    }

    #[test]
    fn test_format_relative_time_days() {
        let three_days_ago = (Utc::now() - Duration::days(3)).timestamp();
        let result = format_relative_time(three_days_ago);
        assert_eq!(result, "il y a 3 jours");
    }

    #[test]
    fn test_format_relative_time_weeks() {
        let two_weeks_ago = (Utc::now() - Duration::weeks(2)).timestamp();
        let result = format_relative_time(two_weeks_ago);
        assert_eq!(result, "il y a 2 semaines");
    }

    #[test]
    fn test_format_relative_time_months() {
        let two_months_ago = (Utc::now() - Duration::days(60)).timestamp();
        let result = format_relative_time(two_months_ago);
        assert_eq!(result, "il y a 2 mois");
    }

    #[test]
    fn test_format_relative_time_years() {
        let two_years_ago = (Utc::now() - Duration::days(730)).timestamp();
        let result = format_relative_time(two_years_ago);
        assert_eq!(result, "il y a 2 ans");
    }
}
