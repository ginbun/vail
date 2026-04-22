use sqlx::PgPool;

use crate::{
    domain::orion::infra_statistics::{
        OrionChartPointAggregate, OrionInfraWorkplaceAggregate, OrionLoginHistoryAggregate,
    },
    infrastructure::orion::infra_statistics_repository,
};

pub async fn get_infra_workplace_statistics(
    pool: &PgPool,
    user_id: i64,
    unread_message_count: i64,
) -> Result<Option<OrionInfraWorkplaceAggregate>, sqlx::Error> {
    let Some(user) = infra_statistics_repository::get_infra_user_summary(pool, user_id).await?
    else {
        return Ok(None);
    };

    let user_session_count = infra_statistics_repository::count_active_ssh_sessions(pool, user_id)
        .await
        .unwrap_or(0);
    let chart_points = infra_statistics_repository::list_operator_chart_points(pool, user_id)
        .await
        .unwrap_or_default();
    let login_history = infra_statistics_repository::list_login_history(pool, user_id, 10)
        .await
        .unwrap_or_default();
    let (operator_chart_labels, operator_chart_values) = split_chart_points(chart_points);

    Ok(Some(OrionInfraWorkplaceAggregate {
        user_id,
        username: user.username,
        nickname: user.nickname,
        unread_message_count,
        last_login_time: user.last_login_time,
        user_session_count,
        operator_chart_labels,
        operator_chart_values,
        login_history,
    }))
}

fn split_chart_points(points: Vec<OrionChartPointAggregate>) -> (Vec<String>, Vec<i64>) {
    let mut labels = Vec::with_capacity(points.len());
    let mut values = Vec::with_capacity(points.len());
    for point in points {
        labels.push(point.label);
        values.push(point.count);
    }
    (labels, values)
}

pub fn fill_login_history_defaults(
    history: OrionLoginHistoryAggregate,
) -> OrionLoginHistoryAggregate {
    OrionLoginHistoryAggregate {
        id: history.id,
        address: Some(history.address.unwrap_or_default()),
        location: Some(history.location.unwrap_or_default()),
        user_agent: Some(history.user_agent.unwrap_or_default()),
        result: Some(history.result.unwrap_or(0)),
        error_message: Some(history.error_message.unwrap_or_default()),
        create_time: history.create_time,
    }
}

#[cfg(test)]
mod tests {
    use super::{fill_login_history_defaults, split_chart_points};
    use crate::domain::orion::infra_statistics::{
        OrionChartPointAggregate, OrionLoginHistoryAggregate,
    };

    #[test]
    fn split_chart_points_preserves_order() {
        let points = vec![
            OrionChartPointAggregate {
                label: "04-16".to_string(),
                count: 1,
            },
            OrionChartPointAggregate {
                label: "04-17".to_string(),
                count: 3,
            },
        ];

        let (labels, values) = split_chart_points(points);
        assert_eq!(labels, vec!["04-16", "04-17"]);
        assert_eq!(values, vec![1, 3]);
    }

    #[test]
    fn fill_login_history_defaults_replaces_none_fields() {
        let row = OrionLoginHistoryAggregate {
            id: 5,
            address: None,
            location: None,
            user_agent: None,
            result: None,
            error_message: None,
            create_time: 100,
        };

        let filled = fill_login_history_defaults(row);
        assert_eq!(filled.id, 5);
        assert_eq!(filled.address.as_deref(), Some(""));
        assert_eq!(filled.location.as_deref(), Some(""));
        assert_eq!(filled.user_agent.as_deref(), Some(""));
        assert_eq!(filled.result, Some(0));
        assert_eq!(filled.error_message.as_deref(), Some(""));
        assert_eq!(filled.create_time, 100);
    }
}
