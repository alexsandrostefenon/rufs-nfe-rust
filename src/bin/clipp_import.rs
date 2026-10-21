use indexmap::IndexMap;
use serde_json::{Value, json};

async fn import(message: &Value) -> Result<(), Box<dyn std::error::Error>> {
    use convert_case::Casing;
    use rsfbclient::{prelude::*, FbError};

	fn get_json(row: &rsfbclient::Row) -> Result<Value, Box<dyn std::error::Error>> {
        fn get_json_copy(obj :&mut Value, row :&rsfbclient::Row) -> Result<(), Box<dyn std::error::Error>> {
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

                let field_name = column.name.to_case(convert_case::Case::Camel);
                obj[field_name] = value;
            }

            Ok(())
        }

		let mut obj = json!({});
        get_json_copy(&mut obj, row)?;
		Ok(obj)
	}

    let db_url = message.get("dbUrl").ok_or("Missing field 'dbUrl' in message")?.as_str().ok_or("Broken field 'dbUrl' in message")?;
	let date_initial = message.get("dateInitial").ok_or("Missing field 'dateInitial' in message")?.as_str().ok_or("Broken field 'dateInitial' in message")?;
	println!("[clipp_import.import] rsfbclient::builder_pure_rust().from_string({db_url})?.connect()? ...");
    let mut fb_conn = rsfbclient::builder_pure_rust().from_string(db_url)?.connect()?;
	println!("[clipp_import.import] ... connected.");

    #[cfg(debug_assertions)]
    println!("[clipp_import] map_person ...");

    let mut map_person = IndexMap::new();

    {
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
            coalesce(ddd_celul,'') || coalesce(fone_celul,'') as phone,
            insc_estad as ie_rg,
            insc_munic as im,
            nome_fanta as fantasy,
            c.id_cliente as id_import,
            coalesce(trim(f.cpf),trim(j.cnpj),lpad(cast(c.id_cliente as varchar(18)),15,'00.000.000/0000') || '-00') as cnpj_cpf
        from tb_cliente c left outer join tb_cli_pf f on c.id_cliente = f.id_cliente left outer join tb_cli_pj j on c.id_cliente = j.id_cliente order by id_import
        ";

        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let obj = get_json(&row?)?;
            map_person.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj);
        }
    }

    #[cfg(debug_assertions)]
    println!("[clipp_import] ...map_person.");

    #[cfg(debug_assertions)]
    println!("[clipp_import] tb_estoque ...");

    let (map_product, map_service) = {
        //TABLE TB_ESTOQUE ID_ESTOQUE
        //TABLE TB_EST_IDENTIFICADOR ID_IDENTIFICADOR
        let sql = r#"
        select
            trim(replace(e.descricao, '+', ' ')) as name,
            left(observacao, 255) as description,
            left(lower(uni_medida), 2) as unity,
            prc_venda as "value",
            prc_custo as value_cost,
            margem_lb as margin_sale,
            --case grade_serie when 'S' then 'true' else 'false' end as serial_control,
            e.id_tipoitem,
            ei.id_identificador
        from
            tb_est_identificador as ei,
            tb_estoque as e,
            --tb_est_grupo as eg,
            tb_est_tipo_item_sis etis
        where
            ei.id_estoque = e.id_estoque
            --and e.id_grupo = eg.id_grupo
            and e.id_tipoitem = etis.id_tipoitem
        order by id_identificador
        "#;

        let mut map_product = IndexMap::new();
        let mut map_service = IndexMap::new();
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let obj = get_json(&row?)?;
            let id_tipo_item = obj.get("idTipoitem").ok_or("broken id_tipoitem")?.as_str().ok_or("broken id_tipoitem str")?;
            let id_identificador = obj.get("idIdentificador").ok_or("broken idIdentificador")?.as_u64().ok_or("broken u64")?;

            if ["9"].contains(&id_tipo_item) {
                /*
                let obj = match publish("service", &["id"], &obj, false).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };
                */
                map_service.insert(id_identificador, obj);
            } else {
                /*
                let obj = match publish("product", &["id"], &obj, false).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };
                */
                map_product.insert(id_identificador, obj);
            }

        }

        (map_product, map_service)
    };

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... tb_estoque.");

    let mut map_documents = IndexMap::new();

    #[cfg(debug_assertions)]
    println!("[clipp_import] tb_os ...");

    {
        let sql = r#"
        select
        id_status,
        1 as type,
        id_status as state,
        id_cliente,
        (dt_os || 'T' || hr_os) as "date",
        left(observacao,255) as additional_data,
        id_os as id_import
        from tb_os where dt_os >= ? order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ((date_initial),))?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut document = get_json(&row)?;
            let client_id = document.get("idCliente").ok_or("broken")?.as_u64().ok_or("broken")?;
            document["personDest"] = map_person.get(&client_id).ok_or("Broken 'personDest'")?.clone();
            document["requestService"] = json!([]);
            document["requestProduct"] = json!([]);
            document["requestPayment"] = json!([]);
            document["requestFreight"] = json!({"value": 0.0});
            let obj_id_status_in = document.get("idStatus").ok_or("Missing id_status")?.as_u64().ok_or("broken u64")?;

            for (id_in, id_out) in vec![(1, 220),(2, 250), (3, 270), (4, 280), (5, 250), (6, 220), (7, 260), (9, 320), (10, 310), (11, 240), (12, 320)] {
                if obj_id_status_in == id_in {
                    document["state"] = json!(id_out);
                    break;
                }
            }

            map_documents.insert(document.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, document);
        }
    }

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... tb_os.");

    #[cfg(debug_assertions)]
    println!("[clipp_import] tb_os_objeto_os ...");

    fn merge(obj_out :&mut Value, obj_in :&Value) -> Result<(), Box<dyn std::error::Error>> {
        let map_out = obj_out.as_object_mut().ok_or("[clipp_import.merge] Broken obj_out.as_object_mut")?;
        let map_in = obj_in.as_object().ok_or("[clipp_import.merge] Broken obj_in.as_object")?;

        for (field_name, value) in map_in {
            if map_out.get(field_name).is_some() {
                continue;
            }

            map_out.insert(field_name.clone(), value.clone());
        }

        Ok(())
    }

    {
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
        from tb_os_objeto_os where id_os in (select id_os from tb_os where dt_os >= ?) order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ((date_initial),))?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_repair import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            obj["name"] = json!("EQUIPAMENTO PARA CONSERTO, REPARO OU MANUTENCAO");
            let id_os = obj.get("idImport").ok_or("Missing idOs")?.as_u64().ok_or("broken u64 idOs")?;
            let document = map_documents.get_mut(&id_os).ok_or("[import_clipp] Missing id_os in map_request.")?;
            #[cfg(debug_assertions)]
            println!("[clipp_import] id_os={id_os} : {obj}");
            document["requestRepair"] = obj;
        }
    }

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... tb_os_objeto_os.");

    #[cfg(debug_assertions)]
    println!("[clipp_import] tb_os_item ...");

    {
        let sql = r#"
        select
            id_identificador,
            id_os,
            qtd_item as quantity,
            vlr_unit as "value",
            vlr_desc as value_desc,
            vlr_total as value_item,
            id_itemos as id_import
        from tb_os_item where id_os in (select id_os from tb_os where dt_os >= ?) order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ((date_initial),))?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_product import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            #[cfg(debug_assertions)]
            println!("[clipp_import] request_product {obj}");
            let id_identificador = obj.get("idIdentificador").ok_or("Missing idIdentificador")?.as_u64().ok_or("broken u64 idIdentificador")?;
            let id_os = obj.get("idOs").ok_or("Missing idOs")?.as_u64().ok_or("broken u64 idOs")?;
            let request = map_documents.get_mut(&id_os).ok_or("[import_clipp] Missing id_os in map_request.")?;

            if let Some(product) = map_product.get(&id_identificador) {
                merge(&mut obj, product)?;
                let request_product = request.get_mut("requestProduct").ok_or("[import_clipp] Broken 'requestProduct'")?.as_array_mut().ok_or("[import_clipp] Broken 'requestProduct' as array")?;
                request_product.push(obj);
            } else if let Some(service) = map_service.get(&id_identificador) {
                merge(&mut obj, service)?;
                let request_service = request.get_mut("requestService").ok_or("[import_clipp] Broken 'requestService'")?.as_array_mut().ok_or("[import_clipp] Broken 'requestService' as array")?;
                request_service.push(obj);
            } else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("idIdentificador is not product or service")?;
            }
        }
    }

	let auth_token = message.get("authorization").ok_or("Missing field 'authorization' in message")?.as_str().ok_or("Broken field 'authorization' in message")?;
	let customer = message.get("customer").ok_or("Missing field 'customer' in message")?.as_str().ok_or("Broken field 'customer' in message")?;

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... tb_os_item.");

    let mut i = 1;
    let size = map_documents.len();
    println!("[clipp_import] rufs_nfe_rust::import::import_document ({size}) ...");

    for (_id_import, mut document) in map_documents {
        println!("[clipp_import] rufs_nfe_rust::import::import_document {}/{}.", i, size);
        i += 1;
        document["customer"] = json!(customer);
        document["person"] = json!({"cnpjCpf": customer});
        let _request = rufs_nfe_rust::import::import_document(auth_token, document, false, false).await?;
        //let request_id = request.get("id").ok_or("broken request id")?.as_u64().ok_or("broken u64")?;
    }

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... finished.");

/*
TB_MOVDIARIO                   	55651
TB_OS_ITEM                     	16521
TB_CTAPAG_MOVTO                	13898
TB_ALIQUOTAS_IBPT              	12097
TB_CTAREC_MOVTO                	11913
TB_OS                          	6248
TB_OS_NOME                     	6247
TB_OS_OBJETO_OS                	6247
TB_CONTA_RECEBER               	5981
TB_CTAREC_BAIXA                	5909
TB_CIDADE_SIS                  	5571
TB_CIDADE_CARTORIO             	5570
TB_CONTA_PAGAR                 	3038
TB_CTAPAG_BAIXA_MOVTO          	3014
TB_CTAPAG_BAIXA                	2975
TB_CLIENTE                     	2802
TB_CLI_PJ                      	1815
TB_CEST_NCM                    	1251
TB_EST_PRODUTO_LOG             	1178
TB_COD_PROD_ANP                	1040
TB_EST_SERVICO_SIS             	1017
TB_CLI_PF                      	1010
TB_FUNC_MOD_COLUNA_SIS         	1006
TB_EST_IDENTIFICADOR           	910
TB_ESTOQUE                     	910
TB_EST_PRODUTO                 	896
TB_FUNC_AUDITORIA_SIS          	801
TB_CFOP_SIS                    	787
TB_PLANO_CONTAS_SPED_SIS       	666
TB_NFVENDA_CONF_SIS            	653
TB_CLI_PF_ADICIONAIS           	609
TB_EST_SERIAL                  	504
TB_PARAMETRO                   	399
TB_BANCO_SIS                   	363
TB_FORNECEDOR                  	354
TB_D4_PAF                      	324
TB_EST_PRODUTO_EMPRESA_LOG     	282
TB_PAIS_SIS                    	256
IBE$VERSION_HISTORY            	201
TB_PLANO_CONTAS                	169
TB_OS_HIST_OS                  	163
TB_EST_QTD_HISTORICO           	136
TB_CST_SIS                     	135
TB_CENQ_IPI_SIS                	129
TB_FUNC_MODULO                 	127
TB_CTAREC_BAIXA_MOVTO          	122
TB_NFV_ITEM                    	117
TB_NFV_ITEM_ICMS               	117
TB_NFV_ITEM_ST                 	117
TB_NFV_ITEM_ARMA               	117
TB_J2_PAF                      	115
TB_EST_PRODUTO_EMPRESA         	113
*/
/*
    #[cfg(debug_assertions)]
    println!("[clipp_import] stock_product ...");

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
                match publish("stock_service", &["id"], &obj, false).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                }
            } else if let Some(id) = map_product.get(&id_import) {
                obj["id"] = json!(id);
                match publish("stock_product", &["id"], &obj, false).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                };
            }
        }
    }

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... stock_product.");
*/
/*
    #[cfg(debug_assertions)]
    println!("[clipp_import] tb_nfvenda ...");

    {
        let sql = r#"
        select
        1 as type,
        320 as state,
        id_cliente,
        (dt_saida || 'T' || hr_saida) as "date_in_out",
        (dt_saida || 'T' || hr_saida) as "date",
        NF_NUMERO as numero,
        cast(NF_SERIE as int) as serie,
        cast(NF_MODELO as int) as mod,
        ENT_SAI,
        left(INF_COMP_EDIT,255) as additional_data,
        ID_NFVENDA*10000 as id_import
        from tb_nfvenda where dt_saida >= ? order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ((date_initial),))?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut document = get_json(&row)?;
            let client_id = document.get("idCliente").ok_or("broken")?.as_u64().ok_or("broken")?;
            document["personDest"] = map_person.get(&client_id).ok_or("Broken 'personDest'")?.clone();
            document["person"] = json!({"cnpjCpf": person_omission});
            document["requestProduct"] = json!([]);
            document["requestPayment"] = json!([]);
            document["requestFreight"] = json!({"value": 0.0});
            let request = rufs_nfe_rust::import::import_document(auth_token, document, false).await?;
            let request_id = request.get("id").ok_or("broken request id")?.as_u64().ok_or("broken u64")?;
            map_request.insert(request.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, request_id);
        }
    }

    #[cfg(debug_assertions)]
    println!("[clipp_import] ... tb_nfvenda.");
*/
/*
    {
        let sql = r#"
        select
        id_nfvenda,
        id_identificador,
        cfop,
        qtd_item as quantity,
        vlr_unit as "value",
        vlr_desc as value_desc,
        vlr_total as value_item,
        vlr_frete as value_freight,
        vlr_tottrib as value_all_tax,
        id_nvfitem as id_import
        from tb_nfv_item order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

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
            let id_nfvenda = obj.get("idNfvenda").ok_or("Missing idNfvenda")?.as_u64().ok_or("broken u64 idNfvenda")? * 10000;

            let Some(request) = map_request.get(&id_nfvenda) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] Missing id_os in map_request.")?;
            };

            obj["request"] = json!(request);

            if let Some(product) = map_product.get(&id_identificador) {
                obj["product"] = json!(product);

                let obj_out = match publish("request_product", &["id"], &obj, false).await {
                    Ok(value) => value,
                    Err(err) => {
                        if err.to_string().contains(r#"duplicate key value violates unique constraint "request_product_pkey""#) {
                            continue;
                        }

                        return Err(err)?;
                    },
                };

                map_request_product.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out);
            } else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("idIdentificador is not product")?;
            }
        }

    }
*/
/*
    {
        let sql = r#"
        select
        a.id_nfvenda,
        r.documento,
        r.dt_vencto,
        r.vlr_ctarec,
        r.id_ctarec as id_import,
        from tb_conta_receber r,tb_nfv_ctarec a where r. order by id_import
        "#;
        let _rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;
    }
*/
/*
TB_NFVENDA_FMAPAGTO_NFCE
TB_NFV_CTAREC
TB_CONTA_RECEBER
*/
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "kafka"))]
	{
        let host = std::env::var("REDIS_HOST").unwrap_or("127.0.0.1".to_owned());
        let client = redis::Client::open(format!("redis://{host}/")).map_err(|err| format!("Redis failt : {err}"))?;
		let mut con = client.get_connection().expect("conn");

		const GROUP_NAME: &str = "clipp_group";
		let res: Result<(), _> = redis::Commands::xgroup_create_mkstream(&mut con, "clipp", GROUP_NAME, "$");

		if let Err(e) = res {
			println!("[clipp_import.main] Group already exists: {e:?}")
		}

		let opts = redis::streams::StreamReadOptions::default().block(10000).group(GROUP_NAME, "clipp-import-1");

		loop {
			let srr: Result<redis::streams::StreamReadReply, redis::RedisError> = redis::Commands::xread_options(&mut con,
				&["clipp"],
				&[">"],
				&opts);

			let srr = match srr {
				Ok(srr) => srr,
				Err(err) => {
					eprintln!("[clipp_import.main] {err}");
					break;
				},
			};

			for redis::streams::StreamKey { key, ids } in srr.keys {
				println!("[clipp_import.main] Stream key {key}");
				let mut list_id = vec![];

				for redis::streams::StreamId { id, map } in ids {
					println!("[clipp_import.main] Redis Events Stream : ID {id}");

					for (n, s) in map {
						println!("[clipp_import.main] Redis Events Stream : Stream n {n}");

						if let redis::Value::BulkString(bytes) = s {
							let message = std::str::from_utf8(&bytes);

							match message {
								Ok(message) => {
									println!("[clipp_import.main] Redis Events Stream : {message}.");
									let value = serde_json::from_str::<Value>(message);

									match value {
											Ok(value) => {                                                
												if let Err(err) = import(&value).await {
													eprintln!("[clipp_import.main] Redis Events Stream : Error of process_broker_message({message}) : {}", err);
												}
											},
											Err(err) => eprintln!("[clipp_import.main] Redis Events Stream : value is not valid json ({message}) : {}", err),
										}
								},
								Err(err) => eprintln!("[clipp_import.main] Redis Events Stream : value is not utf8 string : {}", err),
							}
						} else {
							panic!("Weird data")
						}
					}

					list_id.push(id);
				}

				redis::TypedCommands::xack(&mut con, key, GROUP_NAME, &list_id).expect("ack");
			}
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    #[tokio::test]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
		let list = [
            // WIN1252,ISO8859_1
            "firebird://SYSDBA:masterkey@localhost:3050//var/lib/firebird/data/clipp-12345678901.fdb?charset=WIN1252"
		];

		let customer_user = "12345678901.guest";
		let password = "e10adc3949ba59abbe56e057f20f883e";
		let client = reqwest::Client::new();
		let res = client.post("http://localhost:8080/nfe/rest/login").json(&json!({"user": customer_user, "password": password})).send().await?;
		let status = res.status();

		let login_response = if status == reqwest::StatusCode::OK {
			let res: Value = res.json().await?;
			res
		} else {
			let text = res.text().await?;
			return Err(text)?;
		};

		let mut message = json!({"customer": "123.456.789-01", "dateInitial": "2004-09-12"});
		message["authorization"] = login_response.get("jwtHeader").ok_or("Missing jwtHeader in login response!")?.clone();

		for item in list {
			message["dbUrl"] = json!(item);
			crate::import(&message).await?;
		}

        Ok(())
    }
}
