"use strict";

const angle = document.getElementById("angle");
const x = document.getElementById("x");
const y = document.getElementById("y");
const width = document.getElementById("width");
const height = document.getElementById("height");
const perspective_x = document.getElementById("perspective_x");
const perspective_y = document.getElementById("perspective_y");
const handler = throttle(onInput);
angle.addEventListener("input", handler);
x.addEventListener("input", handler);
y.addEventListener("input", handler);
width.addEventListener("input", handler);
height.addEventListener("input", handler);
perspective_x.addEventListener("input", handler);
perspective_y.addEventListener("input", handler);

async function onInput()
{
	const config =
	{
		board_width: 19,
		board_height: 19,
		angle: Number(angle.value),
		x: Number(x.value),
		y: Number(y.value),
		width: Number(width.value),
		height: Number(height.value),
		perspective_x: Number(perspective_x.value),
		perspective_y: Number(perspective_y.value),
	};
	const request = new Request("/api/calibrate/display",
	{
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(config),
	});
	await fetch(request);
}

// Prevents a function from being called before the previous call has finished.
function throttle(callback)
{
	let promise = null;
	let waiting = false;
	const fun = async () =>
	{
		if (promise)
		{
			// If the function is called while the previous call is still in progress, set a flag to call it again once the previous call is done
			waiting = true;
			return;
		}

		// Otherwise, call the function immediately and wait for it to finish
		waiting = false;
		promise = callback();
		await promise;
		promise = null;

		if (waiting)
			fun();
	};
	return fun;
}
