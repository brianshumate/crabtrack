# CrabTrack 🦀

> NOTE: CrabTrack is a vibe engineered application.

## What?

CrabTrack is a simple TUI based satellite tracker written in Rust.

You can use it with Two Line Element (TLE) data files to track a range of satellites orbiting the Earth. You can also learn when they will be visible from your location or when they are within communication range.

## Why?

I've used several nice text based satellite trackers before, but not a recent one or one that was written in Rust, so I decided to vibe engineer one.

## Releases

Not yet, but you can build it fromm source like this:

1. Clone the repository:
   ```shell
   git clone https://github.com/brianshumate/crabtrack.git
   ```

1. Build the project:
   ```shell
   cd crabtrack
   cargo build --release
   ```

1. Copy `example.config.toml` to `config.toml` and edit it as needed for oyur location details and satellites you want to track.

1. Download the latest TLE file from [Celestrak](https://celestrak.org/NORAD/elements/) and save it as `satellites.tle` in the `data` directory. For example, if you wanted to grab the TLE data for cubesats, you could use the following URL:

   ```shell
   curl --output satellites.tle \
   https://celestrak.org/NORAD/elements/gp.php\?GROUP\=cubesat\&FORMAT\=tle
   ```

1. Run CrabTrack:
   ```shell
   cargo run --release
   ```
