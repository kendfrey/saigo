# Getting Started

It's recommended to make sure Saigo is installed and running before you buy any hardware. Follow the instructions on this page to get it set up.

## Installation

1. Download the [latest release](https://github.com/kendfrey/saigo/releases) for your OS and extract the contents to a folder of your choice.
2. Install the LibTorch dependency:
	- Download version 2.2.0 of LibTorch, matching your OS.
		- For Windows, download https://download.pytorch.org/libtorch/cu118/libtorch-win-shared-with-deps-2.2.0%2Bcu118.zip
		- For Linux, download https://download.pytorch.org/libtorch/cu118/libtorch-cxx11-abi-shared-with-deps-2.2.0%2Bcu118.zip
		- You can find other download options at https://pytorch.org/get-started/locally/. You'll need to replace the version number in the generated URL with 2.2.0, in order to download the correct version.
	- Extract the LibTorch files to a folder of your choice.
	- Create a new environment variable called `LIBTORCH` and set it to the path of that folder.
	- Add the `lib` subfolder to your `PATH` environment variable.
		- For example on Windows, add `%LIBTORCH%\lib` to your `PATH`.
	- LibTorch is now installed. For more technical details, refer to [`tch`](https://github.com/LaurentMazare/tch-rs?tab=readme-ov-file#libtorch-manual-install), which Saigo uses as a dependency. (Note that Saigo depends on a version of `tch` that depends on LibTorch 2.2.0, which is not necessarily the latest version.)
3. Run the program `saigo` from the files that you installed in step 1. If it is installed correctly, it should give the following message:
	```
	Listening on http://localhost:5410/
	Press Ctrl+C to exit.
	```

## Hardware Setup
Mount the camera and projector directly above the board, facing downward. Make sure that they are positioned high enough to cover the entire board. Orientation doesn't matter, since that will be configured later.

## Configuration

1. Once Saigo is running, browse to http://localhost:5410/.
2. Open the display UI in full-screen on the projector display. By default it should show a grid of dots, which you will align with the board later.
2. On the main configuration page, enter your board size.
3. From the main configuration page, click the "Configure display" link and adjust the settings.
	- The display resolution determines the output resolution of the display UI. The higher the resolution, the more CPU power it will use.
	- The sliders adjust the position and orientation of the display. Adjust the sliders until the grid of dots is correctly aligned with the intersections of the board.
4. From the main configuration page, click the "Configure camera" link and adjust the settings.
	- Like the display, higher camera resolutions will use more CPU power.
	- Click and drag the corners of the white box to the corner intersections of the grid.
	- Turn off the projector and make sure the board is empty, then click "Take Reference Image" to teach Saigo what your board looks like. Use the overlay grid to check alignment with the intersections of the board. Drag the corners of the white box and click "Take Reference Image" again to adjust it.
6. Once these steps are complete, you're ready to start a game.

## Playing a Game

To play an offline game against an engine, use `saigo-gtp` as if it were a normal GTP engine. It will connect to your running instance of Saigo, and when you make a move on the board, it will send that move to the GTP controller.

1. Set up `saigo-gtp` as an engine in your GTP controller of choice. For example, in Sabaki, just add a new engine using the path to `saigo-gtp`. No arguments are required.
2. Start a new engine-vs-engine game, with `saigo-gtp` playing as your colour and your AI opponent as the other.
3. When it is your turn to play, a stripe will appear on your side of the board. If the stripe is white, simply place your next stone on the board. If the stripe is yellow, make your opponent's move on the intersection indicated by a blinking light, after which the stripe will turn white.
4. While your opponent is thinking, the white stripe will appear on the far side of the board.
5. To pass, place two of your stones on the board simultaneously. They must be located on the intersections so that they are recognized. The stones must of course be removed again if the opponent continues playing.
6. To resign, place two of your opponent's stones on the board.
7. Note: GTP engines are not notified when the game is over. If your opponent resigns or passes second, you'll need to check your GTP controller to see the result.