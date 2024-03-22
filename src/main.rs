#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Parser, Debug)]
struct Args {
    #[arg(long,default_value = "8080")]
    port: u16,
    #[cfg(debug_assertions)]
    #[arg(long,default_value = "true")]
    reset_db: bool,
    #[cfg(not(debug_assertions))]
    #[arg(long,default_value = "false")]
    reset_db: bool,
}

#[cfg(not(target_arch = "wasm32"))]
async fn server(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use rufs_base_rust::{rufs_micro_service::{RufsMicroService, RufsParams}, openapi::RufsOpenAPI, client::DataViewWatch};
    use rufs_nfe_rust::RufsNfe;
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

    lazy_static::lazy_static! {
        static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
    }

    let params = RufsParams {
        app_name: "rufs_nfe_rust".to_string(), 
        ..Default::default()
    };

    #[cfg(not(debug_assertions))]
    let fs_prefix = "";
    #[cfg(debug_assertions)]
    let fs_prefix = "rufs-nfe-rust/";

    let db_uri = format!("postgres://development:123456@localhost:5432/{}", params.app_name);
    let mut rufs = RufsMicroService::connect(&db_uri, true, &format!("{}sql", fs_prefix), params, &WATCHER).await?;

    if let Some(field) = rufs.openapi.get_property_mut("requestProduct", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de produtos/componentes".to_string()));
    }

    if let Some(field) = rufs.openapi.get_property_mut("requestPayment", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de pagamentos".to_string()));
    }

    if let Some(field) = rufs.openapi.get_property_mut("person", "cnpjCpf") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.openapi.get_property_mut("person", "name") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.openapi.get_property_mut("account", "person") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.openapi.get_property_mut("account", "description") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    rufs.store_open_api("")?;

    #[cfg(feature = "warp")]
    #[cfg(not(feature = "tide"))]
    {
        use warp::Filter;

        let rufs_routes = rufs_base_rust::rufs_micro_service::rufs_warp(rufs).await;
        let listener = format!("127.0.0.1:{}", args.port);
        println!("Staring rufs-nfe server at {}", listener);
        let dedicated = warp::path("nfe_dedicated").and(warp::get()).map(|| {"Hello from rufs-nfe!".to_string()});
        let routes = dedicated
            .or(rufs_routes)
            .or(warp::path("pkg").and(warp::fs::dir(format!("{}pkg", fs_prefix))))
            .or(warp::path("webapp").and(warp::fs::dir(format!("{}webapp", fs_prefix))))
            .or(warp::path::end().and(warp::fs::file(format!("{}webapp/index.html", fs_prefix))))
            ;
        warp::serve(routes).run(([127, 0, 0, 1], args.port)).await;
    }

    #[cfg(feature = "tide")]
    #[cfg(not(feature = "warp"))]
    {
        let mut app = Box::new(tide::with_state(rufs));
        rufs_base_rust::rufs_micro_service::rufs_tide(&mut app).await?;
        let listener = format!("127.0.0.1:{}", args.port);
        println!("Staring rufs-nfe server at {}", listener);
        app.listen(listener).await?;
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = server(&args).await {
        println!("...server exited with error : {}", err);
        std::process::exit(1);
    }
}
        
// wasm-pack build --target web --dev
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use async_std::prelude::FutureExt;
    use rufs_nfe_rust::RufsNfe;
    use std::time::Duration;
    use rufs_base_rust::client::DataViewWatch;
    use crate::{server,Args};

    #[tokio::test]
    async fn selelium() -> Result<(), Box<dyn std::error::Error>> {
        let listening = async {
            println!("server()...");
            let args = Args{ port: 8080, reset_db: true };
            server(&args).await
        };

        lazy_static::lazy_static! {
            static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>; 
        }

        let selelium = async {
            std::thread::sleep( Duration::from_secs( 5 ) );
            println!("selelium...");
            rufs_base_rust::client::tests::selelium(&WATCHER, "/home/alexsandro/Downloads/webapp-rust.side", "http://localhost:8080").await
        };

        listening.race(selelium).await?;
        println!("...selelium.");
        println!("...app.listen().");
        Ok(())
    }
}
