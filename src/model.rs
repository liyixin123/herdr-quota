use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const REFRESH_SECONDS: u64 = 60;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Window {
    pub remaining: Option<u8>,
    pub resets_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Quota {
    pub five_hour: Window,
    pub weekly: Window,
    pub usage_allowed: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Loading,
    Ready,
    RefreshFailed,
    AccountUnknown,
    LoggedOut,
    Unsupported,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Cache {
    pub attempted_at: u64,
    pub fetched_at: Option<u64>,
    // A one-way digest of account metadata, never authentication material.
    pub identity: Option<String>,
    pub quota: Option<Quota>,
    pub state: State,
}

impl Cache {
    pub fn due(&self, now: u64) -> bool {
        if now < self.attempted_at || now.saturating_sub(self.attempted_at) >= REFRESH_SECONDS {
            return true;
        }
        self.quota.as_ref().is_some_and(|q| {
            [&q.five_hour, &q.weekly].iter().any(|w| {
                w.resets_at
                    .is_some_and(|reset| reset > self.attempted_at && reset <= now)
            })
        })
    }

    pub fn render(&self, now: u64) -> String {
        let Some(quota) = &self.quota else {
            let state = match self.state {
                State::LoggedOut => "未登录",
                State::Unsupported => "非订阅账号",
                State::AccountUnknown | State::RefreshFailed => "读取失败",
                _ => "读取中",
            };
            return format!("Codex 剩余 │ 5h 未知 │ 1w 未知 │ [{state}]");
        };
        let expired = [&quota.five_hour, &quota.weekly]
            .iter()
            .any(|w| w.resets_at.is_some_and(|reset| reset <= now));
        let stale = !matches!(self.state, State::Ready)
            || self
                .fetched_at
                .is_none_or(|at| now < at || now.saturating_sub(at) >= REFRESH_SECONDS)
            || expired;
        let mut text = format!(
            "Codex 剩余 │ 5h {} │ 1w {}",
            render_window(&quota.five_hour, now),
            render_window(&quota.weekly, now)
        );
        if stale {
            text.push_str(" │ [过期]");
        }
        if quota.usage_allowed == Some(false) {
            text.push_str(" │ [暂不可用]");
        }
        text
    }
}

fn render_window(window: &Window, now: u64) -> String {
    let bar = match window.remaining {
        Some(percent) => {
            // Keep nonzero remainder visible without making <100% look full.
            let filled = if percent == 100 {
                6
            } else {
                (usize::from(percent) * 6 / 100).clamp(usize::from(percent > 0), 5)
            };
            format!(
                "[{}{}] {percent}%",
                "█".repeat(filled),
                "░".repeat(6 - filled)
            )
        }
        None => "[??????] 未知".to_owned(),
    };
    let countdown = match window.resets_at {
        None => "重置未知".to_owned(),
        Some(reset) if reset <= now => "待刷新".to_owned(),
        Some(reset) => {
            let minutes = (reset - now).div_ceil(60);
            if minutes >= 1440 {
                format!("↻{}d{:02}h", minutes / 1440, minutes % 1440 / 60)
            } else if minutes >= 60 {
                format!("↻{}h{:02}m", minutes / 60, minutes % 60)
            } else {
                format!("↻{minutes}m")
            }
        }
    };
    format!("{bar} {countdown}")
}

pub fn normalize(response: &Value) -> Result<Quota, &'static str> {
    let bucket = if let Some(map) = response
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
        .filter(|m| !m.is_empty())
    {
        // Only the subscription Codex bucket is in scope, never model-specific buckets.
        map.get("codex").ok_or("ambiguous quota bucket")?
    } else {
        let legacy = response
            .get("rateLimits")
            .filter(|v| v.is_object())
            .ok_or("missing quota")?;
        if legacy
            .get("limitId")
            .and_then(Value::as_str)
            .is_some_and(|id| id != "codex")
        {
            return Err("unsupported quota bucket");
        }
        legacy
    };
    fn window(bucket: &Value, duration: u64) -> Window {
        let matches: Vec<_> = ["primary", "secondary"]
            .iter()
            .filter_map(|key| bucket.get(key))
            .filter(|w| w.get("windowDurationMins").and_then(Value::as_u64) == Some(duration))
            .collect();
        if matches.len() != 1 {
            return Window::default();
        }
        let value = matches[0];
        Window {
            remaining: value
                .get("usedPercent")
                .and_then(Value::as_u64)
                .filter(|used| *used <= 100)
                .map(|used| (100 - used) as u8),
            resets_at: value
                .get("resetsAt")
                .and_then(Value::as_u64)
                .filter(|at| *at > 0),
        }
    }
    Ok(Quota {
        five_hour: window(bucket, 300),
        weekly: window(bucket, 10080),
        usage_allowed: response
            .get("ordinaryUsageAllowed")
            .and_then(Value::as_bool),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn windows_are_identified_by_duration_not_position() {
        let q = normalize(&json!({"rateLimits": {
            "primary": {"windowDurationMins":10080,"usedPercent":67},
            "secondary": {"windowDurationMins":300,"usedPercent":28,"resetsAt":200}
        }}))
        .unwrap();
        assert_eq!(q.five_hour.remaining, Some(72));
        assert_eq!(q.weekly.remaining, Some(33));
        assert_eq!(q.weekly.resets_at, None);
    }

    #[test]
    fn missing_invalid_and_ambiguous_windows_stay_unknown() {
        for used in [json!(-1), json!(101), json!(2.5), Value::Null] {
            let q = normalize(
                &json!({"rateLimits":{"primary":{"windowDurationMins":300,"usedPercent":used}}}),
            )
            .unwrap();
            assert_eq!(q.five_hour.remaining, None);
            assert_eq!(q.weekly.remaining, None);
        }
        let q = normalize(&json!({"rateLimits": {
            "primary":{"windowDurationMins":300,"usedPercent":10},
            "secondary":{"windowDurationMins":300,"usedPercent":20}
        }}))
        .unwrap();
        assert_eq!(q.five_hour.remaining, None);
    }

    #[test]
    fn unrelated_buckets_are_never_used() {
        assert!(normalize(&json!({"rateLimitsByLimitId":{"other":{}},"rateLimits":{}})).is_err());
        assert!(normalize(&json!({"rateLimits":{"limitId":"other"}})).is_err());
    }

    #[test]
    fn reset_does_not_refill_and_only_triggers_one_attempt_per_reset() {
        let cache = Cache {
            attempted_at: 100,
            fetched_at: Some(100),
            state: State::Ready,
            quota: Some(Quota {
                five_hour: Window {
                    remaining: Some(7),
                    resets_at: Some(110),
                },
                ..Quota::default()
            }),
            ..Cache::default()
        };
        assert!(cache.due(110));
        let text = cache.render(110);
        assert!(text.contains("7% 待刷新"));
        assert!(text.contains("过期"));
        assert!(!text.contains("100%"));
        let attempted = Cache {
            attempted_at: 110,
            ..cache
        };
        assert!(!attempted.due(111));
        assert!(attempted.due(170));
    }

    #[test]
    fn failed_refresh_and_old_data_are_marked_stale() {
        let mut cache = Cache {
            attempted_at: 100,
            fetched_at: Some(100),
            state: State::Ready,
            quota: Some(Quota::default()),
            ..Cache::default()
        };
        assert!(!cache.render(159).contains("过期"));
        assert!(cache.render(160).contains("过期"));
        cache.state = State::RefreshFailed;
        assert!(cache.render(101).contains("过期"));
        assert!(cache.render(101).contains("重置未知"));
    }
}
