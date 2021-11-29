// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cloud_util::common::read_toml;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct StorageConfig {
    pub storage_port: u16,

    pub kms_port: u16,

    pub db_path: String,

    pub log_file: String,

    pub write_buffer_size: usize,

    pub background_jobs: i32,

    pub max_open_file: i32,

    pub target_file_size_base: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_port: 50003,
            kms_port: 50005,
            db_path: "chain_data".to_string(),
            log_file: "storage-log4rs.yaml".to_string(),
            write_buffer_size: 4 * 64 * 1024 * 1024,
            background_jobs: 2,
            max_open_file: 512,
            target_file_size_base: 64 * 1024 * 1024,
        }
    }
}

impl StorageConfig {
    pub fn new(config_str: &str) -> Self {
        read_toml(config_str, "storage_sled")
    }
}

#[cfg(test)]
mod tests {
    use super::StorageConfig;

    #[test]
    fn basic_test() {
        let config = StorageConfig::new("example/config.toml");

        assert_eq!(config.kms_port, 60005);
        assert_eq!(config.storage_port, 60003);
        assert_eq!(config.write_buffer_size, 65536);
        assert_eq!(config.max_open_file, 65535);
    }
}
