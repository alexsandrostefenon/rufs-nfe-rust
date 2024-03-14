let dataViewManager;
let aggregateChartOptions = {};
let aggregateChart = {};

function data_view_show(form_id) {
	const div_id = `data_view-${form_id}`;
	let data_view = document.getElementById(div_id);

	if (data_view != null && data_view.hidden == true) {
		data_view.hidden = false;
	}
}

function data_view_hide(form_id) {
	const div_id = `data_view-${form_id}`;
	let data_view = document.getElementById(div_id);

	if (data_view != null && data_view.hidden == false) {
		data_view.hidden = true;
	}
}

function updateChanges(event, changes) {
	if (changes == null) {
		return;
	}
	
	if (changes instanceof Map) {
		changes = Object.fromEntries(changes);
	}

	console.log(changes);

	for (let [formId, fields] of Object.entries(changes)) {
		if (fields instanceof Map) {
			fields = Object.fromEntries(fields);
		}

		const instanceFormId = `${formId}`;
		const form = document.getElementById(instanceFormId);
		const divFormId = `div-${formId}`;
		const divForm = document.getElementById(divFormId);

		if (form == null) {
			console.error(`Missing form ${instanceFormId}`);
			continue;
		}

		if (divForm == null) {
			console.error(`Missing divForm ${divFormId}`);
			continue;
		}

		data_view_show(formId);
		
		for (let [fieldName, value] of Object.entries(fields)) {
			if (form.hidden == true) {
				form.hidden = false;
			}

			if (divForm.hidden == true) {
				divForm.hidden = false;
			}

			if (Array.isArray(value)) {
				console.error(`Unexpected array fild (${fieldName})`);
			} else if (typeof value === 'object') {
				console.error(`Unexpected array fild (${fieldName})`);
/*
				const fields = value;
				const form_child = form[fieldName];

				for (let [fieldName, value] of fields) {
					if (typeof value === 'object') {
					} else {
						const element = form_child[fieldName];

						if (element == null) {
							throw new Error(`Missing element ${fieldName} in form ${form_child.name} !`);
						}
						
						element.value = value;
					}
				}
	*/
			} else {
				const element = form.elements[fieldName];
				
				if (element == null) {
					throw new Error(`Missing element ${fieldName} in form ${form.name} !`);
				}
						
				element.value = value;

				for (let flagIndex = 0; flagIndex < 64; flagIndex++) {
					const elementFlag = form[`${fieldName}-${flagIndex}`];
					
					if (elementFlag == null) {
						break;
					}

					let flagValue = (value & (1 << flagIndex)) != 0;

					if (event.target != elementFlag) {
						if (fieldName == "mask" && flagIndex== 3 && flagValue == true)
							console.log(`${fieldName}-${flagIndex}.checked = ${flagValue};`);

						elementFlag.checked = flagValue;
					} else {
						setTimeout(() => elementFlag.checked = flagValue, 100);
					}
				}
			}
		}
	}
}

function updateTables(event, tables) {
	if (tables == null) {
		return;
	}
	
	if (tables instanceof Map) {
		tables = Object.fromEntries(tables);
	}

	for (let [formId, html] of Object.entries(tables)) {
		data_view_show(formId);
		const div = document.getElementById(`div-table--${formId}`);

		if (div == null) {
			console.error(`Missing table ${formId}`);
			continue;
		}

		div.innerHTML = html;
		
		if (div.hidden == true) {
			div.hidden = false;
		}

		const divForm = document.getElementById(`div-instance--${formId}`);
		
		if (divForm != null && divForm.hidden == true) {
			divForm.hidden = false;
		}
	}
}

var appOnChange = event => {
    let element = event.target;

	if (element.value == null) {
		console.error(`appOnChange with null value in field ${element.id}`);
		return;
	}

	if (element.type == "number" && element.value.length == 0) {
		console.error(`appOnChange with empty number value in field ${element.id}`);
		return;
	}

	let value = element.value;

	if (element.type == "checkbox") {
		value = element.checked.toString();
	}

	const form = element.form;

	if (form != null) {
		form.inert = true;
	}

	console.log(`appOnChange : ${element.id} =`, value);
	let data = {};
	data[element.id] = value;
	dataViewManager.process({form_id: element.id, event: "OnChange", data}).
	then(viewResponse => {
		if (viewResponse instanceof Map) {
			viewResponse = Object.fromEntries(viewResponse);
		}

		updateChanges(event, viewResponse.changes);
		updateTables(event, viewResponse.tables);
	}).catch(err => {
		console.error(err);
		document.querySelector('#http-working').hidden = true;
		document.querySelector('#http-error').innerHTML = err;
		document.querySelector('#http-error').hidden = false;
	}).then(() => {
		if (form != null) {
			form.inert = false;
		}
	});
}

var appOnClick = event => {
    let element = event.target;

	if (["i"].includes(element.localName)) {
		element = element.parentElement;
	}

	if (["button"].includes(element.localName) == false && element.href == null) {
		return;
	}

	if (["button", "a"].includes(element.localName) && element.dataset.bsToggle != null) {
		return;
	}

	if (element.type == "text") {
		return;
	}
	
	//event.stopPropagation();
	event.preventDefault();
	let target = element.id;

	if (target == "" && element.href != null && element.href.includes("#!")) {
		target = element.href;
	}

	if (target != null && target.length > 0 && target.startsWith("menu-") == false) {
		console.log("appOnClick : ", target);
		document.querySelector('#http-error').hidden = true;
		document.querySelector('#http-working').innerHTML = "Processando...";
		document.querySelector('#http-working').hidden = false;
		dataViewManager.process({form_id: target, event: "OnClick", data: {}}).
		then(viewResponse => {
			if (viewResponse instanceof Map) {
				viewResponse = Object.fromEntries(viewResponse);
			}

			console.log(viewResponse);
			const html = viewResponse.html;

			if (viewResponse.form_id != null && html != null && html.length > 0) {
				const div_id = `data_view-${viewResponse.form_id}`;
				let dataView = document.getElementById(div_id);

				if (dataView != null) {
					dataView.remove();
					dataView = null;
				}

				dataView = document.createElement("div");
				dataView.id = div_id;
				dataView.innerHTML = html;
				document.querySelector('#main').prepend(dataView);
			}

			document.querySelector('#http-working').hidden = true;
			updateChanges(event, viewResponse.changes);
			updateTables(event, viewResponse.tables);

			if (viewResponse.aggregates != null) {
				if (viewResponse.aggregates instanceof Map) {
					viewResponse.aggregates = Object.fromEntries(viewResponse.aggregates);
				}

				for (let [formId, aggregateResults] of Object.entries(viewResponse.aggregates)) {
					if (aggregateResults instanceof Map) {
						aggregateResults = Object.fromEntries(aggregateResults);
					}
			
					const id = `chart-aggregate--${formId}`;
					const chart = document.getElementById(id);
			
					if (chart == null) {
						console.error(`Missing chart ${id}`);
						continue;
					}

					const ctx = chart.getContext('2d');
					const xData = Array.from(Object.keys(aggregateResults));
					const yData = Array.from(Object.values(aggregateResults));
					
					if (aggregateChartOptions[formId] == null) {
						aggregateChartOptions[formId] = {type: 'bar', data: {labels: [], datasets: [{label: "", data: []}]}};
						aggregateChart[formId] = new Chart(ctx, aggregateChartOptions[formId]);
					}
					
					aggregateChartOptions[formId].data.labels = xData;
					aggregateChartOptions[formId].data.datasets[0].data = yData;
					aggregateChart[formId].update();
				}
			}

			if (viewResponse.forms != null) {
				if (viewResponse.forms instanceof Map) {
					viewResponse.forms = Object.fromEntries(viewResponse.forms);
				}

				for (let [formId, form] of Object.entries(viewResponse.forms)) {
					if (form.visible == false) {
						data_view_hide(formId);
					}

					if (form.visible == true) {
						data_view_show(formId);
					}
				}
			}
		}).catch(err => {
			console.error(err);
			document.querySelector('#http-working').hidden = true;
			document.querySelector('#http-error').innerHTML = err;
			document.querySelector('#http-error').hidden = false;
		});
	}
	
}

var login = event => {
	document.querySelector('#http-error').hidden = true;
	document.querySelector('#http-working').innerHTML = "Aguardando resposta do servidor...";
	document.querySelector('#http-working').hidden = false;
    let element = event.target; let rowIndex = 0;
	const form = element.form;

	if (form.reportValidity()) {
		//event.stopPropagation();
		event.preventDefault();
		dataViewManager.login({path: "/login", user: form.user.value, password: form.password.value}).
		then(loginResponse => {
			const addToParent = (menu, list) => {
				if (menu instanceof Map) {
					menu = Object.fromEntries(menu);
				}

				for (let [name, field] of Object.entries(menu)) {
					if (typeof field === 'object') {
						list.push(
						`<li class='nav-item dropdown'><a class='nav-link dropdown-toggle' href='#' role='button' data-bs-toggle="dropdown" aria-expanded='false' id="menu-${name}">${name}</a><ul class='dropdown-menu'>`);
						addToParent(field, list);
						list.push(`</ul>\n</li>`);
					} else {
						list.push(`<li><a class='dropdown-item' href='#!/app/${field}'>${name}</a></li>\n`);
					}
				}
			};

			if (loginResponse instanceof Map) {
				loginResponse = Object.fromEntries(loginResponse);
			}

			let list = [];
			list.push(`<ul class='nav nav-pills'>`);
			addToParent(loginResponse.menu, list);
			list.push(`</ul>`);
			const str = list.join("\n");
			let div = document.createElement("div");
			div.innerHTML = str;
			div.addEventListener('click', appOnClick);
			document.querySelector('#menu').appendChild(div);
			form.hidden = true;
			document.querySelector('#http-working').hidden = true;

			for (let element of document.querySelectorAll(`a[href='#!/app/${loginResponse.path}']`)) {
				element.click();
			}
		}).catch(err => {
			console.error(err);
			document.querySelector('#http-working').hidden = true;
			document.querySelector('#http-error').innerHTML = err;
			document.querySelector('#http-error').hidden = false;
		});
	}
}

class DataViewManagerWs {

	constructor(path) {
		this.path = path;
    }

	login(obj_in) {
		let url = `${this.path}/wasm_ws/login`;
		return fetch(url, {method: "POST", body: JSON.stringify(obj_in)}).
		then(response => {
			if (response.status != 200) {
				return response.text().
				then(text => {
					throw text
				});
			}
			
			return response.json();
		}).
		then(loginResponse => {
			this.token = loginResponse.jwt_header;
			return loginResponse;
		});
    }

    process(obj_in) {
		let options = {method: "POST", body: JSON.stringify(obj_in), headers: {}};

		if (this.token != undefined) {
			options.headers["Authorization"] = "Bearer " + this.token;
		}

		let url = `${this.path}/wasm_ws/process`;
		return fetch(url, options).
		then(response => {
			if (response.status != 200) {
				return response.text().
				then(text => {
					throw text
				});
			}
			
			return response.json();
		});
    }

}

async function run() {
	const path = window.location.origin;
	const urlParams = new URLSearchParams(document.location.search);

	if (urlParams.get("mode") == "ws") {
		dataViewManager = new DataViewManagerWs(path);
	} else {
		// import init, { DataViewManager } from '../rufs_nfe_rust.js';
		const rufs_nfe_rust = await import("../rufs_nfe_rust.js");
		await rufs_nfe_rust.default();
		dataViewManager = new rufs_nfe_rust.DataViewManager(path);
	}
	
	document.querySelector('#login-send').addEventListener('click', login);
	document.querySelector('#main').addEventListener('click', appOnClick);
	document.querySelector('#main').addEventListener('change', appOnChange);
}

run();
