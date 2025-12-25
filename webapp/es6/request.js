import {appOnClick} from "./app.js";

let menuParams = null;
let loginResponse = null;
let dataViewManager = null;
let lastScannedCode = null;

async function init(debug) {
	const html = `
    `;
	return html;
}

function onRufsEvent(params) {
    const event = JSON.parse(params);
    console.log(event);
    const href = `#!/app/request/search`;
    appOnClick({target: {href, id: ""}, type: "click"});
}

async function process(params) {
    if (params != null && params.loginResponse != null) {
        loginResponse = params.loginResponse;
        dataViewManager = params.dataViewManager;
        params.addEventListener(onRufsEvent);
    }

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

    delete params.addEventListener;
    params.form_id = params.form_id.replaceAll(".js", "");
    return dataViewManager.process(params);
}

export {process};
