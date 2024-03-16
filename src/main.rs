use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
    time::Instant,
};

use faster_paths::{
    ch::{
        ch_path_finder::ChPathFinder,
        shortcut_replacer::{slow_shortcut_replacer::SlowShortcutReplacer, ShortcutReplacer},
        ContractedGraphInformation,
    },
    graphs::{
        graph::Graph,
        graph_factory::GraphFactory,
        path::{PathFinding, ShortestPathRequest, ShortestPathValidation},
    },
    simple_algorithms::slow_dijkstra::SlowDijkstra,
};
use osm_converter::sphere::{
    geometry::{linestring::Linestring, planet::Planet},
    graph::graph::Fmi,
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

    let path_finding_graph = GraphFactory::from_gr_file(args.gr_path.as_str());

    let reader = BufReader::new(File::open(args.ch_path).unwrap());
    let ch_information: ContractedGraphInformation = bincode::deserialize_from(reader).unwrap();
    let shortcut_replacer: Box<dyn ShortcutReplacer + Send + Sync> =
        Box::new(SlowShortcutReplacer::new(&ch_information.shortcuts));
    let ch_path_finder = ChPathFinder::new(ch_information.ch_graph, shortcut_replacer);
    let ch_path_finder: Arc<Box<dyn PathFinding>> = Arc::new(Box::new(ch_path_finder));

    let slow_dijkstra = SlowDijkstra::new(path_finding_graph);
    let slow_dijkstra: Arc<Box<dyn PathFinding>> = Arc::new(Box::new(slow_dijkstra));

    let path_finding_graph = Arc::new(GraphFactory::from_gr_file(args.gr_path.as_str()));

    println!("ready");

    let promote = {
        warp::post()
            .and(warp::path("route"))
            .and(warp::body::json())
            .map(move |route_request: RouteRequest| {
                let from = coordinates_graph.nearest(route_request.from.0, route_request.from.1);
                let to = coordinates_graph.nearest(route_request.to.0, route_request.to.1);

                let request = ShortestPathRequest::new(from, to).unwrap();
                let start = Instant::now();
                let pathx = slow_dijkstra.get_shortest_path(&request).unwrap();
                let time = start.elapsed();

                let cost = Some(pathx.weight);
                let validation = ShortestPathValidation {
                    request: request.clone(),
                    weight: cost,
                };
                assert!(path_finding_graph
                    .validate_path(&validation, &Some(pathx.clone()))
                    .is_ok());

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
