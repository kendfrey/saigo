"use strict";

const canvas = document.querySelector("canvas");
const ctx = canvas.getContext("2d");

const ws = new WebSocket(`ws://${location.host}/ws/display`);
ws.addEventListener("message", onMessage);

wakeLock();

async function onMessage(event)
{
	const buffer = await event.data.arrayBuffer();
	const imageData = toImageData(buffer);
	canvas.width = imageData.width;
	canvas.height = imageData.height;
	ctx.putImageData(imageData, 0, 0);
}

async function wakeLock()
{
	try
	{
		await navigator.wakeLock.request();
	}
	catch (_)
	{
		
	}
}