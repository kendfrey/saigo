"use strict";

const canvas = document.querySelector("canvas");
const ctx = canvas.getContext("2d");

const ws = new WebSocket(`ws://${location.host}/ws/display`);
ws.addEventListener("message", onMessage);

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