# rs1090
tar1090 port to Rust

## Description

rs1090 is a compatible port of the processing side of tar1090 to Rust. It reads decoded data from readsb and sends it to tar1090's web interface in a transparent way.

## Differences

rs1090 uses the type of aircraft and it's traveled distance to estimate the CO2 emissions, keeping totals per day, per aircraft type and per aircraft "flag".
