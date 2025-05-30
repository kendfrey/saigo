# Contributing Training Data

This project includes tools for collecting training data for the image recognition model. This training data can be submitted to the author for use in training the built-in model. It's also possible to train a custom model specifically for your board and stones.

## Gathering Training Data

1. Ensure Saigo is running and configured correctly.
2. Run `gather-td`, passing it a folder path to save the training data into. For example, `gather-td training-data\my-board`.
3. Place stones onto the board one after the other.
	- It's recommended (but not required) to place stones alternating black/white/black/white until the board is full, without moving or removing any stones. The rules of Go do not need to be followed.
4. Once you are finished, close `gather-td`. Your training data folder should now contain a series of images of the board.

## Labeling Training Data

1. Run `label-td`, passing it the folder path containing the training data. For example, `label-td training-data\my-board`.
2. Open the `label-td` UI at http://localhost:5416/.
3. Step through the images one at a time, labeling the stones and obscured points.
	- Click on an intersection to add or remove a stone (indicated by a black or white dot). This will automatically alternate between black and white stones.
	- Right-click and drag to mark or unmark points as obscured (indicated by a red X). This will typically be from a player's hand while placing a stone, but could be anything that makes the board unreadable at that point, such as a player's head or other foreign object. A point should be marked as obscured when the intersection point is hidden, or at least 50% of the stone-sized area is hidden.
4. Once you are finished, close `label-td`. Your training data folder should now have a `.txt` label file for every captured image (not including `reference.png`).