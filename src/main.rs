use std::{collections::HashMap, fs::File, io::BufReader, sync::Arc, time::Instant};

use faster_paths::{
    ch::{
        ch_path_finder::ChPathFinder,
        shortcut_replacer::{
            fast_shortcut_replacer::FastShortcutReplacer,
            slow_shortcut_replacer::SlowShortcutReplacer, ShortcutReplacer,
        },
        ContractedGraphInformation,
    },
    graphs::path::{PathFinding, ShortestPathRequest},
    hl::{hub_graph::HubGraph, hub_graph_path_finder::HubGraphPathFinder},
};
use osm_converter::sphere::{
    geometry::{linestring::Linestring, planet::Planet, point::Point},
    graph::graph::Fmi,
    spatial_partition::point_spatial_partition::PointSpatialPartition,
};
use serde::{Deserialize, Serialize};
use warp::{http::Response, Filter};

use clap::Parser;

/// Starts a routing service on localhost:3030/route
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of .fmi file
    #[arg(short, long)]
    gr_path: String,
    /// Path of .fmi file
    #[arg(short, long)]
    co_path: String,
    /// Path of .fmi file
    #[arg(short, long)]
    ch_path: String,
    /// Path of .fmi file
    #[arg(short, long)]
    hl_path: String,
}

#[derive(Deserialize, Serialize)]
struct RouteRequest {
    from: (f64, f64), // lon, lat
    to: (f64, f64),   // lon, lat
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);

    let coordinates_graph = Arc::new(Fmi::from_gr_co_file(
        args.gr_path.as_str(),
        args.co_path.as_str(),
    ));

    let mut point_grid = PointSpatialPartition::new_root(10);
    point_grid.add_points(&coordinates_graph.points);

    let mut point_id_map = HashMap::new();
    for (id, point) in coordinates_graph.points.iter().cloned().enumerate() {
        point_id_map.insert(point, id as usize);
    }
    let point_grid = Arc::new(point_grid);
    let point_id_map = Arc::new(point_id_map);

    // ch
    let reader = BufReader::new(File::open(args.ch_path).unwrap());
    let ch_information: ContractedGraphInformation = bincode::deserialize_from(reader).unwrap();
    let shortcut_replacer: Box<dyn ShortcutReplacer + Send + Sync> =
        Box::new(SlowShortcutReplacer::new(&ch_information.shortcuts));
    let ch_path_finder = ChPathFinder::new(ch_information.ch_graph, shortcut_replacer);

    // hl
    let fast_shortcut_replacer: Box<dyn ShortcutReplacer> =
        Box::new(FastShortcutReplacer::new(&ch_information.shortcuts));
    let reader = BufReader::new(File::open(args.hl_path).unwrap());
    let hl: HubGraph = bincode::deserialize_from(reader).unwrap();
    let _hl_path_finder = HubGraphPathFinder::new(hl, fast_shortcut_replacer);

    let path_finder: Arc<Box<dyn PathFinding>> = Arc::new(Box::new(ch_path_finder));

    println!("ready");

    let promote = {
        warp::post()
            .and(warp::path("route"))
            .and(warp::body::json())
            .map(move |route_request: RouteRequest| {
                let from_point = Point::from_coordinate(route_request.from.1, route_request.from.0);
                let nearest_from_proint = point_grid.get_nearest(&from_point).unwrap();
                let from = *point_id_map.get(&nearest_from_proint).unwrap() as u32;

                let to_point = Point::from_coordinate(route_request.to.1, route_request.to.0);
                let nearest_to_proint = point_grid.get_nearest(&to_point).unwrap();
                let to = *point_id_map.get(&nearest_to_proint).unwrap() as u32;

                let request = ShortestPathRequest::new(from, to).unwrap();
                let start = Instant::now();
                let pathx = path_finder.get_shortest_path(&request).unwrap();
                let time = start.elapsed();

                let ids = pathx.vertices;
                let path = coordinates_graph.convert_path(&ids);
                let linestring = Linestring::new(path);
                let mut planet = Planet::new();
                planet.linestrings.push(linestring);

                println!(
                    "route_request: {:>7} -> {:>7}, cost: {:>9}, took: {:>3}ms",
                    from,
                    to,
                    pathx.weight,
                    time.as_millis()
                );
                Response::builder().body(format!("{}", planet.to_geojson_str()))
            })
            .with(cors)
    };

    warp::serve(promote).run(([127, 0, 0, 1], 3030)).await;
}
