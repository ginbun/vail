use sqlx::PgPool;

use crate::{
    domain::orion::host::OrionHostAggregate,
    infrastructure::orion::host_repository::{
        self, OrionHostQueryFilters as RepositoryHostQueryFilters,
    },
};

#[derive(Debug, Clone, Default)]
pub struct OrionHostQueryFilters {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub search_value: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_hosts(pool: &PgPool) -> Result<Vec<OrionHostAggregate>, sqlx::Error> {
    host_repository::list_hosts(pool).await
}

pub async fn get_host_by_id(
    pool: &PgPool,
    host_id: i64,
) -> Result<Option<OrionHostAggregate>, sqlx::Error> {
    host_repository::get_host_by_id(pool, host_id).await
}

pub async fn query_hosts(
    pool: &PgPool,
    filters: OrionHostQueryFilters,
) -> Result<Vec<OrionHostAggregate>, sqlx::Error> {
    host_repository::query_hosts(pool, &to_repository_filters(&filters)).await
}

pub async fn count_hosts(
    pool: &PgPool,
    filters: OrionHostQueryFilters,
) -> Result<i64, sqlx::Error> {
    host_repository::count_hosts(pool, &to_repository_filters(&filters)).await
}

fn to_repository_filters(filters: &OrionHostQueryFilters) -> RepositoryHostQueryFilters {
    RepositoryHostQueryFilters {
        id: filters.id,
        name: filters.name.clone(),
        address: filters.address.clone(),
        search_value: filters.search_value.clone(),
        status: filters.status.clone(),
        limit: filters.limit,
        offset: filters.offset,
    }
}

#[cfg(test)]
mod tests {
    use super::{to_repository_filters, OrionHostQueryFilters};

    #[test]
    fn converts_host_query_filters_to_repository_type() {
        let filters = OrionHostQueryFilters {
            id: Some(7),
            name: Some("prod".to_string()),
            address: Some("10.0".to_string()),
            search_value: Some("bastion".to_string()),
            status: Some("ENABLED".to_string()),
            limit: Some(50),
            offset: Some(100),
        };

        let mapped = to_repository_filters(&filters);
        assert_eq!(mapped.id, Some(7));
        assert_eq!(mapped.name.as_deref(), Some("prod"));
        assert_eq!(mapped.address.as_deref(), Some("10.0"));
        assert_eq!(mapped.search_value.as_deref(), Some("bastion"));
        assert_eq!(mapped.status.as_deref(), Some("ENABLED"));
        assert_eq!(mapped.limit, Some(50));
        assert_eq!(mapped.offset, Some(100));
    }
}
