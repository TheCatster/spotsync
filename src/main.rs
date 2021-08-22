#![feature(string_remove_matches, slice_partition_dedup)]
use aspotify::{authorization_url, Client, ClientCredentials, PlaylistSimplified, Scope};

use fern::{log_file, Dispatch};
use log::{info, warn, LevelFilter};

use anyhow::{Context, Result};

use chrono::{Duration, Utc};
use dirs::home_dir;
use ron::{
    de::from_reader,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    env,
    fs::{self, write, File},
    hash::Hash,
    io::{self, Write},
    path::PathBuf,
    process::Command,
    time,
};
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct PlaylistConfig {
    title: String,
    songs: Vec<Song>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq)]
struct Song {
    title: String,
    artists: Vec<String>,
    album: String,
    id: String,
}

impl Ord for Song {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Song {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Error)
        .chain(io::stdout())
        .chain(log_file("spotsync.log").expect("No permission to write to the current directory."))
        .apply()
        .expect("Failed to dispatch Fern logger!");

    let client = authenticate_spotify().await;
    info!("Connecting to Spotify API...");

    let dir =
        env::var("SONG_DIR").context("SONG_DIR variable not set! Make sure your .env has it.")?;
    let song_dir = if dir.starts_with('~') {
        let mut song_dir = home_dir().unwrap();
        song_dir.push(dir.strip_prefix("~/").unwrap());
        song_dir
            .into_os_string()
            .into_string()
            .expect("Song directory invalid!")
    } else {
        dir.to_owned()
    };

    let song_format = env::var("SONG_FORMAT")
        .context("SONG_FORMAT variable not set! Make sure your .env has it.")?;

    let check_every_days = env::var("CHECK_EVERY_DAYS")
        .context("CHECK_EVERY_DAYS variable not set! Make sure your .env has it.")?
        .parse::<i64>()?;

    // Fetch remote playlists
    let spotify_playlists = retrieve_spotify_users_playlists(&client, 10).await;

    // Main logic
    loop {
        let mut time_now = Utc::now();
        let target_time = time_now
            .checked_add_signed(Duration::days(check_every_days))
            .expect("Unable to add to duration!");

        for playlist in &spotify_playlists {
            local_playlist_make_if_not_exists(playlist, &song_dir);
            let local_songs =
                local_playlist_read_songs(&client, playlist, &song_dir, &song_format).await?;
            let spotify_songs = spotify_playlist_read_songs(&client, playlist).await;
            let mut needed_songs: Vec<Song> = compare_playlists(&local_songs, &spotify_songs).await;
            if !needed_songs.is_empty() {
                download_songs(&playlist.name, &song_dir, &song_format, &needed_songs).await?;
                info!("Updating RON for playlist: {}", &playlist.name);
                update_ron(&playlist.name, &mut needed_songs).await?;
            }
        }

        while time_now < target_time {
            sleep(time::Duration::from_secs(900)).await;
            time_now = Utc::now();
        }
    }
}

async fn update_ron(playlist_title: &str, songs: &mut [Song]) -> Result<()> {
    let playlist_file = format!("data/playlists/\"{}\".ron", playlist_title);
    let f = File::open(&playlist_file).expect("Failed opening playlist RON!");

    let playlist: PlaylistConfig = from_reader(f)?;

    let mut updated_songs = playlist.songs.to_vec();
    updated_songs.append(&mut songs.to_vec());

    let updated_playlist: PlaylistConfig = PlaylistConfig {
        title: playlist.title,
        songs: updated_songs,
    };

    let pretty = PrettyConfig::new()
        .with_depth_limit(4)
        .with_separate_tuple_members(true)
        .with_enumerate_arrays(true);
    let playlist_ron = to_string_pretty(&updated_playlist, pretty)?;

    let playlist_file = format!("data/playlists/\"{}\".ron", playlist_title);
    write(playlist_file, playlist_ron)?;

    Ok(())
}

async fn download_songs(
    playlist_title: &str,
    song_dir: &str,
    song_format: &str,
    songs: &[Song],
) -> Result<()> {
    for song in songs {
        let song_id = song.id.to_owned();

        let ytmdl = Command::new("ytmdl")
            .arg("-q")
            .arg(format!("--output={}/{}", song_dir, playlist_title))
            .arg(format!("--spotify-id={}", song_id))
            .arg(format!("--format={}", song_format))
            .arg(format!("{} {}", &song.title, &song.artists.join(" ")))
            .spawn()?;

        let output = ytmdl.wait_with_output()?;

        if !output.status.success() {
            warn!("ytmdl failed to download the song! Ensure it is installed!");
        }

        info!("Successfully downloaded song: {}", song.title);
    }

    Ok(())
}

async fn compare_playlists(local: &[Song], remote: &[Song]) -> Vec<Song> {
    if local == remote {
        vec![]
    } else {
        let mut unique_songs: Vec<Song> = vec![];
        for song in remote {
            if !local.contains(song) {
                unique_songs.push(song.to_owned());
            }
        }

        unique_songs
    }
}

async fn authenticate_spotify() -> Client {
    let dotenv_file = dotenv::dotenv();

    if dotenv_file.is_err() {
        warn!("Could not read env file! Assuming in docker.");
    }

    // Create data directory if not available
    let path = PathBuf::from("data/playlists");
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let path = PathBuf::from("data/playlists");
        std::fs::create_dir_all(path).unwrap();
    };

    match std::fs::read_to_string("data/.refresh_token") {
        Ok(_) => {
            info!(".refresh_token present, refreshing client.");
            Client::with_refresh(
                ClientCredentials::from_env().expect("Cannot read env vars for SpotSync!"),
                std::fs::read_to_string("data/.refresh_token").expect("Cannot read refresh token file!"),
            )
        }
        Err(_) => {
            info!(".refresh_token not present, creating one with proper scope.");
            let client = Client::new(
                ClientCredentials::from_env().expect("Cannot read env vars for SpotSync!"),
            );

            let (url, state) = authorization_url(
                &client.credentials.id,
                [
                    Scope::UgcImageUpload,
                    Scope::UserReadPlaybackState,
                    Scope::UserModifyPlaybackState,
                    Scope::UserReadCurrentlyPlaying,
                    Scope::Streaming,
                    Scope::AppRemoteControl,
                    Scope::UserReadEmail,
                    Scope::UserReadPrivate,
                    Scope::PlaylistReadCollaborative,
                    Scope::PlaylistModifyPublic,
                    Scope::PlaylistReadPrivate,
                    Scope::PlaylistModifyPrivate,
                    Scope::UserLibraryModify,
                    Scope::UserLibraryRead,
                    Scope::UserTopRead,
                    Scope::UserReadRecentlyPlayed,
                    Scope::UserFollowRead,
                    Scope::UserFollowModify,
                ]
                .iter()
                .copied(),
                false,
                "http://localhost:8888/callback",
            );

            println!("Go to this website: {}", url);

            print!("Enter the URL that you were redirected to: ");
            io::stdout().flush().unwrap();
            let mut redirect = String::new();
            io::stdin().read_line(&mut redirect).unwrap();

            client.redirected(&redirect, &state).await.unwrap();

            write(
                "data/.refresh_token",
                client
                    .refresh_token()
                    .await
                    .expect("Could not obtain refresh token from Spotify!"),
            )
            .expect("Unable to write to refresh token file, possibly no permission?");

            client
        }
    }
}

async fn spotify_playlist_read_songs(client: &Client, playlist: &PlaylistSimplified) -> Vec<Song> {
    // Why is this so ugly!? There has to be a better way.
    client
        .playlists()
        .get_playlist(&playlist.id, None)
        .await
        .expect("Unable to retrieve playlist!")
        .data
        .tracks
        .items
        .iter()
        .map(|playlist_item| {
            let song_title = match playlist_item.item.as_ref() {
                Some(aspotify::PlaylistItemType::Track(track)) => &track.name,
                _ => "Track not found",
            };

            let song_artists = match playlist_item.item.as_ref() {
                Some(aspotify::PlaylistItemType::Track(track)) => track
                    .artists
                    .iter()
                    .map(|x| x.name.to_string())
                    .collect::<Vec<String>>(),
                _ => vec!["Artists not found".to_string()],
            };

            let song_album = match playlist_item.item.as_ref() {
                Some(aspotify::PlaylistItemType::Track(track)) => &track.album.name,
                _ => "Track not found",
            };

            let song_id = match playlist_item.item.as_ref() {
                Some(aspotify::PlaylistItemType::Track(track)) => track
                    .id
                    .as_ref()
                    .expect("Local songs do not have a track ID!")
                    .to_owned(),
                _ => String::from("Track not found"),
            };

            Song {
                title: song_title.to_string(),
                artists: song_artists,
                album: song_album.to_string(),
                id: song_id,
            }
        })
        .filter(|song| {
            (song.title != *String::from("Track not found"))
                && (song.artists != vec!["Artists not found".to_string()])
                && (song.album != *String::from("Track not found"))
                && (song.id != *String::from("Track not found"))
        })
        .collect::<Vec<Song>>()
}

async fn retrieve_spotify_users_playlists(
    client: &Client,
    limit: usize,
) -> Vec<PlaylistSimplified> {
    client
        .playlists()
        .current_users_playlists(limit, 0)
        .await
        .expect("Could not retrieve the current user's playlists!")
        .data
        .items
}

fn local_playlist_make_if_not_exists(playlist: &PlaylistSimplified, song_dir: &str) {
    // Create directory if it doesn't exist
    let path = PathBuf::from(format!("{}/{}", song_dir, &playlist.name));
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let path = PathBuf::from(format!("{}/{}", song_dir, &playlist.name));
        std::fs::create_dir_all(path).unwrap();
    };

    // Create RON if it doesn't exist
    let path = PathBuf::from(format!("data/playlists/\"{}\".ron", &playlist.name));
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let path = PathBuf::from(format!("data/playlists/\"{}\".ron", &playlist.name));
        File::create(path).unwrap();
    };
}

async fn local_playlist_read_songs(
    client: &Client,
    playlist: &PlaylistSimplified,
    song_dir: &str,
    song_format: &str,
) -> Result<Vec<Song>> {
    let input_path = format!("data/playlists/\"{}\".ron", &playlist.name);
    let f = File::open(&input_path).expect("Failed opening file");
    let playlist_ron: Result<PlaylistConfig, ron::Error> = from_reader(f);
    let playlist_struct: Vec<Song> = match playlist_ron {
        Ok(x) => x.songs,
        Err(_) => {
            create_playlist_ron(client, playlist, song_dir, song_format)
                .await?
                .songs
        }
    };

    Ok(playlist_struct)
}

async fn create_playlist_ron(
    client: &Client,
    playlist: &PlaylistSimplified,
    song_dir: &str,
    song_format: &str,
) -> Result<PlaylistConfig> {
    let songs: Vec<Song> = spotify_playlist_read_songs(client, playlist).await;
    download_songs(&playlist.name, song_dir, song_format, &songs).await?;

    let playlist_struct = PlaylistConfig {
        title: playlist.name.to_string(),
        songs,
    };

    let pretty = PrettyConfig::new()
        .with_depth_limit(4)
        .with_separate_tuple_members(true)
        .with_enumerate_arrays(true);
    let playlist_ron = to_string_pretty(&playlist_struct, pretty)?;

    let playlist_file = format!("data/playlists/\"{}\".ron", &playlist.name);
    write(playlist_file, playlist_ron)?;

    Ok(playlist_struct)
}
