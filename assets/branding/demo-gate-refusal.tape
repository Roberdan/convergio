# VHS tape — Convergio hero demo: a lying agent is refused, the refusal is provable.
# Regenerate with:  vhs assets/branding/demo-gate-refusal.tape
# Requires: the daemon running (`convergio start`) and `cvg` on PATH.

Output assets/branding/demo-gate-refusal.gif

Set Shell "bash"
Set FontSize 20
Set Width 1200
Set Height 760
Set Padding 28
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
Type "export PATH=$HOME/.cargo/bin:$PATH"
Enter
Type "clear"
Enter
Show

Sleep 600ms
Type "cvg demo"
Sleep 800ms
Enter
Sleep 4s
Sleep 3s
