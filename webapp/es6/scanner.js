const regExpNfeChaveAcesso = /(?<chaveNfe>\b\d{22}\s?\d{22,27}\b)/;
let stoped = true;
let menuParams = null;
let loginResponse = null;
let dataViewManager = null;
let lastScannedCode = null;

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function log(text, data) {
	console.log(text, data);
	const element = document.getElementById("scanner-log");

	if (typeof text != "string") {
		text = JSON.stringify(text);
	}

	if (typeof data == "string") {
		text = text + " " + data;
	} else if (data != null) {
		text = text + " " + JSON.stringify(data);
	}

	element.value = text + "\n" + element.value;
}

function processCode(value) {
	lastScannedCode = value;
	log("processCode:", value);

	{
		const element = document.getElementById("nfe_import-qr_code");

		if (element != null) {
			element.value = value;
		}
	}
}

function decodeImgBase64(imgBase64) {
	if (menuParams.includes("QR_CODE")) {
		window.qrcode.decode(imgBase64);
	} else {
		window.Quagga.decodeSingle({
			decoder: {
				readers: ["code_128_reader", "ean_reader"]
			},
			src: imgBase64
		}, result => {
			if (result && result.codeResult) {
				processCode(result.codeResult.code);
			}
		});
	}
}

function stop() {
	const video = document.getElementById("scanner-video");

	if (video == null) {
		return;
	}

	const stream = video.srcObject;

	if (stream == null) {
		return;
	}

	stream.getTracks().forEach(function(track) {
		track.stop();
	});

	stoped = true;
	const input_file = document.getElementById("scanner-input_file");
	input_file.hidden = false;
	video.hidden = true;
}

function play(stream) {
	window.qrcode.callback = async (response) => {
		if (response.startsWith("error") == false) {
			processCode(response);
		}
	}

	const input_file = document.getElementById("scanner-input_file");
	input_file.hidden = true;
	const video = document.getElementById("scanner-video");
	video.hidden = false;
	video.srcObject = stream;
	const canvas = document.getElementById("scanner-canvas");
	const canvasContext = canvas.getContext('2d');

	let processImage = () => {
		canvasContext.drawImage(video, 0, 0);

		try {
			const imgBase64 = canvas.toDataURL();
			decodeImgBase64(imgBase64);
		} catch (e) {
			log(e);
		}

		if (stoped == false) {
			setTimeout(processImage, 500);
		}
	}

	video.play().then( () => {
		stoped = false;
		const w = video.videoWidth;
		const h = video.videoHeight;
		canvas.width = w;
		canvas.height = h;
		//canvas.style.width = w + "px";
		//canvas.style.height = h + "px";
		canvasContext.clearRect(0, 0, w, h);
		log(`video.play : ${w} x ${h}`);
		processImage();
	});
}

async function init(debug) {
	const listDevices = [];
	listDevices.push(`<option value="file">Importar arquivo</option>`);
	let inputFileHidden = "hidden";

	if (debug == true) {
		inputFileHidden = "";
	}

	if (navigator.mediaDevices != undefined) {
		navigator.mediaDevices.getUserMedia({video: {width: 1280, height: 720}, audio: false})
		.then(stream => {
			//play(stream);

			if (navigator.mediaDevices.enumerateDevices != undefined) {
				navigator.mediaDevices.enumerateDevices().then(devices => {
					let deviceId = null;

					devices.forEach(device => {
						if (device.kind === "videoinput") {
							if (deviceId == null) {
								deviceId = device.deviceId;
							}

							listDevices.push(`<option value="${device.deviceId}">${device.label}</option>`);
							log(device);
							log(navigator.mediaDevices.getSupportedConstraints());
							log(device.getCapabilities());
						}
					});

					const element = document.getElementById("scanner-video_input");
					element.innerHTML = listDevices.join("");
				});
			} else {
				console.error("enumerateDevices() not supported.");
			}
		});
	} else {
		console.error("mediaDevices() not supported.");
	}

	const html = `
<h4>Scanner EAN/Qr-code</h4>

<form id="scanner" name="scanner" class="form-horizontal" role="form" data-rufs-module="scanner.js">
	<div class="form-group">
	    <label for="video_input" class="col-sm-2 control-label">Dispositivo de video</label>

	    <div class="col-sm-8">
			<select class="form-control" id="scanner-video_input" name="video_input" required></select>
	    </div>

		<div class="col-sm-10">
			<button class="btn btn-default" id="scanner-exit"><span class="glyphicon glyphicon-remove"></span> Sair</button>
		</div>
	</div>

    <div class="form-group">
		<label for="scanner-code" class="col-sm-2 control-label">EAN/Qr-code</label>
		<input id="scanner-code" class="col-sm-10" type="text"></input>
    </div>

    <div class="form-group" ${inputFileHidden}>
		<label for="scanner-input_file" class="col-sm-2 control-label">Qr-code (Foto/Imagem)</label>
	    <input id="scanner-input_file" class="col-sm-10" type="file" accept="image/*"></input>
    </div>

	<textarea style="overflow: auto;width: 100%" id="scanner-log" rows="5"></textarea>
</form>

<video id="scanner-video" width="320" height="240" style="height: 50%" hidden></video>
<canvas id="scanner-canvas" width="320" height="240" style="height: 50%" hidden></canvas>
`;
	return html;
}

function deviceChange(videoInput) {
	stop();

	if (videoInput == "file") {
		return;
	}

	let video_options = {
		deviceId: {exact: videoInput},
		//facingMode: 'environment',
		width: 1280,
		width: 720
	};

	navigator.mediaDevices.getUserMedia({video: video_options, audio: false}).then(stream => {
		play(stream);
	});
}

function onRufsEvent(params) {
	const event = JSON.parse(params);
	console.log(event);
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
			const element = document.getElementById("data_view_root-scanner");

			if (element != null) {
				element.hidden = false;
			} else {
				const html = await init(debug);
				const formId = "scanner";
				viewResponse.html[formId] = html;
			}
		} else if (form_id == "scanner-exit") {
			stop();
			const element = document.getElementById("data_view_root-scanner");
			element.hidden = true;
		}
	} else if (event == "change") {
		if (form_id == "scanner-input_file") {
			for (let file of data[form_id]) {
				let reader = new FileReader();

				reader.onload = eventReader => {
					const base64data = eventReader.target.result;
					decodeImgBase64(base64data);
				}

				reader.readAsDataURL(file);
			}
		} else if (form_id == "scanner-video_input") {
			deviceChange(data[form_id]);
		} else if (form_id == "scanner-code" && data[form_id] != "") {
			processCode(data[form_id]);
		}
	} else if (event == "keyup") {
		const oldValue = data[form_id];

		if (form_id == "scanner-code" && oldValue != null && oldValue.length >= 47 || (oldValue.length >= 44 && oldValue.startsWith("55") == false)) {
			processCode(oldValue);
		}
	}

	return viewResponse;
}

export {process};
