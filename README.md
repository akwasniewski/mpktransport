# MPK Transport
A visalisation of transport routing algorithms implemented for a class on algorithms in public transportation at TCS JU.

## Building
We allowed ourselves to include GTFS_KRK_T, a tram transit network for Kraków, Poland to simplify setup, all the example commands are set up for this input data.

### Generate footpaths CSV (Optional for raptor, required for CSA)
1. Download the OSM PBF file. For Kraków, use the following:
  > curl -L -o data/krakow.osm.pbf https://download.geofabrik.de/europe/polan/malopolskie-latest.osm.pbf
2. Generate footpaths CSV:
  > cargo run --bin create_footpaths data/krakow.osm.pbf data/GTFS_KRK_T data/footpaths.csv

### Build and run the app
To run the MPK Transport simulation, use the following command:
  > cargo rundata/GTFS_KRK_T
