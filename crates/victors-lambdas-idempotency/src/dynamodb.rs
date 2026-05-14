//! Amazon `DynamoDB` idempotency store.

use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aws_sdk_dynamodb::{
    Client,
    operation::{delete_item::DeleteItemOutput, get_item::GetItemOutput, put_item::PutItemOutput},
    primitives::Blob,
    types::{AttributeValue, ReturnValue, ReturnValuesOnConditionCheckFailure},
};

use crate::{
    AsyncIdempotencyStore, IdempotencyKey, IdempotencyRecord, IdempotencyStatus,
    IdempotencyStoreError, IdempotencyStoreFuture, IdempotencyStoreResult,
};

const DEFAULT_KEY_ATTR: &str = "id";
const DEFAULT_STATIC_PK_VALUE: &str = "idempotency";
const DEFAULT_EXPIRY_ATTR: &str = "expiration";
const DEFAULT_STATUS_ATTR: &str = "status";
const DEFAULT_DATA_ATTR: &str = "data";
const DEFAULT_VALIDATION_ATTR: &str = "validation";
const STATUS_IN_PROGRESS: &str = "INPROGRESS";
const STATUS_COMPLETED: &str = "COMPLETED";

/// Asynchronous idempotency store backed by an Amazon `DynamoDB` table.
///
/// The default table layout uses a single string partition key attribute named
/// `id`. For composite-key tables, configure a sort key attribute; the
/// partition key value then defaults to `idempotency` and can be overridden with
/// [`Self::with_static_pk_value`].
#[derive(Clone, Debug)]
pub struct DynamoDbIdempotencyStore {
    client: Client,
    table_name: String,
    key_attr: String,
    sort_key_attr: Option<String>,
    static_pk_value: String,
    expiry_attr: String,
    status_attr: String,
    data_attr: String,
    validation_attr: String,
}

impl DynamoDbIdempotencyStore {
    /// Creates a `DynamoDB` idempotency store with default attribute names.
    ///
    /// The store accepts a client instead of constructing one internally so
    /// Lambda handlers can choose how they load AWS SDK configuration and so
    /// this crate does not force an `aws-config` dependency on all users.
    #[must_use]
    pub fn new(client: Client, table_name: impl Into<String>) -> Self {
        Self {
            client,
            table_name: table_name.into(),
            key_attr: DEFAULT_KEY_ATTR.to_owned(),
            sort_key_attr: None,
            static_pk_value: DEFAULT_STATIC_PK_VALUE.to_owned(),
            expiry_attr: DEFAULT_EXPIRY_ATTR.to_owned(),
            status_attr: DEFAULT_STATUS_ATTR.to_owned(),
            data_attr: DEFAULT_DATA_ATTR.to_owned(),
            validation_attr: DEFAULT_VALIDATION_ATTR.to_owned(),
        }
    }

    /// Returns a copy of the store with a custom partition key attribute.
    ///
    /// # Panics
    ///
    /// Panics when the partition key attribute matches the configured sort key
    /// attribute.
    #[must_use]
    pub fn with_key_attr(mut self, key_attr: impl Into<String>) -> Self {
        let key_attr = key_attr.into();
        assert_ne!(
            self.sort_key_attr.as_deref(),
            Some(key_attr.as_str()),
            "DynamoDB idempotency key_attr and sort_key_attr cannot match"
        );
        self.key_attr = key_attr;
        self
    }

    /// Returns a copy of the store with a custom sort key attribute.
    ///
    /// When set, the idempotency key is stored in this sort key attribute and
    /// the partition key uses the configured static partition value.
    ///
    /// # Panics
    ///
    /// Panics when the sort key attribute matches the partition key attribute.
    #[must_use]
    pub fn with_sort_key_attr(mut self, sort_key_attr: impl Into<String>) -> Self {
        let sort_key_attr = sort_key_attr.into();
        assert_ne!(
            self.key_attr, sort_key_attr,
            "DynamoDB idempotency key_attr and sort_key_attr cannot match"
        );
        self.sort_key_attr = Some(sort_key_attr);
        self
    }

    /// Returns a copy of the store without a sort key attribute.
    #[must_use]
    pub fn without_sort_key_attr(mut self) -> Self {
        self.sort_key_attr = None;
        self
    }

    /// Returns a copy of the store with a custom static partition key value.
    #[must_use]
    pub fn with_static_pk_value(mut self, static_pk_value: impl Into<String>) -> Self {
        self.static_pk_value = static_pk_value.into();
        self
    }

    /// Returns a copy of the store with a custom expiry timestamp attribute.
    #[must_use]
    pub fn with_expiry_attr(mut self, expiry_attr: impl Into<String>) -> Self {
        self.expiry_attr = expiry_attr.into();
        self
    }

    /// Returns a copy of the store with a custom status attribute.
    #[must_use]
    pub fn with_status_attr(mut self, status_attr: impl Into<String>) -> Self {
        self.status_attr = status_attr.into();
        self
    }

    /// Returns a copy of the store with a custom response data attribute.
    #[must_use]
    pub fn with_data_attr(mut self, data_attr: impl Into<String>) -> Self {
        self.data_attr = data_attr.into();
        self
    }

    /// Returns a copy of the store with a custom payload validation attribute.
    #[must_use]
    pub fn with_validation_attr(mut self, validation_attr: impl Into<String>) -> Self {
        self.validation_attr = validation_attr.into();
        self
    }

    /// Returns the underlying AWS SDK client.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the `DynamoDB` table name.
    #[must_use]
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Returns the partition key attribute name.
    #[must_use]
    pub fn key_attr(&self) -> &str {
        &self.key_attr
    }

    /// Returns the optional sort key attribute name.
    #[must_use]
    pub fn sort_key_attr(&self) -> Option<&str> {
        self.sort_key_attr.as_deref()
    }

    /// Returns the static partition key value used with a sort key.
    #[must_use]
    pub fn static_pk_value(&self) -> &str {
        &self.static_pk_value
    }

    /// Returns the expiry timestamp attribute name.
    #[must_use]
    pub fn expiry_attr(&self) -> &str {
        &self.expiry_attr
    }

    /// Returns the status attribute name.
    #[must_use]
    pub fn status_attr(&self) -> &str {
        &self.status_attr
    }

    /// Returns the response data attribute name.
    #[must_use]
    pub fn data_attr(&self) -> &str {
        &self.data_attr
    }

    /// Returns the payload validation attribute name.
    #[must_use]
    pub fn validation_attr(&self) -> &str {
        &self.validation_attr
    }

    async fn fetch_record(
        &self,
        key: &IdempotencyKey,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let output = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .set_key(Some(self.key_item(key)))
            .consistent_read(true)
            .send()
            .await
            .map_err(|error| IdempotencyStoreError::new(error.to_string()))?;

        record_from_get_output(&output, self)
    }

    async fn write_record(
        &self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let mut request = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(self.record_item(&record)?))
            .return_values(ReturnValue::AllOld);

        if record.status().is_in_progress() {
            let now = epoch_seconds(SystemTime::now())?;
            request = request
                .condition_expression("attribute_not_exists(#id) OR #expiry <= :now")
                .expression_attribute_names("#id", &self.key_attr)
                .expression_attribute_names("#expiry", &self.expiry_attr)
                .expression_attribute_values(":now", AttributeValue::N(now.to_string()))
                .return_values_on_condition_check_failure(
                    ReturnValuesOnConditionCheckFailure::AllOld,
                );
        }

        let output = request
            .send()
            .await
            .map_err(|error| IdempotencyStoreError::new(error.to_string()))?;

        record_from_put_output(&output, self)
    }

    async fn delete_record(
        &self,
        key: &IdempotencyKey,
    ) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
        let output = self
            .client
            .delete_item()
            .table_name(&self.table_name)
            .set_key(Some(self.key_item(key)))
            .return_values(ReturnValue::AllOld)
            .send()
            .await
            .map_err(|error| IdempotencyStoreError::new(error.to_string()))?;

        record_from_delete_output(&output, self)
    }

    async fn delete_expired_records(&self, now: SystemTime) -> IdempotencyStoreResult<usize> {
        let now = epoch_seconds(now)?;
        let mut exclusive_start_key = None;
        let mut removed = 0;

        loop {
            let mut request = self
                .client
                .scan()
                .table_name(&self.table_name)
                .filter_expression(self.clear_expired_filter())
                .expression_attribute_names("#id", &self.key_attr)
                .expression_attribute_names("#expiry", &self.expiry_attr)
                .expression_attribute_values(":now", AttributeValue::N(now.to_string()))
                .projection_expression(self.clear_expired_projection());

            if let Some(sort_key_attr) = &self.sort_key_attr {
                request = request
                    .expression_attribute_names("#sort", sort_key_attr)
                    .expression_attribute_values(
                        ":static_pk",
                        AttributeValue::S(self.static_pk_value.clone()),
                    );
            }

            let output = request
                .set_exclusive_start_key(exclusive_start_key.take())
                .send()
                .await
                .map_err(|error| IdempotencyStoreError::new(error.to_string()))?;

            for item in output.items() {
                if let Some(key) = self.key_from_item(item) {
                    if self.delete_record(&key).await?.is_some() {
                        removed += 1;
                    }
                }
            }

            exclusive_start_key = output.last_evaluated_key().cloned();
            if exclusive_start_key.is_none() {
                break;
            }
        }

        Ok(removed)
    }

    fn key_item(&self, key: &IdempotencyKey) -> HashMap<String, AttributeValue> {
        match &self.sort_key_attr {
            Some(sort_key_attr) => HashMap::from([
                (
                    self.key_attr.clone(),
                    AttributeValue::S(self.static_pk_value.clone()),
                ),
                (
                    sort_key_attr.clone(),
                    AttributeValue::S(key.value().to_owned()),
                ),
            ]),
            None => HashMap::from([(
                self.key_attr.clone(),
                AttributeValue::S(key.value().to_owned()),
            )]),
        }
    }

    fn key_from_item(&self, item: &HashMap<String, AttributeValue>) -> Option<IdempotencyKey> {
        match &self.sort_key_attr {
            Some(sort_key_attr) => {
                if item
                    .get(&self.key_attr)
                    .and_then(string_attribute)
                    .as_deref()
                    != Some(self.static_pk_value.as_str())
                {
                    return None;
                }

                item.get(sort_key_attr)
                    .and_then(string_attribute)
                    .map(IdempotencyKey::new)
            }
            None => item
                .get(&self.key_attr)
                .and_then(string_attribute)
                .map(IdempotencyKey::new),
        }
    }

    fn record_item(
        &self,
        record: &IdempotencyRecord,
    ) -> IdempotencyStoreResult<HashMap<String, AttributeValue>> {
        let mut item = self.key_item(record.key());
        item.insert(
            self.expiry_attr.clone(),
            AttributeValue::N(epoch_seconds(record.expires_at())?.to_string()),
        );
        item.insert(
            self.status_attr.clone(),
            AttributeValue::S(status_to_attribute(record.status()).to_owned()),
        );

        if let Some(payload_hash) = record.payload_hash() {
            item.insert(
                self.validation_attr.clone(),
                AttributeValue::S(payload_hash.to_owned()),
            );
        }

        if let Some(response_data) = record.response_data() {
            item.insert(
                self.data_attr.clone(),
                AttributeValue::B(Blob::new(response_data.to_vec())),
            );
        }

        Ok(item)
    }

    fn clear_expired_projection(&self) -> &'static str {
        if self.sort_key_attr.is_some() {
            "#id, #sort, #expiry"
        } else {
            "#id, #expiry"
        }
    }

    fn clear_expired_filter(&self) -> &'static str {
        if self.sort_key_attr.is_some() {
            "#expiry <= :now AND #id = :static_pk"
        } else {
            "#expiry <= :now"
        }
    }
}

impl AsyncIdempotencyStore for DynamoDbIdempotencyStore {
    fn get<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
        Box::pin(async move { self.fetch_record(key).await })
    }

    fn put(
        &self,
        record: IdempotencyRecord,
    ) -> IdempotencyStoreFuture<'_, Option<IdempotencyRecord>> {
        Box::pin(async move { self.write_record(record).await })
    }

    fn remove<'a>(
        &'a self,
        key: &'a IdempotencyKey,
    ) -> IdempotencyStoreFuture<'a, Option<IdempotencyRecord>> {
        Box::pin(async move { self.delete_record(key).await })
    }

    fn clear_expired(&self, now: SystemTime) -> IdempotencyStoreFuture<'_, usize> {
        Box::pin(async move { self.delete_expired_records(now).await })
    }
}

fn record_from_get_output(
    output: &GetItemOutput,
    store: &DynamoDbIdempotencyStore,
) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
    output
        .item()
        .map(|item| record_from_item(item, store))
        .transpose()
}

fn record_from_put_output(
    output: &PutItemOutput,
    store: &DynamoDbIdempotencyStore,
) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
    output
        .attributes()
        .map(|item| record_from_item(item, store))
        .transpose()
}

fn record_from_delete_output(
    output: &DeleteItemOutput,
    store: &DynamoDbIdempotencyStore,
) -> IdempotencyStoreResult<Option<IdempotencyRecord>> {
    output
        .attributes()
        .map(|item| record_from_item(item, store))
        .transpose()
}

#[cfg(test)]
fn records_from_scan_output(
    output: &aws_sdk_dynamodb::operation::scan::ScanOutput,
    store: &DynamoDbIdempotencyStore,
) -> IdempotencyStoreResult<Vec<IdempotencyRecord>> {
    output
        .items()
        .iter()
        .map(|item| record_from_item(item, store))
        .collect()
}

fn record_from_item(
    item: &HashMap<String, AttributeValue>,
    store: &DynamoDbIdempotencyStore,
) -> IdempotencyStoreResult<IdempotencyRecord> {
    let key = store.key_from_item(item).ok_or_else(|| {
        IdempotencyStoreError::new(format!(
            "DynamoDB item is missing idempotency key attribute {}",
            store
                .sort_key_attr
                .as_deref()
                .unwrap_or_else(|| store.key_attr())
        ))
    })?;
    let expires_at = item
        .get(&store.expiry_attr)
        .and_then(number_attribute)
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds))
        .ok_or_else(|| {
            IdempotencyStoreError::new(format!(
                "DynamoDB item is missing numeric expiry attribute {}",
                store.expiry_attr
            ))
        })?;
    let status = item
        .get(&store.status_attr)
        .and_then(string_attribute)
        .and_then(|status| status_from_attribute(&status))
        .ok_or_else(|| {
            IdempotencyStoreError::new(format!(
                "DynamoDB item is missing valid status attribute {}",
                store.status_attr
            ))
        })?;

    let mut record = match status {
        IdempotencyStatus::InProgress => IdempotencyRecord::in_progress_until(key, expires_at),
        IdempotencyStatus::Completed => IdempotencyRecord::completed_until(key, expires_at),
        IdempotencyStatus::Expired => {
            return Err(IdempotencyStoreError::new(
                "DynamoDB idempotency items cannot store EXPIRED status",
            ));
        }
    };

    if let Some(payload_hash) = item.get(&store.validation_attr).and_then(string_attribute) {
        record = record.with_payload_hash(payload_hash);
    }

    if let Some(response_data) = item.get(&store.data_attr).and_then(response_data_attribute) {
        record = record.with_response_data(response_data);
    }

    Ok(record)
}

fn status_to_attribute(status: IdempotencyStatus) -> &'static str {
    match status {
        IdempotencyStatus::InProgress => STATUS_IN_PROGRESS,
        IdempotencyStatus::Completed => STATUS_COMPLETED,
        IdempotencyStatus::Expired => "EXPIRED",
    }
}

fn status_from_attribute(status: &str) -> Option<IdempotencyStatus> {
    match status {
        STATUS_IN_PROGRESS => Some(IdempotencyStatus::InProgress),
        STATUS_COMPLETED => Some(IdempotencyStatus::Completed),
        _ => None,
    }
}

fn string_attribute(attribute: &AttributeValue) -> Option<String> {
    match attribute {
        AttributeValue::S(value) => Some(value.clone()),
        _ => None,
    }
}

fn number_attribute(attribute: &AttributeValue) -> Option<&str> {
    match attribute {
        AttributeValue::N(value) => Some(value),
        _ => None,
    }
}

fn response_data_attribute(attribute: &AttributeValue) -> Option<Vec<u8>> {
    match attribute {
        AttributeValue::B(value) => Some(value.as_ref().to_vec()),
        AttributeValue::S(value) => Some(value.as_bytes().to_vec()),
        _ => None,
    }
}

fn epoch_seconds(time: SystemTime) -> IdempotencyStoreResult<u64> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| {
            IdempotencyStoreError::new(format!(
                "idempotency record expiry is before UNIX epoch: {error}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        time::{Duration, UNIX_EPOCH},
    };

    use aws_sdk_dynamodb::{
        Client, Config,
        config::{BehaviorVersion, Credentials, Region},
        operation::{get_item::GetItemOutput, scan::ScanOutput},
        primitives::Blob,
        types::AttributeValue,
    };

    use crate::{IdempotencyKey, IdempotencyRecord, IdempotencyStatus};

    use super::{
        DynamoDbIdempotencyStore, record_from_get_output, records_from_scan_output,
        response_data_attribute,
    };

    #[test]
    fn store_uses_default_and_custom_attributes() {
        let store = DynamoDbIdempotencyStore::new(client(), "idempotency")
            .with_key_attr("pk")
            .with_sort_key_attr("sk")
            .with_static_pk_value("idempotency#orders")
            .with_expiry_attr("ttl")
            .with_status_attr("state")
            .with_data_attr("body")
            .with_validation_attr("payload");

        assert!(store.client().config().region().is_some());
        assert_eq!(store.table_name(), "idempotency");
        assert_eq!(store.key_attr(), "pk");
        assert_eq!(store.sort_key_attr(), Some("sk"));
        assert_eq!(store.static_pk_value(), "idempotency#orders");
        assert_eq!(store.expiry_attr(), "ttl");
        assert_eq!(store.status_attr(), "state");
        assert_eq!(store.data_attr(), "body");
        assert_eq!(store.validation_attr(), "payload");
    }

    #[test]
    fn record_items_round_trip_completed_record() {
        let store = DynamoDbIdempotencyStore::new(client(), "idempotency");
        let record =
            IdempotencyRecord::completed_until("order-1", UNIX_EPOCH + Duration::from_secs(60))
                .with_payload_hash("payload-hash")
                .with_response_data(br#"{"ok":true}"#.to_vec());
        let item = store.record_item(&record).expect("item should render");
        let output = GetItemOutput::builder().set_item(Some(item)).build();

        let parsed = record_from_get_output(&output, &store)
            .expect("item should parse")
            .expect("record should exist");

        assert_eq!(parsed.key().value(), "order-1");
        assert_eq!(parsed.status(), IdempotencyStatus::Completed);
        assert_eq!(parsed.expires_at(), UNIX_EPOCH + Duration::from_secs(60));
        assert_eq!(parsed.payload_hash(), Some("payload-hash"));
        assert_eq!(parsed.response_data(), Some(&br#"{"ok":true}"#[..]));
    }

    #[test]
    fn record_item_uses_static_partition_and_sort_key_when_configured() {
        let store = DynamoDbIdempotencyStore::new(client(), "idempotency")
            .with_key_attr("pk")
            .with_sort_key_attr("sk")
            .with_static_pk_value("idempotency#orders");
        let record =
            IdempotencyRecord::in_progress_until("order-1", UNIX_EPOCH + Duration::from_secs(30));
        let item = store.record_item(&record).expect("item should render");

        assert_eq!(
            item.get("pk"),
            Some(&AttributeValue::S("idempotency#orders".to_owned()))
        );
        assert_eq!(
            item.get("sk"),
            Some(&AttributeValue::S("order-1".to_owned()))
        );

        let parsed = super::record_from_item(&item, &store).expect("item should parse");
        assert_eq!(parsed.key(), &IdempotencyKey::new("order-1"));

        let mut other_partition_item = item;
        other_partition_item.insert("pk".to_owned(), AttributeValue::S("other".to_owned()));
        assert_eq!(store.key_from_item(&other_partition_item), None);
    }

    #[test]
    fn scan_output_extracts_records() {
        let store = DynamoDbIdempotencyStore::new(client(), "idempotency");
        let output = ScanOutput::builder()
            .items(HashMap::from([
                ("id".to_owned(), AttributeValue::S("order-1".to_owned())),
                ("expiration".to_owned(), AttributeValue::N("60".to_owned())),
                (
                    "status".to_owned(),
                    AttributeValue::S("INPROGRESS".to_owned()),
                ),
            ]))
            .items(HashMap::from([
                ("id".to_owned(), AttributeValue::S("order-2".to_owned())),
                ("expiration".to_owned(), AttributeValue::N("120".to_owned())),
                (
                    "status".to_owned(),
                    AttributeValue::S("COMPLETED".to_owned()),
                ),
            ]))
            .build();

        let records = records_from_scan_output(&output, &store).expect("items should parse");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].key().value(), "order-1");
        assert_eq!(records[1].status(), IdempotencyStatus::Completed);
    }

    #[test]
    fn malformed_item_returns_store_error() {
        let store = DynamoDbIdempotencyStore::new(client(), "idempotency");
        let output = GetItemOutput::builder()
            .item("id", AttributeValue::S("order-1".to_owned()))
            .item("expiration", AttributeValue::N("60".to_owned()))
            .item("status", AttributeValue::S("UNKNOWN".to_owned()))
            .build();

        let error = record_from_get_output(&output, &store).expect_err("status should fail");

        assert!(error.message().contains("status"));
    }

    #[test]
    fn response_data_accepts_binary_and_string_attributes() {
        assert_eq!(
            response_data_attribute(&AttributeValue::B(Blob::new(b"binary".to_vec()))),
            Some(b"binary".to_vec())
        );
        assert_eq!(
            response_data_attribute(&AttributeValue::S("string".to_owned())),
            Some(b"string".to_vec())
        );
    }

    fn client() -> Client {
        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "access-key",
                "secret-key",
                None,
                None,
                "test",
            ))
            .build();

        Client::from_conf(config)
    }
}
