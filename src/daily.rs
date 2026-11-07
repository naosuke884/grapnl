//! 日替わりのコースの日付と、次のコースに変わるまでの時間。
//! コースの種は `game::today_seed` が UTC の日から作る。日本時間では毎朝 9:00 に変わる。

/// 1 日の秒数。`today_seed` と同じ区切り。
const DAY_SECONDS: f64 = 86_400.0;

/// 種（1970-01-01 からの UTC の日数）のコースの日付（月, 日）。
/// UTC の 0:00 は日本時間の同じ日の 9:00 なので、日本時間で見てもコースが始まる日はこの日付になる。
pub fn course_date(seed: u64) -> (u32, u32) {
    // 日数から暦の日付へ（Howard Hinnant の days_from_civil の逆）。3 月始まりの年で数える。
    let z = seed + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (month as u32, day as u32)
}

/// 種のコースが次のコースに変わるまでの秒数（切り捨て）。もう変わっていたら `None`。
pub fn seconds_until_next_course(seed: u64, unix_seconds: f64) -> Option<u64> {
    let next = (seed + 1) as f64 * DAY_SECONDS;
    let left = next - unix_seconds;
    (left > 0.0).then(|| left.floor() as u64)
}

/// 残り時間を「時:分:秒」にする。時は桁を揃えない（3:12:05）。
pub fn format_countdown(seconds: u64) -> String {
    let h = seconds / 3_600;
    let m = seconds / 60 % 60;
    let s = seconds % 60;
    format!("{h}:{m:02}:{s:02}")
}

/// ゲームオーバーの画面に出す、遊んだコースの日付と次のコースまでの時間。
pub fn course_line(seed: u64, unix_seconds: f64) -> String {
    let (month, day) = course_date(seed);
    match seconds_until_next_course(seed, unix_seconds) {
        Some(left) => format!(
            "今日のコース {month}/{day}   次のコースまで {}",
            format_countdown(left)
        ),
        // ゲームオーバーの画面のまま日をまたいだ。やり直すと新しい日の種で始まる。
        None => format!("{month}/{day} のコースは終わり   もう一度で次のコースへ"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::today_seed;

    /// 2026-11-07 0:00 UTC（日本時間 9:00）。
    const NOV_7_2026: f64 = 1_794_009_600.0;

    #[test]
    fn course_date_is_the_utc_date_of_the_seed() {
        assert_eq!(course_date(0), (1, 1));
        assert_eq!(course_date(today_seed(NOV_7_2026)), (11, 7));
        assert_eq!(course_date(today_seed(NOV_7_2026) - 1), (11, 6));
        // うるう日と年の変わり目。
        assert_eq!(course_date(today_seed(1_709_164_800.0)), (2, 29));
        assert_eq!(course_date(today_seed(1_709_164_800.0) + 1), (3, 1));
        assert_eq!(course_date(today_seed(NOV_7_2026) + 55), (1, 1));
    }

    #[test]
    fn countdown_reaches_zero_at_utc_midnight() {
        let seed = today_seed(NOV_7_2026);
        let next = NOV_7_2026 + 86_400.0;
        assert_eq!(seconds_until_next_course(seed, NOV_7_2026), Some(86_400));
        assert_eq!(
            seconds_until_next_course(seed, NOV_7_2026 + 0.5),
            Some(86_399)
        );
        assert_eq!(seconds_until_next_course(seed, next - 1.0), Some(1));
        assert_eq!(seconds_until_next_course(seed, next - 0.5), Some(0));
        assert_eq!(seconds_until_next_course(seed, next), None);
        assert_eq!(seconds_until_next_course(seed, next + 10.0), None);
        // 0:00 UTC からは次の種になり、その残りはまた 1 日分。
        assert_eq!(today_seed(next), seed + 1);
        assert_eq!(seconds_until_next_course(seed + 1, next), Some(86_400));
    }

    #[test]
    fn countdown_is_hours_minutes_seconds() {
        assert_eq!(format_countdown(0), "0:00:00");
        assert_eq!(format_countdown(59), "0:00:59");
        assert_eq!(format_countdown(3 * 3_600 + 12 * 60 + 5), "3:12:05");
        assert_eq!(format_countdown(35_999), "9:59:59");
        assert_eq!(format_countdown(86_399), "23:59:59");
        assert_eq!(format_countdown(86_400), "24:00:00");
    }

    #[test]
    fn course_line_shows_the_date_and_time_left() {
        let seed = today_seed(NOV_7_2026);
        let now = NOV_7_2026 + 86_400.0 - (3.0 * 3_600.0 + 12.0 * 60.0 + 5.0);
        assert_eq!(
            course_line(seed, now),
            "今日のコース 11/7   次のコースまで 3:12:05"
        );
        assert_eq!(
            course_line(seed, NOV_7_2026 + 86_400.0),
            "11/7 のコースは終わり   もう一度で次のコースへ"
        );
    }
}
