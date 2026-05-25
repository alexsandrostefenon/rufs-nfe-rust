import { addEventListener, dataViewManager, loginResponse } from "./app.js";

let menuParams = null;
let lastScannedCode = null;

function onRufsEvent(params) {
    const event = JSON.parse(params);
    console.log(event);
    // {"service":"request","action":"notify","primaryKey":{"id":2}}
    if (event.service == "request") {
        const regExMenuSearch = /\.(new|view|edit|search)(\?)?/;
        const regExMenuReplace = "/$1$2";
        let schema = loginResponse.path./*replaceAll("/", ".").*/replace(regExMenuSearch, regExMenuReplace);

        for (let element of document.querySelectorAll(`a[href='#!/app/${schema}']`)) {
            element.click();
        }
    }
}

async function init(debug) {
	const html = `
    `;
	return html;
}

async function process(params) {
    const {form_id, event, data} = params;
    const urlParams = new URLSearchParams(document.location.search);
    const debug = urlParams.get("debug") == "true";

    const viewResponse = {
        html: {},
        changes: {},
        tables: {},
        aggregates: {},
        forms: {}
    };

    if (event == "click") {
        if (form_id.includes("#!/app/")) {
            menuParams = form_id;
            const element = document.getElementById("data_view_root-request");

            if (element != null) {
                element.hidden = false;
            } else {
                const html = await init(debug);
                const formId = "request";
                viewResponse.html[formId] = html;
                //return viewResponse;
            }
        } else if (form_id == "request-exit") {
            const element = document.getElementById("data_view_root-request");
            element.hidden = true;
        }
    } else if (event == "change") {
    } else if (event == "keyup") {
    }

    params.form_id = params.form_id.replaceAll(".js", "");
    return dataViewManager.process(params);
}

addEventListener(onRufsEvent);

export {process};
