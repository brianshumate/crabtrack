# CrabTrack roadmap

## Error handling

- Handle errors with configuration
- Handle errors when fetching data from Celestrak
- Handle errors when parsing TLE data
- Handle errors when storing/retrieving data with DuckDB

## Data fetching

- Get TLE data from Celestrak from the app
- Schedule data fetching from Celestrak

## Database support

- Use DuckDB to store TLE data matched to per-satellite information like:
  - Name
  - Launch date
  - Launch site
  - Country of origin
  - Operator
  - Type
  - Radio frequency downlink
  - Radio frequency uplink
  - Notes
