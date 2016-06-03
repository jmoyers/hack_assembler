#!/bin/sh
# Create a 3 paned tmux window with vim on left
tmux new-session -d -s dev
tmux new-window -t dev
tmux rename-window 'Dev'
tmux split-window -h
tmux split-window -v
tmux select-pane -t 0
tmux send-keys 'vim src/main.rs' 'C-m'
tmux select-pane -t 1
tmux send-keys 'cd tests && clear' 'C-m'
tmux select-pane -t 2
tmux send-keys 'cd tests && clear' 'C-m'
tmux attach-session -d -tdev
