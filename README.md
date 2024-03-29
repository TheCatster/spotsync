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

## **WARNING**
The quality of music obtained from `ytmdl` is... iffy. Sometimes the
music is phenomenal and truly sounds like a FLAC, and other times it
isn't even the song or some 60 minute loop version.

Please understand that this is not the fault of `spotsync`! As better
methods are found (Patches welcome!), the program will be updated to
use them. As of now, get your special FLACs from other specialty
sources. `spotsync` helps you fill up your collection quickly and
autonomously, at the cost of some minor accuracy and quality.

## Installation
SpotSync can either be built on your own, or used via `docker` (docker
is the only officially supported method).

SpotSync requires `ytmdl` installed on your system, and accessible to
the program.

~~A PR is open to allow for custom outputs, but as of now it is
necessary to use my fork of the program at [my GitHub (*at
add-output-argument*)](https://github.com/TheCatster/ytmdl).~~

No longer true! The PR was merged into `ytmdl` unstable, so download
the `unstable` branch of `ytmdl` in order to use `spotsync`.

This also means that the custom output feature will soon be in
`master` of `ytmdl`, and the README will be updated as soon as that is
true.

### Docker
A `Dockerfile` and `docker-compose.yml.example` are provided, which
will allow you to run `spotsync` in a docker container. Simply copy
the compose file without `.example` and customise to your needs.

## Usage
### Docker
- `docker-compose build` to build the image.
- `docker-compose run spotsync sh` in order to generate `.refresh_token`.
- And then just `docker-compose up -d`!

## Configuration
The following variables can be customised.

| ENVVAR | DEFAULT VALUE | COMMENT |
|--------|---------------|---------|
| `CLIENT_ID` | 1234567890 | Get this from Spotify Dashboard |
| `CLIENT_SECRET` | 1234567890 | Get this from Spotify Dashboard |
| `SONG_DIR` | /music | No trailing slash! |
| `SONG_FORMAT` | mp3 | Options: mp3, m4a, opus |
| `CHECK_EVERY_DAYS` | 1 | Frequency to check playlists. Maximum of 7 days. |
