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

mod config;
mod db;
mod util;

use clap::Clap;
use git_version::git_version;
use log::{debug, info, warn};

const GIT_VERSION: &str = git_version!(
    args = ["--tags", "--always", "--dirty=-modified"],
    fallback = "unknown"
);
const GIT_HOMEPAGE: &str = "https://github.com/cita-cloud/storage_sled";

/// network service
#[derive(Clap)]
#[clap(version = "0.1.0", author = "Rivtower Technologies.")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// print information from git
    #[clap(name = "git")]
    GitInfo,
    /// run this service
    #[clap(name = "run")]
    Run(RunOpts),
}

/// A subcommand for run
#[derive(Clap)]
struct RunOpts {
    /// Sets grpc port of this service.
    #[clap(short = 'p', long = "port", default_value = "50003")]
    grpc_port: String,
    /// Sets db path.
    #[clap(short = 'd', long = "db")]
    db_path: Option<String>,
    /// Chain config path
    #[clap(short = 'c', long = "config", default_value = "config.toml")]
    config_path: String,
}

fn main() {
    ::std::env::set_var("RUST_BACKTRACE", "full");

    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::GitInfo => {
            println!("git version: {}", GIT_VERSION);
            println!("homepage: {}", GIT_HOMEPAGE);
        }
        SubCommand::Run(opts) => {
            let fin = run(opts);
            warn!("Should not reach here {:?}", fin);
        }
    }
}

use crate::config::StorageConfig;
use crate::util::init_grpc_client;
use cita_cloud_proto::storage::{
    storage_service_server::StorageService, storage_service_server::StorageServiceServer, Content,
    ExtKey, Value,
};
use db::DB;
use status_code::StatusCode;
use std::net::AddrParseError;
use tonic::{transport::Server, Request, Response, Status};

pub struct StorageServer {
    db: DB,
}

impl StorageServer {
    fn new(db: DB) -> Self {
        StorageServer { db }
    }
}

#[tonic::async_trait]
impl StorageService for StorageServer {
    async fn store(
        &self,
        request: Request<Content>,
    ) -> Result<Response<cita_cloud_proto::common::StatusCode>, Status> {
        debug!("store request: {:?}", request);

        let content = request.into_inner();
        let region = content.region;
        let key = content.key;
        let value = content.value;

        if region == 11 {
            match self.db.store_full_block(key, value).await {
                Ok(()) => Ok(Response::new(StatusCode::Success.into())),
                Err(status) => {
                    warn!("store_full_block failed: {}", status.to_string());
                    Ok(Response::new(status.into()))
                }
            }
        } else {
            match self.db.store(region, key, value) {
                Ok(()) => Ok(Response::new(StatusCode::Success.into())),
                Err(status) => {
                    warn!("store failed: {}", status.to_string());
                    Ok(Response::new(status.into()))
                }
            }
        }
    }

    async fn load(&self, request: Request<ExtKey>) -> Result<Response<Value>, Status> {
        debug!("load request: {:?}", request);

        let ext_key = request.into_inner();
        let region = ext_key.region;
        let key = ext_key.key;

        if region == 11 {
            match self.db.load_full_block(key) {
                Ok(value) => Ok(Response::new(Value {
                    status: Some(StatusCode::Success.into()),
                    value,
                })),
                Err(status) => {
                    warn!("load_full_block failed: {}", status.to_string());
                    Ok(Response::new(Value {
                        status: Some(status.into()),
                        value: vec![],
                    }))
                }
            }
        } else {
            match self.db.load(region, key) {
                Ok(value) => Ok(Response::new(Value {
                    status: Some(StatusCode::Success.into()),
                    value,
                })),
                Err(status) => {
                    warn!("load failed: {}", status.to_string());
                    Ok(Response::new(Value {
                        status: Some(status.into()),
                        value: vec![],
                    }))
                }
            }
        }
    }

    async fn delete(
        &self,
        request: Request<ExtKey>,
    ) -> Result<Response<cita_cloud_proto::common::StatusCode>, Status> {
        debug!("delete request: {:?}", request);

        let ext_key = request.into_inner();
        let region = ext_key.region;
        let key = ext_key.key;

        match self.db.delete(region, key) {
            Ok(()) => Ok(Response::new(StatusCode::Success.into())),
            Err(status) => {
                warn!("delete error: {}", status.to_string());
                Ok(Response::new(status.into()))
            }
        }
    }
}

#[tokio::main]
async fn run(opts: RunOpts) -> Result<(), StatusCode> {
    let config = StorageConfig::new(&opts.config_path);
    init_grpc_client(&config);
    // init log4rs
    log4rs::init_file(&config.log_file, Default::default()).unwrap();

    let grpc_port = {
        if "50003" != opts.grpc_port {
            opts.grpc_port.clone()
        } else if config.storage_port != 50003 {
            config.storage_port.to_string()
        } else {
            "50003".to_string()
        }
    };
    info!("grpc port of this service: {}", grpc_port);

    let db_path = match opts.db_path {
        Some(path) => path,
        None => config.db_path.clone(),
    };
    info!("db path of this service: {}", &db_path);

    let addr_str = format!("127.0.0.1:{}", grpc_port);
    let addr = addr_str.parse().map_err(|e: AddrParseError| {
        warn!("grpc listen addr parse failed: {} ", e.to_string());
        StatusCode::FatalError
    })?;

    // init db
    let db = DB::new(&db_path, &config);
    let storage_server = StorageServer::new(db);

    Server::builder()
        .add_service(StorageServiceServer::new(storage_server))
        .serve(addr)
        .await
        .map_err(|e| {
            warn!("start controller grpc server failed: {} ", e.to_string());
            StatusCode::FatalError
        })?;

    Ok(())
}
