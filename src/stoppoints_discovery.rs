use actix_web::{Json, Query, Result, State};
use crate::context::Context;
use gtfs_structures;
use std::borrow::Borrow;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorCondition {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Line {
    pub line_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AnnotatedStopPoint {
    pub stop_point_ref: String,
    pub stop_name: String,
    pub lines: Vec<Line>,
    pub location: Location,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StopPointsDelivery {
    pub version: String,
    pub response_time_stamp: String,
    pub status: bool,
    pub error_condition: Option<ErrorCondition>,
    pub annotated_stop_point: Vec<AnnotatedStopPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Siri {
    pub stop_points_delivery: StopPointsDelivery,
}

impl AnnotatedStopPoint {
    fn from(stop: &gtfs_structures::Stop, context: &Context) -> Self {
        let lines = context
            .lines_of_stops
            .get(&stop.id)
            .unwrap_or(&std::collections::HashSet::new())
            .iter()
            .map(|route_id| Line {
                line_ref: route_id.to_owned(),
            }).collect();

        Self {
            stop_point_ref: stop.id.to_owned(),
            stop_name: stop.name.to_owned(),
            lines,
            location: Location {
                longitude: stop.longitude,
                latitude: stop.latitude,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct Params {
    q: Option<String>,
    #[serde(rename = "BoundingBoxStructure.UpperLeft.Longitude")]
    upper_left_longitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.UpperLeft.Latitude")]
    upper_left_latitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.LowerRight.Latitude")]
    lower_right_longitude: Option<f64>,
    #[serde(rename = "BoundingBoxStructure.LowerRight.Latitude")]
    lower_right_latitude: Option<f64>,
}

fn name_matches(stop: &gtfs_structures::Stop, q: &str) -> bool {
    stop.name.to_lowercase().contains(q)
}

fn bounding_box_matches(
    stop: &gtfs_structures::Stop,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
) -> bool {
    stop.longitude >= min_lon
        && stop.longitude <= max_lon
        && stop.latitude >= min_lat
        && stop.latitude <= max_lat
}

pub fn stoppoints_discovery((state, query): (State<Context>, Query<Params>)) -> Result<Json<Siri>> {
    let stops = &state.gtfs.stops;

    let request = query.into_inner();
    let q = request.q.unwrap_or_default().to_lowercase();
    let min_lon = request.upper_left_longitude.unwrap_or(-180.);
    let max_lon = request.lower_right_longitude.unwrap_or(180.);
    let min_lat = request.lower_right_latitude.unwrap_or(-90.);
    let max_lat = request.upper_left_latitude.unwrap_or(90.);

    let filtered = stops
        .values()
        .filter(|s| name_matches(s, &q))
        .filter(|s| bounding_box_matches(s, min_lon, max_lon, min_lat, max_lat))
        .map(|stop| AnnotatedStopPoint::from(stop.borrow(), &state))
        .collect();

    Ok(Json(Siri {
        stop_points_delivery: StopPointsDelivery {
            version: "2.0".to_string(),
            response_time_stamp: chrono::Utc::now().to_rfc3339(),
            annotated_stop_point: filtered,
            error_condition: None,
            status: true,
        },
    }))
}