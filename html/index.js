"use strict";

const canvas = document.querySelector("canvas");
const ctx = canvas.getContext("2d");

const ws = new WebSocket(`ws://${location.host}/ws/display`);
ws.addEventListener("message", onMessage);

async function onMessage(event)
{
	const blob = event.data;
	const data = new ImageData(new Uint8ClampedArray(await blob.arrayBuffer()), 1280, 720);
	ctx.putImageData(data, 0, 0);
}