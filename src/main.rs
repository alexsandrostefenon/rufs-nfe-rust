#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
use rufs_base_rust::{rufs_micro_service::{RufsMicroService, RufsParams}, openapi::RufsOpenAPI, client::DataViewWatch};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Parser, Debug)]
struct Args {
    #[arg(long,default_value = "8080")]
    port: u16,
    #[cfg(debug_assertions)]
    #[cfg(feature = "postgres")]
    //#[arg(long,default_value = "false")]
    #[arg(long,default_value = "true")]
    reset_db: bool,
    #[cfg(not(debug_assertions))]
    #[arg(long,default_value = "false")]
    reset_db: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "clipp")]
async fn import_clipp(rufs: &RufsMicroService<'_>) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use convert_case::Casing;
    use rsfbclient::{prelude::*, FbError};
    use rufs_base_rust::entity_manager::EntityManager;
    use serde_json::{json, Value};

	fn get_json(row: &rsfbclient::Row) -> Result<Value, Box<dyn std::error::Error>> {
		let mut obj = json!({});

		for column in &row.cols {
			let value : Value = match &column.value {
                rsfbclient::SqlType::Text(value) => json!(value),
                rsfbclient::SqlType::Integer(value) => json!(value),
                rsfbclient::SqlType::Floating(value) => json!(value),
                rsfbclient::SqlType::Timestamp(value) => json!(value),
                rsfbclient::SqlType::Binary(_value) => serde_json::Value::Null,
                rsfbclient::SqlType::Boolean(value) => json!(value),
                rsfbclient::SqlType::Null => serde_json::Value::Null,
            };

			obj[column.name.to_case(convert_case::Case::Camel)] = value;
		}

		Ok(obj)
	}

    let mut fb_conn = rsfbclient::builder_native().from_string("firebird://SYSDBA:masterkey@localhost:3050//var/lib/firebird/3.0/data/CLIPP-3.fdb?charset=WIN1252")?.connect()?;//WIN1252,ISO8859_1

    let map_person = {
        let sql = "
        select
        left(nome, 255) as name,
        left(observacao, 255) as additional_data,
        cast(id_pais as integer) as country,
        end_cep as zip,--replace(end_cep,'-','') as zip,
        cast(id_cidade as integer) as city,
        end_bairro as district,
        coalesce(end_tipo,'') || ' ' || coalesce(end_lograd,'') as address,
        end_numero as address_number,
        end_comple as complement,
        email_cont as email,
        fone_celul as phone,
        insc_estad as ie_rg,
        insc_munic as im,
        nome_fanta as fantasy,
        c.id_cliente as id_import,
        coalesce(f.cpf,j.cnpj,lpad(cast(c.id_cliente as varchar(18)),15,'00.000.000/0000') || '-00') as cnpj_cpf
        from tb_cliente c left outer join tb_cli_pf f on c.id_cliente = f.id_cliente left outer join tb_cli_pj j on c.id_cliente = j.id_cliente order by id_import
        ";

        let mut map = HashMap::new();
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let obj = get_json(&row?)?;

            let _obj_out = match rufs.entity_manager.insert(&rufs.openapi, "person", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    if err.to_string().contains(r#"duplicate key value violates unique constraint "person_pkey""#) {
                        map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj.get("cnpjCpf").ok_or("broken cnpj_cpf")?.as_str().ok_or("broken str")?.to_string());
                        continue;
                    }

                    return Err(err)?;
                },
            };

            map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj.get("cnpjCpf").ok_or("broken cnpj_cpf")?.as_str().ok_or("broken str")?.to_string());
        }

        map
    };

    let map_request = {
        let sql = r#"
        select
        id_status,
        2 as type,
        id_status as state,
        '90.979.337/0001-85' as person,
        id_cliente,
        (dt_os || 'T' || hr_os) as "date",
        left(observacao,255) as additional_data,
        id_os as id_import
        from tb_os order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;
        let mut map = HashMap::new();

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let client_id = obj.get("idCliente").ok_or("broken")?.as_u64().ok_or("broken")?;
            obj["personDest"] = json!(map_person.get(&client_id));
            let obj_id_status_in = obj.get("idStatus").ok_or("Missing id_status")?.as_u64().ok_or("broken u64")?;

            for (id_in, id_out) in vec![(1, 220),(2, 250), (3, 270), (4, 280), (5, 250), (6, 220), (7, 260), (9, 320), (10, 310), (11, 240), (12, 320)] {
                if obj_id_status_in == id_in {
                    obj["state"] = json!(id_out);
                    break;
                }
            }

            let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    if err.to_string().contains(r#"duplicate key value violates unique constraint "request_person_person_dest_date_key""#) {
                        continue;
                    }

                    return Err(err)?;
                },
            };

            map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out.get("id").ok_or("broken request id")?.as_u64().ok_or("broken u64")?);
        }

        map
    };

    let (map_product, map_service) = {
        let sql = r#"
        select
        left(descricao, 120) as name,
        left(observacao, 255) as description,
        left(lower(uni_medida), 2) as unity,
        prc_venda as "value",
        prc_custo as value_cost,
        margem_lb as margin_sale,
        (case grade_serie when 'S' then true else false end) as serial_control,
        id_tipoitem,
        id_estoque as id_import
        from tb_estoque order by id_import
        "#;

        let mut map_product = HashMap::new();
        let mut map_service = HashMap::new();
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let obj = get_json(&row?)?;
            let id_tipo_item = obj.get("idTipoitem").ok_or("broken id_tipoitem")?.as_str().ok_or("broken id_tipoitem str")?;
            let id_import = obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?;

            if ["9"].contains(&id_tipo_item) {
                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "service", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };

                map_service.insert(id_import, obj_out.get("id").ok_or("broken service id")?.as_u64().ok_or("broken service id u64")?);
            } else {
                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "product", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };

                map_product.insert(id_import, obj_out.get("id").ok_or("broken product id")?.as_u64().ok_or("broken u64")?);
            }

        }

        (map_product, map_service)
    };

    {
        let sql = r#"
        select
        prc_venda as "value",
        prc_custo as value_cost,
        margem_lb as margin_sale,
        id_estoque as id_import
        from tb_estoque order by id_import
        "#;

        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let mut obj = get_json(&row?)?;
            let id_import = obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?;

            if let Some(id) = map_service.get(&id_import) {
                obj["id"] = json!(id);
                match rufs.entity_manager.insert(&rufs.openapi, "stock_service", &obj).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                }
            } else if let Some(id) = map_product.get(&id_import) {
                obj["id"] = json!(id);
                match rufs.entity_manager.insert(&rufs.openapi, "stock_product", &obj).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                };
            }
        }
    }

    let _map_request_repair = {
        let sql = r#"
        select
        defeito as defect,
        coalesce(ident1,'') as serial,
        ident2 as inmetro,
        ident3 as marca,
        ident4 as modelo,
        ident5 as capacidade,
        ('Marca: ' || coalesce(ident3,'') || '\nModelo: ' || coalesce(ident4,'') || '\nCapacidade: ' || coalesce(ident5,'')) as additional_data,
        id_os as id_import
        from tb_os_objeto_os order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;
        let mut map = HashMap::new();

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_repair import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let id_os = obj.get("idImport").ok_or("Missing idOs")?.as_u64().ok_or("broken u64 idOs")?;

            let Some(request) = map_request.get(&id_os) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] request_repair : Missing id_os in map_request.")?;
            };

            obj["request"] = json!(request);

            let Some(product) = map_product.get(&1) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] Missing product in map_product.")?;
            };

            obj["product"] = json!(product);
            
            let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_repair", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    println!("[import_clipp] request import insert : {}", err);
                    return Err(err)?;
                },
            };

            map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out.get("request").ok_or("broken request id in request repair.")?.as_u64().ok_or("broken u64")?);
        }

        map
    };

    let (_map_request_product, _map_request_service) = {
        let sql = r#"
        select
        id_identificador,
        id_os,
        qtd_item as quantity,
        vlr_unit as "value",
        vlr_desc as value_desc,
        vlr_total as value_item,
        id_itemos as id_import
        from tb_os_item order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;
        let mut map_request_product = HashMap::new();
        let mut map_request_service = HashMap::new();

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_product import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let id_identificador = obj.get("idIdentificador").ok_or("Missing idIdentificador")?.as_u64().ok_or("broken u64 idIdentificador")?;
            let id_os = obj.get("idOs").ok_or("Missing idOs")?.as_u64().ok_or("broken u64 idOs")?;

            let Some(request) = map_request.get(&id_os) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] Missing id_os in map_request.")?;
            };

            obj["request"] = json!(request);

            if let Some(product) = map_product.get(&id_identificador) {
                obj["product"] = json!(product);

                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_product", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        if err.to_string().contains(r#"duplicate key value violates unique constraint "request_product_pkey""#) {
                            continue;
                        }

                        return Err(err)?;
                    },
                };
    
                map_request_product.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out);
            } else if let Some(service) = map_service.get(&id_identificador) {
                obj["service"] = json!(service);

                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_service", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        if err.to_string().contains(r#"duplicate key value violates unique constraint "request_service_pkey""#) {
                            continue;
                        }

                        return Err(err)?;
                    },
                };
    
                map_request_service.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out);
            } else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("idIdentificador is not product or service")?;
            }
        }

        (map_request_product, map_request_service)
    };

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn server(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use rufs_base_rust::rufs_micro_service::RufsMicroServiceAuthenticator;
    use serde_json::Value;
    use rufs_nfe_rust::RufsNfe;

    if args.reset_db {
        let path = "openapi-rufs_nfe_rust.json";

        if Path::new(path).exists() {
            std::fs::remove_file(path)?;
        }

        let (pg_conn, connection) = tokio_postgres::connect("postgres://development:123456@localhost:5432/rufs_nfe_development", tokio_postgres::NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        println!("DROP DATABASE IF EXISTS rufs_nfe_rust...");
        pg_conn.execute("DROP DATABASE IF EXISTS rufs_nfe_rust", &[]).await?;        
        println!("...DROP DATABASE IF EXISTS rufs_nfe_rust.");
        println!("CREATE DATABASE rufs_nfe_rust...");
        pg_conn.execute("CREATE DATABASE rufs_nfe_rust", &[]).await?;        
        println!("...CREATE DATABASE rufs_nfe_rust.");
    }

    #[derive(Clone)]
    pub struct State {
        authenticator: RufsMicroServiceAuthenticator
    }
        
    lazy_static::lazy_static! {
        static ref RUFS_STATE : State = State {
            authenticator: RufsMicroServiceAuthenticator()
        };
    }

    lazy_static::lazy_static! {
        static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
    }

    let params = RufsParams {
        app_name: "rufs_nfe_rust".to_string(), 
        ..Default::default()
    };

    #[cfg(debug_assertions)]
    let fs_prefix = "rufs-nfe-rust/";
    #[cfg(not(debug_assertions))]
    let fs_prefix = "";
    println!("std::env::current_dir() : {:?}", std::env::current_dir()?);
    println!("fs_prefix = {:?}", fs_prefix);

    let db_uri = format!("postgres://development:123456@localhost:5432/{}", params.app_name);
    let mut rufs = RufsMicroService::connect(&db_uri, true, &format!("{}sql", fs_prefix), params, &WATCHER).await?;

    if let Some(field) = rufs.openapi.get_property_mut("requestRepair", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Equipamento para conserto/revisão".to_string()));
    }

    if let Some(field) = rufs.openapi.get_property_mut("requestProduct", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de produtos/componentes".to_string()));
    }

    if let Some(field) = rufs.openapi.get_property_mut("requestService", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de serviços".to_string()));
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

    rufs.store_open_api()?;

    if args.reset_db {
        #[cfg(feature = "clipp")]
        import_clipp(&rufs).await?;
    }

    #[cfg(feature = "warp")]
    #[cfg(not(feature = "tide"))]
    {
        use warp::Filter;

        let rufs_routes = rufs_base_rust::rufs_micro_service::rufs_warp(rufs, &RUFS_STATE.authenticator).await;
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
    //use async_std::prelude::FutureExt;
    use futures_lite::future::FutureExt;
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
