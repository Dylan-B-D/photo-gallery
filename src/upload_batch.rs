use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct UploadBatchConfig {
    pub max_bytes: usize,
    pub max_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct UploadBatchConfigs {
    pub local: UploadBatchConfig,
    pub public: UploadBatchConfig,
}

pub fn local_upload_batch_config() -> UploadBatchConfig {
    UploadBatchConfig {
        max_bytes: 350 * 1024 * 1024,
        max_count: 120,
    }
}

pub fn public_upload_batch_config() -> UploadBatchConfig {
    UploadBatchConfig {
        max_bytes: 20 * 1024 * 1024,
        max_count: 4,
    }
}

pub fn upload_batch_configs() -> UploadBatchConfigs {
    UploadBatchConfigs {
        local: local_upload_batch_config(),
        public: public_upload_batch_config(),
    }
}

pub fn upload_batch_configs_json() -> String {
    serde_json::to_string(&upload_batch_configs()).expect("upload batch config serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_batches_are_small_enough_for_proxied_image_processing() {
        let config = public_upload_batch_config();

        assert!(config.max_count <= 10);
        assert!(config.max_bytes <= 30 * 1024 * 1024);
    }

    #[test]
    fn local_batches_can_remain_larger_for_development_uploads() {
        let config = local_upload_batch_config();

        assert!(config.max_count > public_upload_batch_config().max_count);
        assert!(config.max_bytes > public_upload_batch_config().max_bytes);
    }
}
