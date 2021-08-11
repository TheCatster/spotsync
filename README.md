# SpotSync
## Sync your Spotify playlists locally
Download songs from YouTube Music when added to a Spotify
playlist. Keep your music in sync between Spotify and your local
collection, and keep your playlist structure intact!

## Why?
I prefer using free as in freedom software when possible. The problem
is, Spotify's discover features are amazing; I have yet to find an
equivalent (with the closest being the closed source last.fm) I use
Spotify to find new songs and add them to my playlists, but often
forget to manually download the songs to update my Subsonic-API based
server.

This program serves as a middle man to automate this process. It
attempts to do one thing, and do it well, delegating the actual
downloading to `ytmdl`.

## Installation
Currently SpotSync is only available by building it yourself, but
don't you worry! Installations from the AUR, Docker Hub, prebuilt
binaries for Windows, Debian, and more will be available as soon as
the program enters alpha.

SpotSync requires `ytmdl` installed on your system, and accessible to
the program.

A PR is open to allow for custom outputs, but as of now it is necessary
to use my fork of the program at [my GitHub (*at add-output-argument*)](https://github.com/TheCatster/ytmdl).

## Usage
TODO

## Configuration
TODO
