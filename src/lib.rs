use anyhow::{Context};
use chrono::NaiveDateTime;
use rufs_crud_rust::{DataView, DataViewWatch, ServerConnection, DataViewProcessAction};
#[cfg(target_arch = "wasm32")]
use rufs_crud_rust::DataViewManagerWrapper;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
//use convert_case::Casing;
#[cfg(target_arch = "wasm32")]
use web_log::println;

pub struct RufsNfe {}

#[derive(Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
     rufs_group_owner: usize,
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
    rufs_group_owner :usize,
    request :usize,
    product :usize,
    quantity :f64,
    value :f64,
    value_item :Option<f64>,
    value_desc :Option<f64>,
    value_freight :Option<f64>,
    cfop :Option<usize>,
    value_all_tax :Option<f64>,
    serials :Option<String>,   
 }

impl RufsNfe {
    
    fn request_payment_adjusts(data_view_payment : &mut DataView, watcher : &dyn DataViewWatch, server_connection: &ServerConnection, request: &Request, typ :Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
        let remaining_payment = request.sum_value.unwrap_or(0.0) - request.payments_value.unwrap_or(0.0);
        let value = data_view_payment.instance.get("value").unwrap_or(&json!(0.0)).as_f64().unwrap_or(0.0);

        if value == 0.0 {
            let value = json!(remaining_payment);
            data_view_payment.set_value(None, server_connection, watcher, "value", &value)?;
        }

        let account = data_view_payment.instance.get("account").unwrap_or(&Value::Null);
        println!("[request_payment_adjusts] 5 : old account  = {}", account);

        if account.is_null() {
            let accounts = data_view_payment.field_results.get("account").context("expected list of accounts")?;

            let typ = if let Some(typ) = typ {
                typ
            } else {
                data_view_payment.instance.get("type").unwrap_or(&json!(1)).as_u64().unwrap_or(1)
            };
    
            if typ == 1 {
                if accounts.len() > 0 {
                    let account = accounts[accounts.len()-1].get("id").context("missing field id in account")?.clone();//accounts[0].id;//
                    data_view_payment.set_value(None, server_connection, watcher, "account", &account)?;
                }
            } else {
                if accounts.len() > 1 {
                    let account = accounts[accounts.len()-2].get("id").context("missing field id in account")?.clone();//accounts.len()-2
                    data_view_payment.set_value(None, server_connection, watcher, "account", &account)?;
                }
            }
        }

        Ok(())
    }

}

impl DataViewWatch for RufsNfe {

    fn check_set_value(&self, data_view :&mut DataView, child_name: Option<&str>, server_connection: &ServerConnection, field_name: &str, field_value: &Value) -> Result<bool, Box<dyn std::error::Error>> {
        println!("check_set_value 1 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);

        if data_view.element_id.schema_name == "request" {
            println!("check_set_value 1.1 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);

            if let Some(child_name) = child_name {
                println!("check_set_value 1.1.1 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);
                println!("check_set_value 1.1.2 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);

                if child_name == "requestProduct" && data_view.instance.get("product").is_some() && ["quantity", "value"].contains(&field_name) {
                    if let Some(data_view) = data_view.childs.iter_mut().find(|item| item.element_id.schema_name == child_name) {
                        if data_view.instance.get("value").is_none() {
                            // TODO : se valor unitário está ausente, pegar o valor do cadastro de produtos.
                            data_view.set_value(None, server_connection, self, "value", &json!(0.0))?;
                        }

                        if data_view.instance.get("quantity").is_none() {
                            data_view.set_value(None, server_connection, self, "quantity", &json!(1.0))?;
                        }

                        let field_value :f64 = match field_value {
                            Value::Number(field_value) => field_value.as_f64().context("expected type is f64")?,
                            _ => todo!(),
                        };

                        let mut request_product: RequestProduct = serde_json::from_value(data_view.instance.clone())?;

                        if field_name == "quantity" {
                            request_product.value_item = Some(request_product.value * field_value);
                        } else if field_name == "value" {
                            request_product.value_item = Some(request_product.quantity * field_value);
                        }

                        data_view.instance = serde_json::to_value(request_product)?;
                    }
                }

                println!("check_set_value 1.1.3 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);

                if child_name == "requestPayment" && ["type"].contains(&field_name) {
                    println!("check_set_value 1.1.3.1 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);

                    if let Some(data_view_child) = data_view.childs.iter_mut().find(|item| item.element_id.schema_name == child_name) {
                        println!("check_set_value 1.1.3.1.1 {}.{:?}.{} = {}", data_view.element_id.schema_name, child_name, field_name, field_value);
                        let typ = field_value.as_u64().unwrap_or(1);
                        // due_date
                        if [1,4,10,11,12,13].contains(&typ) {
                            let value = data_view.instance.get("date").context("check_set_value 1 : context")?;
                            data_view_child.set_value(None, server_connection, self, "dueDate", value)?;
                        }
                        // payday
                        if [1,4,10,11,12,13].contains(&typ) {
                            let value = data_view.instance.get("date").context("check_set_value 2 : context")?;
                            //data_view_child.instance["payday"] = value.clone();
                            data_view_child.set_value(None, server_connection, self, "payday", value)?;
                        }

                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        println!("check_set_value 1.1.3.1.8 {}", data_view_child.instance);
                        RufsNfe::request_payment_adjusts(data_view_child, self, server_connection, &request, Some(typ))?;
                        println!("check_set_value 1.1.3.1.9 {}", data_view_child.instance);
                    }
                }
            } else {
                /*
                if ["sumValue"].contains(&field_name) {
                    if let Some(data_view_payment) = data_view.childs.iter_mut().find(|item| item.schema_name == "requestPayment") {
                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        RufsNfe::request_payment_adjusts(data_view_payment, self, server_connection, &request, None)?;
                    }
                }
                 */
            }
        }

        Ok(true)
    }
     
    fn check_save(&self, data_view :&mut DataView, child_name: Option<&str>, server_connection: &ServerConnection) -> Result<(bool, DataViewProcessAction), Box<dyn std::error::Error>> {
        let action = if ["rufsUser", "request"].contains(&data_view.element_id.schema_name.as_str()) {
            if let Some(schema_name_child) = child_name {
                if data_view.element_id.schema_name == "request" {

                    if schema_name_child == "requestProduct" {
                        let item = data_view.childs.iter().find(|item| item.element_id.schema_name == schema_name_child).context(format!("Missing child {} in parent {}", schema_name_child, data_view.element_id.schema_name))?;
                        println!("[RufsNfe.check_save.request.requestProduct] 1 : instance = {}", item.instance);
                        let request_product: RequestProduct = serde_json::from_value(item.instance.clone())?;
                        println!("[RufsNfe.check_save.request.requestProduct] 2 : RequestProduct = {:?}", request_product);
                        let product_value = f64::trunc(request_product.quantity * request_product.value * 1000.0) / 1000.0;
                        let products_desc_value = request_product.value_desc.unwrap_or(0.0);
                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        let products_value_old = request.products_value.unwrap_or(0.0);
                        let desc_value_old = request.desc_value.unwrap_or(0.0);
                        let sum_value_old = request.sum_value.unwrap_or(0.0);
                        data_view.set_value(None, server_connection, self, "productsValue", &json!(products_value_old + product_value))?;
                        data_view.set_value(None, server_connection, self, "descValue", &json!(desc_value_old - products_desc_value))?;
                        let sum_value = f64::trunc((sum_value_old + product_value - products_desc_value)*1000.0)/1000.0;
                        data_view.set_value(None, server_connection, self, "sumValue", &json!(sum_value))?;
                        let data_view_payment = data_view.childs.iter_mut().find(|item| item.element_id.schema_name == "requestPayment").context(format!("Missing child {} in parent {}", "requestPayment", data_view.element_id.schema_name))?;
                        let request: Request = serde_json::from_value(data_view.instance.clone())?;
                        RufsNfe::request_payment_adjusts(data_view_payment, self, server_connection, &request, None)?;
                    }
/*
                    let data_view_request_payment = data_view.childs.iter_mut().find(|item| item.schema_name == "requestPayment").context(format!("Missing child requestPayment in parent {}", data_view.element_id.schema_name))?;

                    if schema_name_child != "requestPayment" {
                    }
*/
                }
            }

            DataViewProcessAction::Edit
        } else {
            DataViewProcessAction::Search
        };

        Ok((true, action))
    }
     
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = DataViewManager)]
pub struct DataViewManagerWrapperNfe {
    data_view_manager_wrapper :DataViewManagerWrapper,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = DataViewManager)]
impl DataViewManagerWrapperNfe {

    #[wasm_bindgen(constructor)]
	pub fn new(path: &str) -> Self {
        let watcher  = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;

        Self {
            data_view_manager_wrapper :DataViewManagerWrapper::new(path, watcher),
        }
    }

	pub async fn login(&mut self, login_path: &str, user: &str, password: &str/*, callback_partial: CallbackPartial*/) -> Result<JsValue, JsValue> {
        let _ret = match self.data_view_manager_wrapper.data_view_manager.server_connection.login(login_path, user, password).await {
            Ok(ret) => ret,
            Err(err) => return Err(JsValue::from_str(&err.to_string())),
        };
        
        let menu = json!({
            "Cadastros": {
                "Clientes e Fornecedores": "person/search",
                "Produtos": "product/search",
                "Contas": "account/search",
                "Requisições": "request/search",
                "Usuários": "rufs_user/search",
            },
            "Movimento": {
                "Financeiro": "request_payment/search",
                "Estoque": "stock/search",
            },
            "Rotinas": {
                "Compra": "request/new?overwrite.type=1&overwrite.state=10",
                "Venda": "request/new?overwrite.type=2&overwrite.state=10",
                "Importar": "request/import?overwrite.type=1&overwrite.state=10",
            },
            "Tabelas": {
                "Confaz Cest": "confaz_cest/search"
            }
        });

        let login_response = json!({"menu": menu, "path": self.data_view_manager_wrapper.data_view_manager.server_connection.login_response.path});
        Ok(serde_wasm_bindgen::to_value(&login_response)?)
    }

    pub async fn process_click_target(&mut self, target: &str) -> Result<JsValue, JsValue> {
        self.data_view_manager_wrapper.process_click_target(target).await
    }

    pub async fn process_edit_target(&mut self, target: &str, value: &str) -> Result<JsValue, JsValue> {
        self.data_view_manager_wrapper.process_edit_target(target, value).await
    }

}

