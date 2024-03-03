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
    use rufs_base_rust::{rufs_micro_service::RufsMicroService, micro_service_server::MicroServiceServer, openapi::RufsOpenAPI, client::DataViewWatch, db_adapter_postgres::DbAdapterPostgres, db_adapter_file::DbAdapterFile};
    use rufs_nfe_rust::RufsNfe;
    use serde_json::Value;
    use std::sync::Arc;

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

    lazy_static::lazy_static! {
        static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
    }

    let mut rufs = RufsMicroService{
        check_rufs_tables: true,
        migration_path: args.migration_path,
        static_paths: args.static_paths,
        micro_service_server: MicroServiceServer{
            app_name: "rufs_nfe_rust".to_string(), ..Default::default()
        }, 
        watcher: &WATCHER,
        entity_manager: DbAdapterPostgres::default(),
        db_adapter_file: DbAdapterFile::default(),
        ws_server_connections: Arc::default(),
        ws_server_connections_tokens: Arc::default(),
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
    rufs_base_rust::rufs_micro_service::rufs_tide_new(rufs).await
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let args = Args::parse();

    let app = match server(args).await {
        Ok(app) => app,
        Err(err) => {
            println!("...server exited with error : {}", err);
            std::process::exit(1);
        },
    };

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
    use rufs_base_rust::client::{DataViewWatch};
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
            rufs_base_rust::client::tests::selelium(&WATCHER, "/home/alexsandro/Downloads/webapp-rust.side", "http://localhost:8080").await
        };

        listening.race(selelium).await?;
        println!("...selelium.");
        println!("...app.listen().");
        Ok(())
    }
}
