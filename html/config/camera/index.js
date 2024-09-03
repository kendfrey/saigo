"use strict";

const device = document.getElementById("device");
const handler = throttle(onInput);
device.addEventListener("input", handler);

const canvas = document.getElementById("preview");
const ctx = canvas.getContext("2d");

const ws = new WebSocket(`ws://${location.host}/ws/camera`);
ws.addEventListener("message", onMessage);

load();

async function load()
{
	await loadCameras();
	await loadConfig();
}

async function loadCameras()
{
	const request = new Request("/api/cameras");
	const response = await fetch(request);
	const cameras = await response.json();
	for (const item of cameras)
	{
		const option = document.createElement("option");
		option.textContent = item;
		device.appendChild(option);
	}
}

async function loadConfig()
{
	const request = new Request("/api/config/camera");
	const response = await fetch(request);
	const config = await response.json();
	device.value = config.device;
}

async function onInput()
{
	const config =
	{
		device: device.value,
	};
	const request = new Request("/api/config/camera",
	{
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(config),
	});
	await fetch(request);
}

async function onMessage(event)
{
	const buffer = await event.data.arrayBuffer();
	const view = new DataView(buffer);
	const width = view.getUint32(0); // The first 4 bytes are the width
	const height = view.getUint32(4); // The next 4 bytes are the height
	const imageData = new ImageData(new Uint8ClampedArray(buffer, 8), width, height); // The rest is the image data
	canvas.width = width;
	canvas.height = height;
	ctx.putImageData(imageData, 0, 0);
}