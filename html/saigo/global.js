"use strict";

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
