//! AWS `AppSync` resolver model aliases.

use aws_lambda_events::event::appsync::AppSyncDirectResolverEvent;

/// AWS `AppSync` direct Lambda resolver event model.
pub type AppSyncResolverEvent<
    TArguments = serde_json::Value,
    TSource = serde_json::Value,
    TStash = serde_json::Value,
> = AppSyncDirectResolverEvent<TArguments, TSource, TStash>;

/// AWS `AppSync` batch direct Lambda resolver event model.
pub type AppSyncBatchResolverEvent<
    TArguments = serde_json::Value,
    TSource = serde_json::Value,
    TStash = serde_json::Value,
> = Vec<AppSyncDirectResolverEvent<TArguments, TSource, TStash>>;
