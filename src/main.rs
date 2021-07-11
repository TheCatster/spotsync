#![feature(string_remove_matches)]
use aspotify::{
    authorization_url, Client, ClientCredentials, PlaylistSimplified, Scope, TwoWayCursorPage,
};
use chrono::Local;
use fern::{log_file, Dispatch};
use log::{debug, error, info, log_enabled, Level};
use std::{
    env, fs,
    io::{self, Write},
};

mod tests;

#[tokio::main]
async fn main() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Error)
        .chain(std::io::stdout())
        .chain(
            fern::log_file("spotsync.log")
                .expect("No permission to write to the current directory."),
        )
        .apply()
        .expect("Failed to dispatch Fern logger!");

    let client = authenticate_spotify().await;
    info!("Connecting to Spotify API...");

    // Fetch remote playlists
    let spotify_playlists = retrieve_spotify_users_playlists(&client, 10).await;

    // Creating the local playlist if necessary
    for playlist in &spotify_playlists {
        local_playlist_make_dir_if_not_exists(&playlist.name);
    }

    // Fetching list of songs locally
    let local_songs: Vec<Vec<String>> = spotify_playlists
        .iter()
        .map(|x| local_playlist_read_songs(&x.name))
        .collect();
    println!("{:?}", local_songs);

    // Why doesn't this work? What's the difference between this and the for loop?
    // let spotify_songs = spotify_playlists
    //     .iter()
    //     .map(|x| async move {
    //         spotify_playlist_read_songs(&client, &x).await
    //     });

    // Fetching list of songs on Spotify
    let mut spotify_songs: Vec<Vec<String>> = Vec::new();

    for spotify_playlist in spotify_playlists {
        spotify_songs.push(spotify_playlist_read_songs(&client, &spotify_playlist).await)
    }

    // Compare PlaylistSimplified

    // Download missing songs
}

async fn authenticate_spotify() -> Client {
    dotenv::dotenv().expect("Could not read .env file!");

    match std::fs::read_to_string(".refresh_token") {
        Ok(_) => {
            info!(".refresh_token present, refreshing client.");
            Client::with_refresh(
                ClientCredentials::from_env().expect("Cannot read env vars for SpotSync!"),
                std::fs::read_to_string(".refresh_token").expect("Cannot read refresh token file!"),
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

            fs::write(
                ".refresh_token",
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

async fn spotify_playlist_read_songs(
    client: &Client,
    playlist: &PlaylistSimplified,
) -> Vec<String> {
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
            let song = match playlist_item.item.as_ref().expect("No such playlist item!") {
                aspotify::PlaylistItemType::Track(track) => &track.name,
                _ => "Track not found",
            };

            song.to_string()
        })
        .collect::<Vec<String>>()
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

fn local_playlist_make_dir_if_not_exists(dir_name: &str) {
    let mut path = env::current_dir().expect("Could not read current directory!");
    path.push(format!("./music/{}", dir_name));
    let metadata = fs::metadata(path);
    if metadata.is_err() {
        let mut path = env::current_dir().expect("Could not read current directory!");
        let playlist_dir = format!("./music/{}", dir_name);
        path.push(playlist_dir);
        std::fs::create_dir_all(path).unwrap();
    };
}

fn local_playlist_read_songs(playlist: &str) -> Vec<String> {
    let paths = fs::read_dir(&format!("./music/{}", playlist)).unwrap();

    paths
        .map(|x| {
            let mut filename = x
                .as_ref()
                .unwrap()
                .file_name()
                .into_string()
                .expect("Cannot convert filename into valid UTF-8 string!");

            let extension = String::from(
                x.unwrap()
                    .path()
                    .extension()
                    .expect("Cannot convert filename into valid UTF-8 string!")
                    .to_str()
                    .expect("Cannot convert filename into valid UTF-8 string!"),
            );

            filename.remove_matches(&format!(".{}", extension));

            filename
        })
        .collect()
}
