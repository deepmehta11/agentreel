Output demo.gif

Set FontSize 14
Set Width 1100
Set Height 650
Set Theme "Catppuccin Mocha"
Set Padding 15
Set TypingSpeed 40ms

# View a trajectory
Type "agentreel view ~/.agentreel/trajectories/deep.trajectory.json --full"
Enter
Sleep 5s

# Stats
Type "agentreel stats ~/.agentreel/trajectories/deep.trajectory.json"
Enter
Sleep 3s

# List local trajectories
Type "agentreel list"
Enter
Sleep 3s

# Compare
Type "agentreel compare ~/.agentreel/trajectories/deep.trajectory.json ~/.agentreel/trajectories/thinking.trajectory.json"
Enter
Sleep 4s

# Redact
Type "agentreel redact ~/.agentreel/trajectories/deep.trajectory.json -o /tmp/safe.json && echo 'Secrets redacted!'"
Enter
Sleep 2s

Sleep 2s
