use chrono::NaiveDateTime;
use rufs_base_rust::client::{DataView, DataViewWatch, ServerConnection, DataViewFormType, HtmlElementId};
#[cfg(target_arch = "wasm32")]
use rufs_base_rust::client::DataViewManagerWrapper;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(target_arch = "wasm32")]
use web_log::println;

pub struct RufsNfe {}

#[derive(Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
     id              : usize,
     #[serde(rename = "type")]
     typ            : usize,
     state           : usize,
     person           : String,
     person_dest      : String,
     date             : NaiveDateTime,
     additional_data  : Option<String>,
     products_value   : Option<f64>,
     services_value   : Option<f64>,
     transport_value  : Option<f64>,
     desc_value       : Option<f64>,
     sum_value        : Option<f64>,
     payments_value   : Option<f64>,
 }

#[derive(Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestProduct {
    id :Option<usize>,
    request :usize,
    product :Option<usize>,
    quantity :f64,
    #[serde(default)]
    value :f64,
    value_item :Option<f64>,
    value_desc :Option<f64>,
    value_freight :Option<f64>,
    cfop :Option<usize>,
    value_all_tax :Option<f64>,
    serials :Option<String>,
 }

impl RufsNfe {

    async fn request_payment_adjusts(data_view_payment : &mut DataView, watcher: &dyn DataViewWatch, server_connection: &ServerConnection, request: &Request, typ :Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
        let remaining_payment = request.sum_value.unwrap_or(0.0) - request.payments_value.unwrap_or(0.0);

        if data_view_payment.filter_results.len() == 0 {
            let value = json!(remaining_payment);
            //println!("[request_payment_adjusts] : value  = {}", value);
            data_view_payment.set_value(server_connection, watcher, "value", &value, None).await?;
        }

        let account = data_view_payment.params.instance.get("account").unwrap_or(&Value::Null);
        println!("[request_payment_adjusts] : old account  = {}", account);

        if account.is_null() {
            let typ = if let Some(typ) = typ {
                typ
            } else {
                data_view_payment.params.instance.get("type").unwrap_or(&json!(1)).as_u64().unwrap_or(1)
            };

            let accounts = data_view_payment.field_results_str.get("account").ok_or("expected list of accounts")?;

            if typ == 1 {
                if accounts.len() > 0 {
                    let account = server_connection.get_item_from_description("account", &accounts[accounts.len()-1])?.ok_or("Broken xxx")?;
                    let account_id = account.get("id").ok_or("missing field id in account")?;
                    data_view_payment.set_value(server_connection, watcher, "account", account_id, None).await?;
                }
            } else {
                if accounts.len() > 1 {
                    let account = server_connection.get_item_from_description("account", &accounts[accounts.len()-2])?.ok_or("Broken xxx")?;
                    let account_id = account.get("id").ok_or("missing field id in account")?;
                    data_view_payment.set_value(server_connection, watcher, "account", account_id, None).await?;
                }
            }
        }

        Ok(())
    }

}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DataViewWatch for RufsNfe {

    async fn check_set_value(&self, data_view: &mut DataView, element_id: &HtmlElementId, server_connection: &ServerConnection, field_name: &str, field_value: &Value) -> Result<bool, Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        println!("[RufsNfe::check_set_value] 1");

        if data_view.data_view_id.schema_name == "request" {
            let schema_name = &element_id.data_view_id.schema_name;

            #[cfg(debug_assertions)]
            println!("[RufsNfe::check_set_value] 1.1");

            if schema_name == "requestProduct" && ["quantity", "value", "valueDesc"].contains(&field_name) {
                #[cfg(debug_assertions)]
                println!("[RufsNfe::check_set_value] 1.1.1");

                if let Some(data_view_child) = data_view.childs.iter_mut().find(|item| &item.data_view_id.schema_name == schema_name) {
                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.1");

                    if data_view_child.params.instance.get("product").is_none() {
                        #[cfg(debug_assertions)]
                        println!("[RufsNfe::check_set_value] 1.1.1.1.1");

                        return Ok(true);
                    }

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.2");

                    if field_name != "value" && data_view_child.params.instance.get("value").is_none() {
                        #[cfg(debug_assertions)]
                        println!("[RufsNfe::check_set_value] 1.1.1.2.1");

                        // TODO : se valor unitário está ausente, pegar o valor do cadastro de produtos.
                        data_view_child.set_value(server_connection, self, "value", &json!(0.0), None).await?;
                    }

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.3");

                    if field_name != "quantity" && data_view_child.params.instance.get("quantity").is_none() {
                        data_view_child.set_value(server_connection, self, "quantity", &json!(1.0), None).await?;
                    }

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.4");

                    if field_name != "valueDesc" && data_view_child.params.instance.get("valueDesc").is_none() {
                        data_view_child.set_value(server_connection, self, "valueDesc", &json!(0.0), None).await?;
                    }

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.5");

                    let field_value :f64 = match field_value {
                        Value::Number(field_value) => field_value.as_f64().ok_or("expected type is f64")?,
                        Value::String(str) => str.parse::<f64>()?,
                        Value::Null => todo!(),
                        Value::Bool(_) => todo!(),
                        Value::Array(_values) => todo!(),
                        Value::Object(_map) => todo!(),
                    };

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.6");

                    let mut request_product: RequestProduct = match serde_json::from_value(data_view_child.params.instance.clone()) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("[RufsNfe::check_set_value] serde_json::from_value({}) : {err}", data_view_child.params.instance);
                            return Err(err)?;
                        }
                    };

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.7");

                    let value_item = if field_name == "quantity" {
                        let value_desc = request_product.value_desc.unwrap_or(0.0);
                        (field_value * request_product.value) - value_desc
                    } else if field_name == "value" {
                        let value_desc = request_product.value_desc.unwrap_or(0.0);
                        (100.0 * request_product.quantity * field_value).trunc() / 100.0 - value_desc
                    } else if field_name == "valueDesc" {
                        (100.0 * request_product.quantity * request_product.value).trunc() / 100.0 - field_value
                    } else {
                        request_product.value_item.unwrap_or(0.0)
                    };

                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.1.1.8 : value_item = {value_item}");

                    data_view_child.set_value(server_connection, self, "valueItem", &json!(value_item), None).await?;

                    match field_name {
                        "quantity" => request_product.quantity = field_value,
                        "value" => request_product.value = field_value,
                        "valueDesc" => request_product.value_desc = Some(field_value),
                        _ => todo!()
                    }
                }
            }

            #[cfg(debug_assertions)]
            println!("[RufsNfe::check_set_value] 1.2");

            if schema_name == "requestPayment" && ["type"].contains(&field_name) {
                #[cfg(debug_assertions)]
                println!("[RufsNfe::check_set_value] 1.2.1");

                if let Some(data_view_child) = data_view.childs.iter_mut().find(|item| &item.data_view_id.schema_name == schema_name) {
                    #[cfg(debug_assertions)]
                    println!("[RufsNfe::check_set_value] 1.2.1.1");

                    let typ = field_value.as_u64().unwrap_or(1);
                    // due_date
                    if [1,4,10,11,12,13].contains(&typ) {
                        let value = data_view.params.instance.get("date").ok_or("check_set_value 1 : context")?;
                        data_view_child.set_value(server_connection, self, "dueDate", value, None).await?;
                    }
                    // payday
                    if [1,4,10,11,12,13].contains(&typ) {
                        let value = data_view.params.instance.get("date").ok_or("check_set_value 2 : context")?;
                        //data_view_child.params.instance["payday"] = value.clone();
                        data_view_child.set_value(server_connection, self, "payday", value, None).await?;
                    }

                    let request: Request = serde_json::from_value(data_view.params.instance.clone())?;
                    RufsNfe::request_payment_adjusts(data_view_child, self, server_connection, &request, Some(typ)).await?;
                }
            }
        }

        Ok(true)
    }

    async fn check_save(&self, data_view :&mut DataView, element_id: &HtmlElementId, server_connection: &mut ServerConnection) -> Result<(bool, DataViewFormType), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        println!("[RufsNfe::check_save] start, element_id = {:?} ...", element_id);

        if data_view.data_view_id.schema_name == "request" {
            let schema_name = &element_id.data_view_id.schema_name;

            #[cfg(debug_assertions)]
            println!("[RufsNfe::check_save] 1.1");

            if let Some(data_view_child) = data_view.childs.iter().find(|item| item.data_view_id.id == element_id.data_view_id.id) {
                if schema_name == "requestProduct" {
                    let instance = if let Some(action_exec) = &element_id.action_exec && action_exec == "delete" {
                        let primary_key = data_view_child.params.primary_key.as_ref().ok_or("[RufsNfe.check_save] Broken primary_key")?;
                        server_connection.get(schema_name, primary_key).await?.ok_or("Broken primary_key.")?
                    } else {
                        data_view_child.params.instance.clone()
                    };

                    let request_product: RequestProduct = match serde_json::from_value(instance) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("[RufsNfe::check_save] serde_json::from_value({}) : {err}", data_view_child.params.instance);
                            return Err(err)?;
                        }
                    };

                    let scale = if let Some(action_exec) = &element_id.action_exec && action_exec == "delete" {
                        -1.0
                    } else {
                        1.0
                    };

                    let product_value_new = f64::trunc((request_product.quantity * request_product.value * 100.0) + 0.1) / 100.0;
                    let request: Request = serde_json::from_value(data_view.params.instance.clone())?;
                    let products_value_old = f64::trunc((request.products_value.unwrap_or(0.0) * 100.0) + 0.1) / 100.0;
                    let request_products_value_new = products_value_old + scale * product_value_new;
                    data_view.set_value(server_connection, self, "productsValue", &json!(request_products_value_new), None).await?;
                    let desc_value_old = f64::trunc((request.desc_value.unwrap_or(0.0) * 100.0) + 0.1) / 100.0;
                    let product_desc_value_new = f64::trunc((request_product.value_desc.unwrap_or(0.0) * 100.0) + 0.1) / 100.0;
                    let request_desc_value_new = desc_value_old + scale * product_desc_value_new;
                    data_view.set_value(server_connection, self, "descValue", &json!(request_desc_value_new), None).await?;
                    data_view.set_value(server_connection, self, "sumValue", &json!(request_products_value_new - request_desc_value_new), None).await?;
                    let data_view_payment = data_view.childs.iter_mut().find(|item| item.data_view_id.schema_name == "requestPayment").ok_or_else(|| format!("Missing child {} in parent {}", "requestPayment", data_view.data_view_id.schema_name))?;                        
                    let request: Request = serde_json::from_value(data_view.params.instance.clone())?;
                    RufsNfe::request_payment_adjusts(data_view_payment, self, server_connection, &request, None).await?;
                } else if schema_name == "requestPayment" {
                    let payments_value_old = data_view.params.instance.get("paymentsValue").unwrap_or(&json!(0.0)).as_f64().unwrap_or(0.0);
                    let payment_value = data_view_child.params.instance.get("value").unwrap_or(&json!(0.0)).as_f64().unwrap_or(0.0);

                    let scale = if let Some(action_exec) = &element_id.action_exec && action_exec == "delete" {
                        -1.0
                    } else {
                        1.0
                    };

                    let payments_value_new = payments_value_old + scale * payment_value;
                    data_view.set_value(server_connection, self, "paymentsValue", &json!(payments_value_new), None).await?;
                }
            }
        }

        let form_type = if ["new-rufs_user", "new-request"].contains(&element_id.data_view_id.id.as_str()) {
            DataViewFormType::Edit
        } else {
            DataViewFormType::Search
        };

        #[cfg(debug_assertions)]
        println!("[RufsNfe::check_save] ... exit.");
        Ok((true, form_type))
    }

}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = DataViewManager)]
pub struct DataViewManagerWrapperApp {
    data_view_manager_wrapper :DataViewManagerWrapper,
    watcher :Box<dyn DataViewWatch>
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = DataViewManager)]
impl DataViewManagerWrapperApp {

    #[wasm_bindgen(constructor)]
	pub fn new(path: &str) -> Self {
        use rufs_base_rust::client::DataViewManager;
        let data_view_manager = DataViewManager::new(path);
        let data_view_manager_wrapper = DataViewManagerWrapper{data_view_manager};
        let watcher: Box<dyn DataViewWatch> = Box::new(RufsNfe{});
        Self {data_view_manager_wrapper, watcher}
    }

	pub async fn login_from_response(&mut self, params :JsValue) -> Result<JsValue, JsValue> {
        self.data_view_manager_wrapper.login_from_response(params).await
    }

    pub async fn process(&mut self, params :JsValue) -> Result<JsValue, JsValue> {
        self.data_view_manager_wrapper.process(params, &self.watcher).await
    }

}
