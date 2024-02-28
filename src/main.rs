#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(debug_assertions)]
#[derive(Clone, Parser, Debug)]
struct Args {
    #[arg(long,default_value = "8080")]
    port: u16,
    #[arg(long, num_args = 1.., value_delimiter = ' ', default_value = "rufs-nfe-rust/pkg rufs-nfe-rust/webapp")]
    static_paths: Vec<String>,
    #[arg(long, default_value = "rufs-nfe-rust/sql")]
    migration_path: String,
    #[arg(long,default_value = "true")]
    reset_db: bool
}

#[cfg(not(debug_assertions))]
#[derive(Clone, Parser, Debug)]
struct Args {
    #[arg(long,default_value = "8080")]
    port: u16,
    #[arg(long, num_args = 1.., value_delimiter = ' ', default_value = "pkg webapp")]
    static_paths: Vec<String>,
    #[arg(long, default_value = "sql")]
    migration_path: String,
    #[arg(long,default_value = "false")]
    reset_db: bool
}

#[cfg(not(target_arch = "wasm32"))]
async fn server(args: Args) -> Result<Box<tide::Server<rufs_base_rust::rufs_micro_service::RufsMicroService<'static>>>, Box<dyn std::error::Error>> {
    use std::path::Path;
    use rufs_base_rust::{rufs_micro_service::RufsMicroService, micro_service_server::MicroServiceServer, openapi::RufsOpenAPI, rufs_tide_new};
    use serde_json::Value;

    if args.reset_db {
        let path = "openapi-rufs_nfe_rust.json";

        if Path::new(path).exists() {
            std::fs::remove_file(path)?;
        }

        let (db_client, connection) = tokio_postgres::connect("postgres://development:123456@localhost:5432/rufs_nfe_development", tokio_postgres::NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        println!("DROP DATABASE IF EXISTS rufs_nfe_rust...");
        db_client.execute("DROP DATABASE IF EXISTS rufs_nfe_rust", &[]).await?;        
        println!("...DROP DATABASE IF EXISTS rufs_nfe_rust.");
        println!("CREATE DATABASE rufs_nfe_rust...");
        db_client.execute("CREATE DATABASE rufs_nfe_rust", &[]).await?;        
        println!("...CREATE DATABASE rufs_nfe_rust.");
    }

    let mut rufs = RufsMicroService{
        check_rufs_tables: true,
        migration_path: args.migration_path,
        static_paths: args.static_paths,
        micro_service_server: MicroServiceServer{
            app_name: "rufs_nfe_rust".to_string(), ..Default::default()
        }, 
        ..Default::default()
    };

    rufs.connect(&format!("postgres://development:123456@localhost:5432/{}", rufs.micro_service_server.app_name)).await?;

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("requestProduct", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de produtos/componentes".to_string()));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("requestPayment", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de pagamentos".to_string()));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("person", "cnpjCpf") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("person", "name") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("account", "person") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("account", "description") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    rufs.micro_service_server.store_open_api("")?;
    rufs_tide_new(rufs).await
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    use anyhow::Context;
    use rufs_base_rust::rufs_micro_service::RufsMicroService;
    use rufs_crud_rust::{DataViewManager, DataViewWatch};
    use rufs_nfe_rust::RufsNfe;
    use serde_json::Value;

    let args = Args::parse();

    let mut app = match server(args).await {
        Ok(app) => app,
        Err(err) => {
            println!("...server exited with error : {}", err);
            std::process::exit(1);
        },
    };

    lazy_static::lazy_static! {
        static ref DATA_VIEW_MANAGER_MAP: tokio::sync::Mutex<std::collections::HashMap<String, DataViewManager<'static>>>  = {
            let data_view_manager_map = std::collections::HashMap::new();
            tokio::sync::Mutex::new(data_view_manager_map)
        }; 
        static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
    }

    async fn wasm_login(mut req: tide::Request<RufsMicroService<'_>>) -> tide::Result {
        let data_in = req.body_json::<Value>().await?;
        let mut data_view_manager_map = DATA_VIEW_MANAGER_MAP.lock().await;
        let state = req.state();
        //, data_in.get("path").context("Missing param path")?.as_str().context("Param path is not string")?
        let path = format!("http://127.0.0.1:{}", state.micro_service_server.port);
        let mut data_view_manager = DataViewManager::new(&path, &WATCHER);

        let data_out = match data_view_manager.login(data_in).await {
            Ok(data_out) => data_out,
            Err(err) => {
                let mut response = tide::Response::from(err.to_string());
                response.set_status(401);
                return Ok(response);
            }
        };

        data_view_manager_map.insert(data_view_manager.server_connection.login_response.jwt_header.clone(), data_view_manager);
        Ok(data_out.into())
    }
        
    app.at("/wasm_ws/login").post(wasm_login);

    async fn wasm_process(mut req: tide::Request<RufsMicroService<'_>>) -> tide::Result {
        let authorization_header_prefix = "Bearer ";
        let token_raw = req.header("Authorization").context("Missing header Authorization")?.last().as_str();

        let jwt = if token_raw.starts_with(authorization_header_prefix) {
            &token_raw[authorization_header_prefix.len()..]
        } else {
            return None.context("broken token")?;
        };

        let mut data_view_manager_map = DATA_VIEW_MANAGER_MAP.lock().await;
        let data_view_manager = data_view_manager_map.get_mut(jwt).context("Missing session")?;
        let data_in = req.body_json::<Value>().await?;

        let data_out = match data_view_manager.process(data_in).await {
            Ok(data_out) => data_out,
            Err(err) => {
                let mut response = tide::Response::from(err.to_string());
                response.set_status(500);
                return Ok(response);
            }
        };

        let data_out = serde_json::to_value(data_out)?;
        Ok(data_out.into())
    }
        
    app.at("/wasm_ws/process").post(wasm_process);
    let rufs = app.state();
    let listen = format!("127.0.0.1:{}", rufs.micro_service_server.port);
    println!("Staring rufs-nfe server at {}", listen);

    match app.listen(listen).await {
        Ok(_) => println!("...server exited."),
        Err(err) => {
            println!("...server exited with error : {}", err);
            std::process::exit(2);
        },
    }
}
        
// wasm-pack build --target web --dev
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use async_std::prelude::FutureExt;
    use rufs_nfe_rust::RufsNfe;
    use std::time::Duration;
    use rufs_crud_rust::{DataViewWatch};
    use crate::{server,Args};

    #[tokio::test]
    async fn selelium() -> Result<(), Box<dyn std::error::Error>> {
        println!("server()...");
        let args = Args{ port: 8080, static_paths: vec!["rufs-nfe-rust/pkg".to_string(), "rufs-nfe-rust/webapp".to_string()], migration_path: "rufs-nfe-rust/sql".to_string(), reset_db: true };
        let app = server(args).await.unwrap();
        println!("...server().");
        let rufs = app.state();
        let listen = format!("127.0.0.1:{}", rufs.micro_service_server.port);

        let listening = async {
            println!("app.listen({})...", listen);
            app.listen(listen).await.map_err(|err| {
                let dyn_err: Box<dyn std::error::Error> = Box::new(err);
                dyn_err
            })
        };

        lazy_static::lazy_static! {
            static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>; 
        }

        let selelium = async {
            std::thread::sleep( Duration::from_secs( 1 ) );
            println!("selelium...");
            rufs_crud_rust::tests::selelium(&WATCHER, "/home/alexsandro/Downloads/webapp-rust.side", "http://localhost:8080").await
        };

        listening.race(selelium).await?;
        println!("...selelium.");
        println!("...app.listen().");
        Ok(())
    }
}
