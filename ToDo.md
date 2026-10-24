# Enhancements and Bug Fixes

Each of these should be a distinct commit and corresponding pull request.

1. Set the default grid size to 10 x 10
2. Move options to sidebar panel (GUI form with buttons, listbox, etc.)
   1. Level indicator
   2. Running tally of arrows, removed, failed, etc.
   3. Running score of wins/losses (see Wins and Losses)
3. Level completion animations:
   1. After completing a level, the rotate cube in a random orbit until user presses Enter to start next level.
4. Sound effects:
   1. Arrow removed
   2. Arrow blocked
   3. Level completed
   4. Level failed (see Wins and Losses)
5. Add Themes:
   1. Background image and color schemes: countries/cultures, nature (mountains, ocean, arctic, )
6. Wins and Losses based on number of clicks vs arrows to remove
   1. Example: if starting at 36 arrows and clicks > 36 = lose
   2. If lose: Prompt to start level again
   3. Keep score of wins and losses