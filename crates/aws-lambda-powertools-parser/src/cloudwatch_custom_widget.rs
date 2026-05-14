//! Amazon `CloudWatch` dashboard custom widget event models.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Amazon `CloudWatch` dashboard custom widget event.
///
/// `CloudWatch` sends this event shape when a dashboard custom widget invokes a
/// Lambda function.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchDashboardCustomWidgetEvent {
    /// Whether `CloudWatch` is requesting widget documentation.
    #[serde(default)]
    pub describe: bool,
    /// Dashboard context for a normal widget rendering request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_context: Option<CloudWatchWidgetContext>,
}

/// Parser model alias for Amazon `CloudWatch` dashboard custom widget events.
pub type CloudWatchDashboardCustomWidgetModel = CloudWatchDashboardCustomWidgetEvent;

/// Dashboard context for a `CloudWatch` dashboard custom widget request.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchWidgetContext {
    /// Dashboard name containing the widget.
    pub dashboard_name: String,
    /// Dashboard widget identifier.
    pub widget_id: String,
    /// AWS domain name used by the dashboard.
    pub domain: String,
    /// AWS account ID of the dashboard.
    pub account_id: String,
    /// Dashboard locale.
    pub locale: String,
    /// Dashboard time zone information.
    pub timezone: CloudWatchWidgetTimeZone,
    /// Period shown on the dashboard.
    pub period: u32,
    /// Whether automatic period selection is enabled.
    pub is_auto_period: bool,
    /// Time range selected for the widget.
    pub time_range: CloudWatchWidgetTimeRange,
    /// Dashboard theme.
    pub theme: String,
    /// Whether the widget is linked to other dashboard charts.
    pub link_charts: bool,
    /// Widget title.
    pub title: String,
    /// Custom widget parameters.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub params: Map<String, Value>,
    /// Custom widget form submissions.
    #[serde(default, skip_serializing_if = "CloudWatchWidgetForms::is_empty")]
    pub forms: CloudWatchWidgetForms,
    /// Widget height.
    pub height: u32,
    /// Widget width.
    pub width: u32,
}

/// Dashboard time zone information.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchWidgetTimeZone {
    /// Time zone label, such as `UTC` or `Local`.
    pub label: String,
    /// Time zone offset in ISO format.
    #[serde(rename = "offsetISO")]
    pub offset_iso: String,
    /// Time zone offset in minutes.
    pub offset_in_minutes: i32,
}

/// Dashboard widget time range.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchWidgetTimeRange {
    /// Time range mode, such as `relative` or `absolute`.
    pub mode: String,
    /// Start time for the range.
    pub start: i64,
    /// End time for the range.
    pub end: i64,
    /// Relative start offset, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_start: Option<i64>,
    /// Zoomed time range, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<CloudWatchWidgetZoom>,
}

/// Dashboard widget zoom range.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchWidgetZoom {
    /// Zoom start time.
    pub start: i64,
    /// Zoom end time.
    pub end: i64,
}

/// Dashboard custom widget form values.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchWidgetForms {
    /// Values from all widget forms.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub all: Map<String, Value>,
}

impl CloudWatchWidgetForms {
    /// Returns true when no form values are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::CloudWatchDashboardCustomWidgetEvent;

    #[test]
    fn parses_custom_widget_context() {
        let event: CloudWatchDashboardCustomWidgetEvent = serde_json::from_value(json!({
            "describe": false,
            "widgetContext": {
                "dashboardName": "Orders",
                "widgetId": "orders-widget",
                "domain": "amazonaws.com",
                "accountId": "123456789012",
                "locale": "en",
                "timezone": {
                    "label": "UTC",
                    "offsetISO": "+00:00",
                    "offsetInMinutes": 0
                },
                "period": 300,
                "isAutoPeriod": true,
                "timeRange": {
                    "mode": "relative",
                    "start": 1_767_225_300_000_i64,
                    "end": 1_767_225_600_000_i64,
                    "relativeStart": -300_000,
                    "zoom": {
                        "start": 1_767_225_420_000_i64,
                        "end": 1_767_225_540_000_i64
                    }
                },
                "theme": "dark",
                "linkCharts": false,
                "title": "Orders",
                "params": {
                    "service": "orders"
                },
                "forms": {
                    "all": {
                        "region": "us-west-2"
                    }
                },
                "height": 6,
                "width": 12
            }
        }))
        .expect("custom widget event should deserialize");

        let context = event
            .widget_context
            .expect("widget context should be present");

        assert!(!event.describe);
        assert_eq!(context.dashboard_name, "Orders");
        assert_eq!(context.timezone.offset_iso, "+00:00");
        assert_eq!(
            context
                .params
                .get("service")
                .and_then(|value| value.as_str()),
            Some("orders")
        );
        assert_eq!(
            context
                .forms
                .all
                .get("region")
                .and_then(|value| value.as_str()),
            Some("us-west-2")
        );
    }

    #[test]
    fn parses_describe_request_without_context() {
        let event: CloudWatchDashboardCustomWidgetEvent =
            serde_json::from_value(json!({ "describe": true }))
                .expect("describe request should deserialize");

        assert!(event.describe);
        assert!(event.widget_context.is_none());
    }
}
