use chrono::{DateTime, Months};
use convert_case::Casing;
use regex::Regex;
use reqwest::StatusCode;
use serde_json::{Value, json};

pub fn convert_obj_copy(obj_out: &mut Value, obj_in :&Value, fields_not_null :&[&str]) -> Result<(), Box<dyn std::error::Error>> {
	let map = obj_in.as_object().ok_or("obj_in is not 'map' object!")?;

	for (field_name_in, field) in map {
		let field_name_out = field_name_in.to_case(convert_case::Case::Camel);

		match field {
			Value::Object(_obj) => continue,
			Value::Null => continue,
			Value::String(value) => {
				if value.is_empty() && fields_not_null.contains(&field_name_in.as_str()) == false {
					continue;
				}
			},
			_ => {},
		}

		obj_out[field_name_out] = field.clone();
	}

	Ok(())
}

pub fn convert_obj(obj_in :&Value, fields_not_null :&[&str]) -> Result<Value, Box<dyn std::error::Error>> {
	let mut obj_out = json!({});
	convert_obj_copy(&mut obj_out, obj_in, fields_not_null)?;
	Ok(obj_out)
}

pub fn convert_obj_child(obj :&Value, child_name :&str, fields_not_null :&[&str], trace :&str) -> Result<Value, Box<dyn std::error::Error>> {
	let obj_in = obj.get(child_name).ok_or(format!("[{trace}] Missing child '{child_name}'"))?;

	if obj_in.is_object() == false {
		return Err(format!("Child '{child_name}' is not 'map' object!"))?;
	}

	let mut obj_out = json!({});
	convert_obj_copy(&mut obj_out, obj_in, fields_not_null)?;
	Ok(obj_out)
}

pub async fn import_document(auth_token :&str, mut document :Value, require_request_nfe :bool, check_payments :bool) -> Result<Value, Box<dyn std::error::Error>> {
	#[cfg(debug_assertions)]
	tokio::fs::write(format!("/tmp/request_nfe.json"), document.to_string().as_bytes()).await?;
	#[cfg(debug_assertions)]
	let url_base = format!("http://localhost:8081/rest/");
	#[cfg(not(debug_assertions))]
	let url_base = format!("http://rufs-nfe:8080/rest/");
	let client = reqwest::Client::new();

	let post = async |schema_name_snake: &str, obj: &Value| -> Result<Value, Box<dyn std::error::Error>> {
		let url = format!("{url_base}{schema_name_snake}");
		#[cfg(debug_assertions)]
		println!("[import_document.post({schema_name_snake})] {url}");
		let res = client.post(url).bearer_auth(auth_token).json(obj).send().await?;
		let status = res.status();

		if status == StatusCode::OK {
			match res.json().await {
				Ok(value) => return Ok(value),
				Err(err) => {
					return Err(format!("{err}"))?;
				},
			}
		} else {
			#[cfg(debug_assertions)]
			eprintln!("[import_document.post({schema_name_snake})] obj : {obj}");
			let text = res.text().await?;
			return Err(text)?;
		}
	};

	let publish = async |schema_name_snake :&str, primary_keys :&[&str], fields_not_null :&[&str], rec_imported :&Value, replace: bool| -> Result<Value, Box<dyn std::error::Error>> {
		#[cfg(debug_assertions)]
		println!("[import_document.publish({schema_name_snake})] starting ...");
		#[cfg(debug_assertions)]
		println!("[import_document.convert_obj()] starting ...");
		let mut fields_not_null_aux = primary_keys.to_vec();
		fields_not_null_aux.extend_from_slice(fields_not_null);
		let obj = convert_obj(rec_imported, fields_not_null_aux.as_slice())?;
		#[cfg(debug_assertions)]
		println!("[import_document.convert_obj()] ... exited");
		let mut query_list = vec![];

		for primary_key_name in primary_keys {
			#[cfg(debug_assertions)]
			println!("[import_document.primary_key_name {primary_key_name}] starting ...");
			let Some(primary_key) = obj.get(primary_key_name) else {
				let str = serde_json::to_string_pretty(&obj)?;
				eprintln!("[import_document.merge.primary_key_name] error : {str}.");
				return Err(format!("Missing field '{primary_key_name}' in structure {schema_name_snake} in parsed object : {str}"))?;
			};

			let primary_key = match primary_key {
				Value::Number(number) => number.to_string(),
				Value::String(value) => value.to_string(),
				_ => return Err(format!("Invalid primary_key : {primary_key}"))?,
			};

			query_list.push(format!("{primary_key_name}={primary_key}"));
			#[cfg(debug_assertions)]
			println!("[import_document.primary_key_name {primary_key_name}] ... exited");
		}

		let url = format!("{url_base}{schema_name_snake}?{}", query_list.join("&"));

		let method;
		#[cfg(debug_assertions)]
		println!("[import_document put/patch] {url}, starting ...");

		let res = if replace {
			method = "put";
			client.put(&url).bearer_auth(auth_token).json(&obj).send().await?
		} else {
			method = "patch";
			client.patch(&url).bearer_auth(auth_token).json(&obj).send().await?
		};

		#[cfg(debug_assertions)]
		println!("[import_document put/patch] ... exited");

		let status = res.status();

		let obj = if status != StatusCode::OK {
			let text = res.text().await?;

			if text.starts_with("Missing data in") {
				post(schema_name_snake, &obj).await?
			} else {
				eprintln!("[import_document.merge.publish()]\ncurl -X {method} {url} -d '{obj}' -H 'Content-Type: application/json' -H 'authorization: Bearer {auth_token}';\nerror message : {text}");
				return Err(text)?;
			}
		} else {
			let value = res.json::<Value>().await?;

			if let Some(list) = value.as_array() {
				if list.len() == 0 {
					post(schema_name_snake, &obj).await?
				} else {
					return Err(format!("Bad put/patch response : {:?}", list))?;
				}
			} else {
				value
			}
		};

		#[cfg(debug_assertions)]
		println!("[import_document.publish({schema_name_snake})] ... finished");
		return Ok(obj);
	};

	let person = document.get("person").ok_or("Broken person")?;
	publish("person", &["cnpjCpf"], &[], &person, false).await?;
	let person_dest = document.get("personDest").ok_or("Broken personDest")?;
	publish("person", &["cnpjCpf"], &[], person_dest, false).await?;

	if let Some(value) = document.get("nfeId") {
		let nfe_id = value.as_str().ok_or("Expected 'string' type")?;
		let url = format!("{url_base}request_nfe?filter[nfeId]={nfe_id}");
		#[cfg(debug_assertions)]
		println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
		let res = client.get(&url).bearer_auth(auth_token).send().await?;
		let status = res.status();

		if status == StatusCode::OK {
			let list = res.json::<Value>().await?;
			let list = list.as_array().ok_or("Bad format, expected list!")?;

			if list.len() > 0 {
				let ret = list.get(0).ok_or("broken")?;
				println!("[import_document.merge] NFE {nfe_id} already processed, exiting !");
				return Ok(ret.clone());
			}
		} else {
			return Err(res.text().await?)?;
		}
	} else if require_request_nfe {
		return Err("Missing field nfeId")?;
	}

	let date = document.get("date").ok_or("Missing field 'date' in 'document'")?.as_str().ok_or("field 'date' is not 'str'")?.to_string()+"+03:00";

	let person_dest_cnpj_cpf = if let Some(value) = person_dest.get("cnpjCpf") {
		value.as_str().ok_or("cnpjCpf : Bad type")?.to_string()
	} else if let Some(value) = person_dest.get("idImport") {
		value.as_str().ok_or("idImport : Bad type")?.to_string()
	} else {
		return Err("Missing cnpjCpf in person_dest")?;
	};

	let mut request = {
		let person_cnpj_cpf = person.get("cnpjCpf").ok_or("Missing cnpjCpf in person")?.as_str().ok_or("Bad type")?.to_string();
		let date_min = DateTime::parse_from_rfc3339(&date)?.checked_sub_months(Months::new(1)).ok_or("broken 'date_min'")?.to_rfc3339();
		//let date_max = DateTime::parse_from_rfc3339(&date)?.checked_add_days(Days::new(1)).ok_or("broken")?.to_rfc3339();
		let url = format!("{url_base}request?filter[person]={person_cnpj_cpf}&filter[personDest]={person_dest_cnpj_cpf}&filter[paymentsValue]=0&filterRangeMin[date]={date_min}&filterRangeMax[date]={date}");
		#[cfg(debug_assertions)]
		println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
		let res = client.get(&url).bearer_auth(auth_token).send().await?;
		let status = res.status();

		if status == StatusCode::OK {
			let list = res.json::<Value>().await?;
			let list = list.as_array().ok_or("Bad format, expected list!")?;

			match list.len() {
				0 => {
					document["state"] = json!(80);
					document["personDest"] = json!(person_dest_cnpj_cpf);
					document["person"] = json!(person_cnpj_cpf);
					post("request", &document).await?
				},
				_ => list.last().ok_or("Broken")?.clone(),
			}
		} else {
			return Err(res.text().await?)?;
		}
	};

	let request_id = request.get("id").ok_or("Missing 'id' in 'request'.")?.clone();

	if let Some(obj) = document.get("requestRepair") {
		let mut request_repair = convert_obj(obj, &["serial"])?;
		request_repair["request"] = request_id.clone();

		let Some(name) = request_repair.get("name") else {
			eprintln!("[import_document] Missing field 'name' in {request_repair}");
			return Err("[import_document] Missing field 'name' in 'request_repair'.")?;
		};

		let name = name.as_str().ok_or("'product' not 'str'")?;
		let url = format!("{url_base}product?filter[name]={name}");
		#[cfg(debug_assertions)]
		println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
		let mut product_id = 0;
		let res = client.get(&url).bearer_auth(auth_token).send().await?;
		let status = res.status();

		if status == StatusCode::OK {
			let list = res.json::<Value>().await?;
			let list = list.as_array().ok_or("Bad format, expected list!")?;

			if list.len() > 0 {
				product_id = list.get(0).ok_or("Broken")?.get("id").ok_or("Missing field 'product'")?.as_u64().ok_or("'id' not u64")?;
				request_repair["product"] = json!(product_id);
			}
		} else {
			return Err(res.text().await?)?;
		}

		if product_id == 0 {
			product_id = post("product", &request_repair).await?.get("id").ok_or("Missing field 'id'")?.as_u64().ok_or("'id ' is not u64")?;
			request_repair["product"] = json!(product_id);
		} else {
			let url = format!("{url_base}product?id={product_id}");
			#[cfg(debug_assertions)]
			println!("[import_document.merge] curl -X patch '{url}' -H 'Authorization: Bearer {auth_token}';");
			let res = client.patch(url).bearer_auth(auth_token).json(&request_repair).send().await?;

			if res.status() != StatusCode::OK {
				return Err(res.text().await?)?;
			}
		}

		let _item = publish("request_repair", &["request"], &["serial"], &request_repair, true).await?;
	}

	let mut value_products = 0.0;
	let mut value_services = 0.0;

	fn add_f64(item_out: &mut Value, name_out: &str, item_in: &Value, name_in: &str) -> Result<f64, Box<dyn std::error::Error>> {
		let value_in = if let Some(value) = item_in.get(name_in) {
			value.as_f64().ok_or("broken value as f64")?
		} else {
			0.0
		};

		let value_out = if let Some(value_to_publish) = item_out.get(name_out) {
			let value_to_publish = value_to_publish.as_f64().unwrap_or_default() + value_in;
			item_out[name_out] = json!(value_to_publish);
			value_to_publish
		} else {
			item_out[name_out] = json!(value_in);
			value_in
		};

		Ok(value_out)
	}

	{
		let list = document.get("requestProduct").ok_or("Broken request_product")?.as_array().ok_or("expected array")?;
		let mut list_to_publish = vec![];

		for obj in list {
			if obj.get("quantity").is_none() {
				continue;
			}

			let mut request_product = convert_obj(obj, &[])?;
			request_product["request"] = request_id.clone();
			request_product["serials"] = json!("");
			let mut product_id = 0;
			let mut do_patch_product = true;
			let mut barcode_valid = false;

			if let Some(barcode) = request_product.get_mut("barcode") {
				let regex_barcode = Regex::new(r#"^\d{1,14}$"#)?;//"^0?()   \d{1,14}$"
				let barcode_string = barcode.as_str().ok_or("Brokn barcode typ")?.to_string();

				if regex_barcode.is_match(&barcode_string) {
					let barcode_str = if barcode_string.len() == 14 && barcode_string.starts_with("0") {
						&barcode_string[1..]
					} else {
						&barcode_string
					};

					let barcode_str = if barcode_str.len() == 13 && barcode_str.starts_with("00000") {
						&barcode_str[5..]
					} else {
						barcode_str
					};

					if barcode_str.len() == 13 && barcode_str.starts_with("2") == false && barcode_str.starts_with("0000") == false {
						barcode_valid = true;
						*barcode = json!(barcode_str);
					} else if barcode_str.len() == 8 && barcode_str.starts_with("2") == false && barcode_str.starts_with("0") == false {
						barcode_valid = true;
						*barcode = json!(barcode_str);
					}

					let url = format!("{url_base}barcode?barcode={barcode_str}");
					#[cfg(debug_assertions)]
					println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
					let res = client.get(&url).bearer_auth(auth_token).send().await?;
					let status = res.status();

					if status == StatusCode::OK {
						let list = res.json::<Value>().await?;
						let list = list.as_array().ok_or("Bad format, expected list!")?;

						if list.len() > 0 {
							product_id = list.get(0).ok_or("Broken")?.get("product").ok_or("Missing field 'product'")?.as_u64().ok_or("'product' not 'u64'")?;
							request_product["product"] = json!(product_id);
						}
					} else {
						return Err(res.text().await?)?;
					}
				}
			}

			if product_id == 0 {
				let Some(name) = request_product.get("name") else {
					eprintln!("[import_document] Missing field 'name' in {request_product}");
					return Err("[import_document] Missing field 'name' in 'request_product'.")?;
				};

				let name = name.as_str().ok_or("'product' not 'str'")?;
				let url = format!("{url_base}product?filter[name]={name}");
				#[cfg(debug_assertions)]
				println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
				let res = client.get(&url).bearer_auth(auth_token).send().await?;
				let status = res.status();

				if status == StatusCode::OK {
					let list = res.json::<Value>().await?;
					let list = list.as_array().ok_or("Bad format, expected list!")?;

					if list.len() > 0 {
						product_id = list.get(0).ok_or("Broken")?.get("id").ok_or("Missing field 'product'")?.as_u64().ok_or("'id' not u64")?;
						request_product["product"] = json!(product_id);
					}
				} else {
					return Err(res.text().await?)?;
				}

				if product_id == 0 {
					do_patch_product = false;
					product_id = post("product", &request_product).await?.get("id").ok_or("Missing field 'id'")?.as_u64().ok_or("'id ' is not u64")?;
					request_product["product"] = json!(product_id);
				}

				if barcode_valid {
					let _barcode = publish("barcode", &["barcode"], &[], &request_product, false).await?;
				}
			}

			if do_patch_product {
				let url = format!("{url_base}product?id={product_id}");
				#[cfg(debug_assertions)]
				println!("[import_document.merge] curl -X patch '{url}' -H 'Authorization: Bearer {auth_token}';");
				let res = client.patch(url).bearer_auth(auth_token).json(&request_product).send().await?;

				if res.status() != StatusCode::OK {
					return Err(res.text().await?)?;
				}
			}

			fn compare_request_product(a: &Value, b: &Value) -> bool {
				let ret = (a.get("product") == b.get("product")) && (a.get("serials") == b.get("serials"));
				ret
			}

			if let Some(item_to_publish) = list_to_publish.iter_mut().find(|x| compare_request_product(x, &request_product)) {
				add_f64(item_to_publish, "quantity", &request_product, "quantity")?;
				add_f64(item_to_publish, "valueItem", &request_product, "valueItem")?;
				add_f64(item_to_publish, "valueDesc", &request_product, "valueDesc")?;
				add_f64(item_to_publish, "valueFreight", &request_product, "valueFreight")?;
				add_f64(item_to_publish, "valueAllTax", &request_product, "valueAllTax")?;
				item_to_publish["merged"] = json!(true);
			} else {
				list_to_publish.push(request_product);
			}
		}

		for request_product in list_to_publish.iter_mut() {
			let value_desc = if let Some(value) = request_product.get("valueDesc") {
				value.as_f64().ok_or("broken valieItem f64")?
			} else {
				0.0
			};

			let value_item = request_product.get("valueItem").ok_or("broken valueItem")?.as_f64().ok_or("broken valieItem f64")?;
			value_products += value_item - value_desc;
			#[cfg(debug_assertions)]
			println!("value_products {value_products} += value_item {value_item} - value_desc {value_desc};");

			if request_product.get("merged").is_none() == false {
				let quantity = request_product.get("quantity").ok_or("broken quantity")?.as_f64().ok_or("broken quantity f64")?;

				if quantity != 0.0 {
					let value = (value_item * 100.0) / quantity;
					request_product["value"] = json!(value.round() / 100.0);
				}
			}

			let _item = publish("request_product", &["request", "product", "serials"], &[], &request_product, true).await?;
		}
	}

	if let Some(list) = document.get("requestService") {
		let list = list.as_array().ok_or("expected array")?;
		let mut list_to_publish = vec![];

		for obj in list {
			if obj.get("quantity").is_none() {
				continue;
			}

			let mut request_service = convert_obj(obj, &[])?;
			request_service["request"] = request_id.clone();

			let Some(name) = request_service.get("name") else {
				eprintln!("[import_document] Missing field 'name' in {request_service}");
				return Err("[import_document] Missing field 'name' in 'request_service'.")?;
			};

			let name = name.as_str().ok_or("'service' not 'str'")?;
			let url = format!("{url_base}service?filter[name]={name}");
			#[cfg(debug_assertions)]
			println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
			let mut service_id = 0;
			let res = client.get(&url).bearer_auth(auth_token).send().await?;
			let status = res.status();

			if status == StatusCode::OK {
				let list = res.json::<Value>().await?;
				let list = list.as_array().ok_or("Bad format, expected list!")?;

				if list.len() > 0 {
					service_id = list.get(0).ok_or("Broken")?.get("id").ok_or("Missing field 'service'")?.as_u64().ok_or("'id' not u64")?;
					request_service["service"] = json!(service_id);
				}
			} else {
				return Err(res.text().await?)?;
			}

			if service_id == 0 {
				service_id = post("service", &request_service).await?.get("id").ok_or("Missing field 'id'")?.as_u64().ok_or("'id ' is not u64")?;
				request_service["service"] = json!(service_id);
			} else {
				let url = format!("{url_base}service?id={service_id}");
				#[cfg(debug_assertions)]
				println!("[import_document.merge] curl -X patch '{url}' -H 'Authorization: Bearer {auth_token}';");
				let res = client.patch(url).bearer_auth(auth_token).json(&request_service).send().await?;

				if res.status() != StatusCode::OK {
					return Err(res.text().await?)?;
				}
			}

			fn compare_request_service(x: &Value, request_service: &Value) -> bool {
				let ret = x.get("service") == request_service.get("service");
				ret
			}

			if let Some(item_to_publish) = list_to_publish.iter_mut().find(|x| compare_request_service(x, &request_service)) {
				add_f64(item_to_publish, "quantity", &request_service, "quantity")?;
				add_f64(item_to_publish, "valueItem", &request_service, "valueItem")?;
				add_f64(item_to_publish, "valueDesc", &request_service, "valueDesc")?;
				add_f64(item_to_publish, "valueFreight", &request_service, "valueFreight")?;
				item_to_publish["merged"] = json!(true);
			} else {
				list_to_publish.push(request_service);
			}
		}

		for request_service in list_to_publish.iter_mut() {
			let value_desc = if let Some(value) = request_service.get("valueDesc") {
				value.as_f64().ok_or("broken valieItem f64")?
			} else {
				0.0
			};

			let value_item = request_service.get("valueItem").ok_or("broken valueItem")?.as_f64().ok_or("broken valieItem f64")?;
			value_services += value_item - value_desc;
			#[cfg(debug_assertions)]
			println!("value_services {value_services} += value_item {value_item} - value_desc {value_desc};");

			if request_service.get("merged").is_none() == false {
				let quantity = request_service.get("quantity").ok_or("broken quantity")?.as_f64().ok_or("broken quantity f64")?;

				if quantity != 0.0 {
					let value = (value_item * 100.0) / quantity;
					request_service["value"] = json!(value.round() / 100.0);
				}
			}

			let _item = publish("request_service", &["request", "service"], &[], &request_service, true).await?;
		}
	}

	let map_accounts = std::collections::HashMap::from([
		(1, "Caixa interno"),
		(2, "Conta corrente"),
		(3, "Cartão de Crédito"),
		(4, "Conta corrente"),
		(5, "Caixa interno"),
		(10, "Vale Alimentação"),
		(11, "Vale Refeição"),
		(12, "Vale Presente"),
		(13, "Vale Combustível"),
		(14, "Conta corrente"),
		(15, "Conta corrente"),
		(17, "Conta corrente"),
		(20, "Conta corrente"),
	]);

	let list_payments = document.get("requestPayment").ok_or("[import_document_document] Missing field 'request_payment'")?.as_array().ok_or("'request_payment' is not array.")?;
	let mut value_payments = 0.0;

	for request_payment in list_payments {
		let account = {
			let payment_type = request_payment.get("type").ok_or("Missing field 'type'.")?.as_u64().ok_or("field 'type' is not 'u64'")?;
			let description = map_accounts.get(&payment_type).unwrap_or(&"Caixa interno");
			let url = format!("{url_base}account?person={person_dest_cnpj_cpf}&description={description}");
			#[cfg(debug_assertions)]
			println!("[import_document.merge] curl -X GET '{url}' -H 'Authorization: Bearer {auth_token}';");
			let res = client.get(&url).bearer_auth(auth_token).send().await?;

			if res.status() == StatusCode::OK {
				let list = res.json::<Value>().await?;
				let list = list.as_array().ok_or("Bad format, expected list!")?;

				if list.len() > 0 {
					list.get(0).ok_or("broken request_payment list index 0")?.clone()
				} else {
					let account = json!({"person": person_dest_cnpj_cpf, "description": description});
					#[cfg(debug_assertions)]
					println!("[import_document.merge] curl -X post '{url}' -H 'Authorization: Bearer {auth_token}';");
					let res = client.post(url_base.clone() + "account").bearer_auth(auth_token).json(&account).send().await?;
					let status = res.status();

					if status == StatusCode::OK {
						res.json::<Value>().await?
					} else {
						return Err(res.text().await?)?;
					}
				}
			} else {
				return Err(res.text().await?)?;
			}
		};

		let mut request_payment = convert_obj(request_payment, &[])?;
		request_payment["account"] = account.get("id").ok_or("Missing field 'id' in account")?.clone();
		request_payment["request"] = request_id.clone();
		request_payment["dueDate"] = json!(date);
		post("request_payment", &request_payment).await?;

		value_payments += if let Some(value) = request_payment.get("value") {
			value.as_f64().ok_or("broken value as f64")?
		} else {
			0.0
		};
	}

	request["paymentsValue"] = json!(value_payments);
	let mut request_freight = document.get("requestFreight").ok_or("Broken request_freight")?.clone();

	if let Some(value) = request_freight.get("cnpjCpf") && let Some(cnpj_cpf) = value.as_str() {
		publish("person", &["cnpjCpf"], &[], &request_freight, false).await?;
		request_freight["person"] = json!(cnpj_cpf);
	}

	if let Some(value_frete) = request.get("transportValue") {
		if request_freight.get("value").is_none() {
			request_freight["value"] = value_frete.clone();
		}
	}

	let transport_value = request_freight.get("value").ok_or("Broken request_freight.value")?.as_f64().unwrap_or(0.0);

	if transport_value > 0.0 {
		request["transportValue"] = json!(transport_value);
		request_freight["request"] = request_id.clone();
		publish("request_freight", &["request"], &[], &request_freight, true).await?;
	} else if let Some(_containers_count) = request_freight.get("containersCount") {
		request_freight["request"] = request_id.clone();
		publish("request_freight", &["request"], &[], &request_freight, true).await?;
	}

	let total_value = f64::round(value_products * 100.0 + value_services * 100.0 + transport_value * 100.0);

	if check_payments && f64::round(value_payments * 100.0) != total_value {
		return Err(format!("total_value ({total_value}) !=  value_payments ({value_payments}) !"))?;
	}

	let request = publish("request", &["id"], &[], &request, true).await?;

	if document.get("nfeId").is_none() {
		return Ok(request);
	}

	document["request"] = request_id.clone();
	post("request_nfe", &document).await
}
