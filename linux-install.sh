#!/bin/bash
set -e
echo "breevy-rs Linux installer"
echo "1) granting input access (needed for /dev/input and /dev/uinput)"
sudo usermod -aG input "$USER"
sudo install -m 755 "$(dirname "$0")/breevy-rs-linux64" /usr/local/bin/breevy-rs
install -d "$HOME/.config/breevy"
echo
echo "Now copy your Breevy data file to:"
echo "  $HOME/.config/breevy/abbreviations.bvy"
echo
echo "Log out and back in (group change), then run:"
echo "  breevy-rs --hook ~/.config/breevy/abbreviations.bvy"
echo
echo "Type an abbreviation, then press Ctrl to expand (trigger mode),"
echo "or a delimiter such as space/period for word-ending mode."
