"use strict";

const canvas = document.querySelector("canvas");
const ctx = canvas.getContext("2d");

const ws = new WebSocket(`ws://${location.host}/ws/display`);
ws.addEventListener("message", onMessage);

async function onMessage(event)
{
	const buffer = await event.data.arrayBuffer();
	const imageData = toImageData(buffer);
	canvas.width = imageData.width;
	canvas.height = imageData.height;
	ctx.putImageData(imageData, 0, 0);
	wakeLock();
}

let sentinel = null;
async function wakeLock()
{
	if (sentinel !== null)
		return;

	try
	{
		sentinel = await navigator.wakeLock.request();
		sentinel.addEventListener("release", () =>
		{
			sentinel = null;
		});
	}
	catch (e)
	{
		console.warn("Failed to acquire wake lock:", e);
	}
}