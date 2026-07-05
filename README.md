# MPK Transport

## Building

### Generate footpaths CSV (Optional)
1. Download the OSM PBF file. For Kraków, use the following:
  > curl -L -o data/krakow.osm.pbf https://download.geofabrik.de/europe/polan/malopolskie-latest.osm.pbf
2. Generate footpaths CSV:
  > cargo run --bin create_footpaths data/krakow.osm.pbf data/GTFS_KRK_T data/footpaths.csv

### Build and run the app
To run the MPK Transport simulation, use the following command:
  > cargo run data/footpaths.csv data/GTFS_KRK_T
