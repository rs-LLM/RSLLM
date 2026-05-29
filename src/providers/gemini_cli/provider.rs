use crate::domain::dto::ProviderConfig;
use crate::providers::common::provider::CommonProvider;
use crate::providers::provider::Provider;

pub fn build(config: &ProviderConfig) -> CommonProvider {
    <CommonProvider as Provider>::new(config)
}
