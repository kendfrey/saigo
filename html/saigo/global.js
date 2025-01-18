"use strict";

const STONE_SIZE = 16;

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

// Deserializes an ArrayBuffer into an ImageData
function toImageData(buffer)
{
	const view = new DataView(buffer);
	const width = view.getUint32(0); // The first 4 bytes are the width
	const height = view.getUint32(4); // The next 4 bytes are the height
	return new ImageData(new Uint8ClampedArray(buffer, 8), width, height); // The rest is the image data
}
