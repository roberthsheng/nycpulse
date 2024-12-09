//! # NYC Pulse Frontend
//!
//! This module implements the frontend web application for NYC Pulse - a real-time NYC subway
//! monitoring system. The application displays subway line statuses and train positions on an
//! interactive map using Mapbox GL JS.
//!
//! ## Key Components
//!
//! - `StatusPanel`: Displays real-time status information for each subway line
//! - `MapView`: Shows an interactive map with subway stations and real-time train positions
//! - `App`: The main application component that combines the status panel and map view
//!
//! ## Architecture
//!
//! The frontend is built using:
//! - Yew framework for building client-side web applications in Rust
//! - WebAssembly for running Rust code in the browser
//! - Mapbox GL JS for interactive mapping capabilities
//! - Tailwind CSS for styling
//!
//! The application communicates with a backend server to fetch real-time subway data.

use gloo_net::http::Request;
use js_sys::{Array, Object, Reflect};
use nyc_pulse_common::SubwayStatus;
use nyc_pulse_frontend::subway_data::{
    fetch_subway_stations, fetch_train_positions, get_line_style,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{console, Element, HtmlScriptElement};
use yew::prelude::*;

/// Mapbox access token for map initialization
const MAPBOX_TOKEN: &str = "pk.eyJ1Ijoicm9iZXJ0aHNoZW5nIiwiYSI6ImNtNDNwamplbzBmZ2QybG9ueW5tN2R3dDYifQ.MU1xzssfnYeuv5C4VFZeoQ";

/// Default center coordinates for NYC (longitude, latitude)
const NYC_CENTER: [f64; 2] = [-73.977664, 40.761484];

/// Bindings for Mapbox GL JS Popup functionality
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = mapboxgl)]
    type Popup;

    #[wasm_bindgen(constructor, js_namespace = mapboxgl)]
    fn new() -> Popup;

    #[wasm_bindgen(method, js_name = setLngLat)]
    fn set_lng_lat(this: &Popup, coords: &JsValue) -> Popup;

    #[wasm_bindgen(method, js_name = setHTML)]
    fn set_html(this: &Popup, html: &str) -> Popup;

    #[wasm_bindgen(method, js_name = addTo)]
    fn add_to(this: &Popup, map: &JsValue) -> Popup;

    #[wasm_bindgen(method)]
    fn remove(this: &Popup);
}

/// Properties for the StatusPanel component
#[derive(Properties, Clone, PartialEq)]
struct StatusPanelProps {
    /// Vector of subway line statuses
    statuses: Vec<SubwayStatus>,
    /// Currently selected subway line
    active_line: Option<String>,
    /// Callback for when a line is clicked
    on_line_click: Callback<String>,
}

/// Component that displays the status of all subway lines
#[function_component(StatusPanel)]
fn status_panel(props: &StatusPanelProps) -> Html {
    html! {
        <div class="h-full bg-zinc-900 shadow-lg overflow-auto">
            <div class="p-4">
                <h2 class="text-2xl font-bold mb-4 text-zinc-100">{"Line Status"}</h2>
                <div class="space-y-2">
                {
                    props.statuses.iter().map(|status| {
                        let is_active = props.active_line.as_ref().map_or(false, |l| l == &status.line);
                        let line = status.line.clone();
                        let onclick = {
                            let line = line.clone();
                            let on_line_click = props.on_line_click.clone();
                            Callback::from(move |_| {
                                on_line_click.emit(line.clone());
                            })
                        };

                        html! {
                            <div
                                {onclick}
                                class={classes!(
                                    "p-4",
                                    "rounded-lg",
                                    "transition-colors",
                                    "duration-200",
                                    if is_active { "bg-zinc-800" } else { "bg-zinc-800/50" },
                                    "hover:bg-zinc-800",
                                    "cursor-pointer"
                                )}
                            >
                                <div class="flex items-center justify-between">
                                    <div class="flex items-center space-x-3">
                                        <span class={classes!(
                                            get_line_style(&status.line),
                                            "w-10",
                                            "h-10",
                                            "rounded-full",
                                            "flex",
                                            "items-center",
                                            "justify-center",
                                            "text-white",
                                            "font-bold",
                                            "text-lg"
                                        )}>
                                            { &status.line }
                                        </span>
                                        <div class="flex flex-col">
                                            <span class={classes!(
                                                "font-medium",
                                                if status.delays { "text-red-400" } else { "text-green-400" }
                                            )}>
                                                { &status.status }
                                            </span>
                                            <span class="text-xs text-zinc-400">
                                                { "Updated "} { status.timestamp.format("%H:%M:%S").to_string() }
                                            </span>
                                        </div>
                                    </div>
                                    if status.delays {
                                        <span class="animate-pulse rounded-full h-3 w-3 bg-red-500 shadow-[0px_0px_4px_2px_rgba(239,68,68,0.7)]"/>
                                    }
                                </div>
                            </div>
                        }
                    }).collect::<Html>()
                }
                </div>
            </div>
        </div>
    }
}

/// Properties for the MapView component
#[derive(Properties, Clone, PartialEq)]
struct MapProps {
    /// Vector of subway line statuses
    statuses: Vec<SubwayStatus>,
    /// Currently selected subway line
    active_line: Option<String>,
}

/// Component that displays the interactive map with subway stations and trains
#[function_component(MapView)]
fn map_view(_props: &MapProps) -> Html {
    let map_ref = use_state(|| None::<JsValue>);
    let container_ref = use_node_ref();
    let stations_data = use_state(|| None::<String>);

    // Fetch stations data
    {
        let stations_data = stations_data.clone();
        use_effect_with_deps(
            move |_| {
                spawn_local(async move {
                    match fetch_subway_stations().await {
                        Ok(collection) => {
                            if let Ok(json) = serde_json::to_string(&collection) {
                                stations_data.set(Some(json));
                                console::log_1(&"Loaded subway stations data".into());
                            }
                        }
                        Err(e) => {
                            console::error_1(&format!("Error fetching stations: {:?}", e).into())
                        }
                    }
                });
                || {}
            },
            (),
        );
    }

    // Initialize map
    {
        let map_ref = map_ref.clone();
        let container_ref = container_ref.clone();
        let stations_data = stations_data.clone();

        use_effect_with_deps(
            move |data: &Option<String>| {
                if let Some(geojson_data) = data.clone() {
                    let window = web_sys::window().unwrap();
                    let document = window.document().unwrap();

                    let script = document
                        .create_element("script")
                        .unwrap()
                        .dyn_into::<HtmlScriptElement>()
                        .unwrap();
                    script.set_type("text/javascript");
                    script.set_src("https://api.mapbox.com/mapbox-gl-js/v2.15.0/mapbox-gl.js");

                    let link = document
                        .create_element("link")
                        .unwrap()
                        .dyn_into::<web_sys::HtmlLinkElement>()
                        .unwrap();
                    link.set_rel("stylesheet");
                    link.set_href("https://api.mapbox.com/mapbox-gl-js/v2.15.0/mapbox-gl.css");
                    document.head().unwrap().append_child(&link).unwrap();

                    // Store the init function in a variable to avoid FnOnce issues
                    let init_map = {
                        let container_ref = container_ref.clone();
                        let map_ref = map_ref.clone();
                        let geojson_data = geojson_data.clone();

                        move || {
                            if let Some(container) = container_ref.cast::<Element>() {
                                let options = Object::new();
                                Reflect::set(&options, &"container".into(), container.as_ref())
                                    .unwrap();

                                let center = Array::new();
                                center.push(&JsValue::from(NYC_CENTER[0]));
                                center.push(&JsValue::from(NYC_CENTER[1]));
                                Reflect::set(&options, &"center".into(), &center).unwrap();

                                Reflect::set(
                                    &options,
                                    &"style".into(),
                                    &"mapbox://styles/mapbox/dark-v11".into(),
                                )
                                .unwrap();
                                Reflect::set(&options, &"zoom".into(), &JsValue::from(12.0))
                                    .unwrap();
                                Reflect::set(
                                    &options,
                                    &"accessToken".into(),
                                    &JsValue::from_str(MAPBOX_TOKEN),
                                )
                                .unwrap();

                                if let Ok(mapboxgl) =
                                    js_sys::Reflect::get(&window, &"mapboxgl".into())
                                {
                                    if let Ok(map_constructor) =
                                        js_sys::Reflect::get(&mapboxgl, &"Map".into())
                                    {
                                        if let Ok(map_func) =
                                            map_constructor.dyn_into::<js_sys::Function>()
                                        {
                                            if let Ok(map) = js_sys::Reflect::construct(
                                                &map_func,
                                                &Array::of1(&options),
                                            ) {
                                                let map_clone = map.clone();

                                                // Create load handler
                                                let load_handler = {
                                                    let map = map_clone.clone();
                                                    let data = geojson_data.clone();

                                                    Closure::wrap(Box::new(move || {
                                                        let map = map.clone();
                                                        // Add source
                                                        let source = Object::new();
                                                        Reflect::set(
                                                            &source,
                                                            &"type".into(),
                                                            &"geojson".into(),
                                                        )
                                                        .unwrap();

                                                        if let Ok(geojson_obj) =
                                                            js_sys::JSON::parse(&data)
                                                        {
                                                            Reflect::set(
                                                                &source,
                                                                &"data".into(),
                                                                &geojson_obj,
                                                            )
                                                            .unwrap();

                                                            // Add station source
                                                            if let Ok(add_source) = Reflect::get(
                                                                &map,
                                                                &"addSource".into(),
                                                            ) {
                                                                let func = add_source
                                                                    .dyn_into::<js_sys::Function>()
                                                                    .unwrap();
                                                                let _ = func.call2(
                                                                    &map,
                                                                    &"stations".into(),
                                                                    &source,
                                                                );
                                                            }

                                                            // Outer glow layer
                                                            let glow_paint = Object::new();
                                                            Reflect::set(
                                                                &glow_paint,
                                                                &"circle-radius".into(),
                                                                &20.0.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &glow_paint,
                                                                &"circle-color".into(),
                                                                &"rgba(0, 255, 255, 0.1)".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &glow_paint,
                                                                &"circle-blur".into(),
                                                                &3.0.into(),
                                                            )
                                                            .unwrap();

                                                            let glow_layer = Object::new();
                                                            Reflect::set(
                                                                &glow_layer,
                                                                &"id".into(),
                                                                &"stations-glow".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &glow_layer,
                                                                &"type".into(),
                                                                &"circle".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &glow_layer,
                                                                &"source".into(),
                                                                &"stations".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &glow_layer,
                                                                &"paint".into(),
                                                                &glow_paint,
                                                            )
                                                            .unwrap();

                                                            // Inner glow layer
                                                            let inner_glow_paint = Object::new();
                                                            Reflect::set(
                                                                &inner_glow_paint,
                                                                &"circle-radius".into(),
                                                                &10.0.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &inner_glow_paint,
                                                                &"circle-color".into(),
                                                                &"rgba(0, 255, 255, 0.2)".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &inner_glow_paint,
                                                                &"circle-blur".into(),
                                                                &1.0.into(),
                                                            )
                                                            .unwrap();

                                                            let inner_glow_layer = Object::new();
                                                            Reflect::set(
                                                                &inner_glow_layer,
                                                                &"id".into(),
                                                                &"stations-inner-glow".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &inner_glow_layer,
                                                                &"type".into(),
                                                                &"circle".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &inner_glow_layer,
                                                                &"source".into(),
                                                                &"stations".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &inner_glow_layer,
                                                                &"paint".into(),
                                                                &inner_glow_paint,
                                                            )
                                                            .unwrap();

                                                            // Main station layer
                                                            let station_layer = Object::new();
                                                            Reflect::set(
                                                                &station_layer,
                                                                &"id".into(),
                                                                &"stations".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &station_layer,
                                                                &"type".into(),
                                                                &"symbol".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &station_layer,
                                                                &"source".into(),
                                                                &"stations".into(),
                                                            )
                                                            .unwrap();

                                                            // Add layers in order
                                                            if let Ok(add_layer_fn) = Reflect::get(
                                                                &map,
                                                                &"addLayer".into(),
                                                            ) {
                                                                let func = add_layer_fn
                                                                    .dyn_into::<js_sys::Function>()
                                                                    .unwrap();
                                                                let _ = func.call1(&map, &glow_layer).unwrap_or_else(|e| {
                                                                console::error_1(&format!("Failed to add glow layer: {:?}", e).into());
                                                                e
                                                            });
                                                                let _ = func.call1(&map, &inner_glow_layer).unwrap_or_else(|e| {
                                                                console::error_1(&format!("Failed to add inner glow layer: {:?}", e).into());
                                                                e
                                                            });
                                                                let _ = func.call1(&map, &station_layer).unwrap_or_else(|e| {
                                                                console::error_1(&format!("Failed to add station layer: {:?}", e).into());
                                                                e
                                                            });
                                                            }

                                                            // Train source
                                                            let train_source = Object::new();
                                                            Reflect::set(
                                                                &train_source,
                                                                &"type".into(),
                                                                &"geojson".into(),
                                                            )
                                                            .unwrap();
                                                            let empty_geojson = Object::new();
                                                            Reflect::set(
                                                                &empty_geojson,
                                                                &"type".into(),
                                                                &"FeatureCollection".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &empty_geojson,
                                                                &"features".into(),
                                                                &Array::new(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_source,
                                                                &"data".into(),
                                                                &empty_geojson,
                                                            )
                                                            .unwrap();

                                                            if let Ok(add_source) = Reflect::get(
                                                                &map,
                                                                &"addSource".into(),
                                                            ) {
                                                                let func = add_source
                                                                    .dyn_into::<js_sys::Function>()
                                                                    .unwrap();
                                                                let _ = func.call2(
                                                                    &map,
                                                                    &"trains".into(),
                                                                    &train_source,
                                                                );
                                                            }

                                                            // Create the color expression as a JS array
                                                            let color_expression = Array::new();
                                                            color_expression.push(&"get".into());
                                                            color_expression.push(&"color".into());

                                                            // Outer glow for trains
                                                            let train_glow_paint = Object::new();
                                                            Reflect::set(
                                                                &train_glow_paint,
                                                                &"circle-radius".into(),
                                                                &20.0.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_glow_paint,
                                                                &"circle-color".into(),
                                                                &color_expression,
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_glow_paint,
                                                                &"circle-opacity".into(),
                                                                &0.2.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_glow_paint,
                                                                &"circle-blur".into(),
                                                                &3.0.into(),
                                                            )
                                                            .unwrap();

                                                            let train_glow_layer = Object::new();
                                                            Reflect::set(
                                                                &train_glow_layer,
                                                                &"id".into(),
                                                                &"trains-glow".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_glow_layer,
                                                                &"type".into(),
                                                                &"circle".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_glow_layer,
                                                                &"source".into(),
                                                                &"trains".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_glow_layer,
                                                                &"paint".into(),
                                                                &train_glow_paint,
                                                            )
                                                            .unwrap();

                                                            // Solid background circle layer
                                                            let train_bg_paint = Object::new();
                                                            Reflect::set(
                                                                &train_bg_paint,
                                                                &"circle-radius".into(),
                                                                &12.0.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_bg_paint,
                                                                &"circle-color".into(),
                                                                &color_expression,
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_bg_paint,
                                                                &"circle-opacity".into(),
                                                                &1.0.into(),
                                                            )
                                                            .unwrap();

                                                            let train_bg_layer = Object::new();
                                                            Reflect::set(
                                                                &train_bg_layer,
                                                                &"id".into(),
                                                                &"trains-bg".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_bg_layer,
                                                                &"type".into(),
                                                                &"circle".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_bg_layer,
                                                                &"source".into(),
                                                                &"trains".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_bg_layer,
                                                                &"paint".into(),
                                                                &train_bg_paint,
                                                            )
                                                            .unwrap();

                                                            // Main train layer (text)
                                                            let train_layer = Object::new();
                                                            Reflect::set(
                                                                &train_layer,
                                                                &"id".into(),
                                                                &"trains".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_layer,
                                                                &"type".into(),
                                                                &"symbol".into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_layer,
                                                                &"source".into(),
                                                                &"trains".into(),
                                                            )
                                                            .unwrap();

                                                            // Layout properties for the symbol layer
                                                            let train_layout = Object::new();
                                                            Reflect::set(
                                                                &train_layout,
                                                                &"text-field".into(),
                                                                &Array::of2(
                                                                    &"get".into(),
                                                                    &"lines".into(),
                                                                ),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_layout,
                                                                &"text-size".into(),
                                                                &14.0.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_layout,
                                                                &"text-allow-overlap".into(),
                                                                &true.into(),
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_layout,
                                                                &"icon-allow-overlap".into(),
                                                                &true.into(),
                                                            )
                                                            .unwrap();

                                                            // Paint properties for the symbol layer
                                                            let train_paint = Object::new();
                                                            Reflect::set(
                                                                &train_paint,
                                                                &"text-color".into(),
                                                                &"#ffffff".into(),
                                                            )
                                                            .unwrap();

                                                            Reflect::set(
                                                                &train_layer,
                                                                &"layout".into(),
                                                                &train_layout,
                                                            )
                                                            .unwrap();
                                                            Reflect::set(
                                                                &train_layer,
                                                                &"paint".into(),
                                                                &train_paint,
                                                            )
                                                            .unwrap();

                                                            // Add layers in order
                                                            if let Ok(add_layer_fn) = Reflect::get(
                                                                &map,
                                                                &"addLayer".into(),
                                                            ) {
                                                                let func = add_layer_fn
                                                                    .dyn_into::<js_sys::Function>()
                                                                    .unwrap();
                                                                let _ = func.call1(&map, &train_glow_layer).unwrap_or_else(|e| {
                                                                console::error_1(&format!("Failed to add train glow layer: {:?}", e).into());
                                                                e
                                                            });
                                                                let _ = func.call1(&map, &train_bg_layer).unwrap_or_else(|e| {
                                                                console::error_1(&format!("Failed to add train background layer: {:?}", e).into());
                                                                e
                                                            });
                                                                let _ = func.call1(&map, &train_layer).unwrap_or_else(|e| {
                                                                console::error_1(&format!("Failed to add train layer: {:?}", e).into());
                                                                e
                                                            });
                                                            }

                                                            let map_clone = map.clone();
                                                            let update_trains = Closure::wrap(
                                                                Box::new(move || {
                                                                    console::log_1(&"Starting train position update...".into());
                                                                    let map_clone =
                                                                        map_clone.clone();
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                match fetch_train_positions().await {
                                                                    Ok(train_collection) => {
                                                                        console::log_1(&format!("Successfully fetched {} train positions", train_collection.features.len()).into());
                                                                        // Get the source
                                                                        if let Ok(get_source) = Reflect::get(&map_clone, &"getSource".into()) {
                                                                            if let Ok(source_func) = get_source.dyn_into::<js_sys::Function>() {
                                                                                match source_func.call1(&map_clone, &"trains".into()) {
                                                                                    Ok(source) => {
                                                                                        console::log_1(&"Successfully got train source".into());
                                                                                        if let Ok(set_data) = Reflect::get(&source, &"setData".into()) {
                                                                                            let func = set_data.dyn_into::<js_sys::Function>().unwrap();
                                                                                            match serde_wasm_bindgen::to_value(&train_collection) {
                                                                                                Ok(geojson) => {
                                                                                                    let _ = func.call1(&source, &geojson);
                                                                                                    console::log_1(&"Updated train source data".into());
                                                                                                }
                                                                                                Err(e) => console::error_1(&format!("Failed to serialize train data: {:?}", e).into()),
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    Err(e) => console::error_1(&format!("Failed to get train source: {:?}", e).into()),
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(e) => console::error_1(&format!("Failed to fetch train positions: {:?}", e).into()),
                                                                }
                                                            });
                                                                })
                                                                    as Box<dyn FnMut()>,
                                                            );

                                                            // Updates every 0.5s
                                                            let window = web_sys::window().unwrap();
                                                            window
                                                            .set_interval_with_callback_and_timeout_and_arguments_0(
                                                                update_trains.as_ref().unchecked_ref(),
                                                                500,
                                                            )
                                                            .unwrap();
                                                            update_trains.forget();
                                                        }
                                                    })
                                                        as Box<dyn FnMut()>)
                                                };

                                                if let Ok(on_fn) = Reflect::get(&map, &"on".into())
                                                {
                                                    let func = on_fn
                                                        .dyn_into::<js_sys::Function>()
                                                        .unwrap();
                                                    let _ = func.call2(
                                                        &map,
                                                        &"load".into(),
                                                        load_handler.as_ref().unchecked_ref(),
                                                    );
                                                }
                                                load_handler.forget();
                                                map_ref.set(Some(map));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    };

                    let onload = Closure::wrap(Box::new(move || {
                        init_map();
                    }) as Box<dyn FnMut()>);

                    script.set_onload(Some(onload.as_ref().unchecked_ref()));
                    document.head().unwrap().append_child(&script).unwrap();
                    onload.forget();
                }
                || {}
            },
            (*stations_data).clone(),
        );
    }

    html! {
        <div class="h-full w-full relative">
            <div
                ref={container_ref}
                id="map"
                class="absolute inset-0 m-4 rounded-2xl overflow-hidden bg-zinc-800"
            />

            <div class="absolute bottom-8 left-4 bg-zinc-900/90 p-4 rounded-2xl shadow-lg" style="z-index: 2;">
                <div class="space-y-2">
                    <div class="flex items-center gap-2">
                        <div class="h-2 w-2 rounded-full bg-green-400 shadow-[0px_0px_4px_2px_rgba(34,197,94,0.7)]" />
                        <div class="text-sm text-green-300/90 bg-green-800/30 px-2 py-1 rounded-lg">
                            {"Good Service"}
                        </div>
                    </div>
                    <div class="flex items-center gap-2">
                        <div class="h-2 w-2 rounded-full bg-red-400 shadow-[0px_0px_4px_2px_rgba(239,68,68,0.9)]" />
                        <div class="text-sm text-red-300/90 bg-red-800/30 px-2 py-1 rounded-lg">
                            {"Delays"}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Main application component that combines the status panel and map view
#[function_component(App)]
fn app() -> Html {
    let statuses = use_state(Vec::<SubwayStatus>::new);
    let active_line = use_state(|| None::<String>);

    {
        let statuses = statuses.clone();

        use_effect_with_deps(
            move |_: &[(); 0]| {
                let fetch_status = {
                    let statuses = statuses.clone();
                    Box::new(move || {
                        let statuses = statuses.clone();
                        async move {
                            console::log_1(&"Fetching subway status...".into());
                            match Request::get("http://localhost:3000/api/subway/status")
                                .send()
                                .await
                            {
                                Ok(response) => match response.json::<Vec<SubwayStatus>>().await {
                                    Ok(mut data) => {
                                        console::log_1(
                                            &format!("Received {} statuses", data.len()).into(),
                                        );
                                        data.sort_by(|a, b| a.line.cmp(&b.line));
                                        statuses.set(data);
                                    }
                                    Err(e) => console::error_1(
                                        &format!("Error parsing response: {:?}", e).into(),
                                    ),
                                },
                                Err(e) => console::error_1(
                                    &format!("Error fetching status: {:?}", e).into(),
                                ),
                            }
                        }
                    })
                };

                let fetch_future = (fetch_status)();
                wasm_bindgen_futures::spawn_local(fetch_future);

                let interval = {
                    let fetch_status = fetch_status.clone();
                    gloo_timers::callback::Interval::new(300, move || {
                        let fetch_future = (fetch_status)();
                        wasm_bindgen_futures::spawn_local(fetch_future);
                    })
                };

                move || drop(interval)
            },
            [],
        );
    }

    html! {
        <div class="h-screen bg-zinc-900 text-zinc-100">
            <div class="h-full flex gap-4 p-4">
                <div class="w-1/3 bg-zinc-800/50 rounded-2xl overflow-hidden backdrop-blur shadow-lg">
                    <StatusPanel
                        statuses={(*statuses).clone()}
                        active_line={(*active_line).clone()}
                        on_line_click={
                            let active_line = active_line.clone();
                            Callback::from(move |line| active_line.set(Some(line)))
                        }
                    />
                </div>
                <div class="w-2/3 bg-zinc-800/50 rounded-2xl overflow-hidden backdrop-blur shadow-lg">
                    <MapView
                        statuses={(*statuses).clone()}
                        active_line={(*active_line).clone()}
                    />
                </div>
            </div>
        </div>
    }
}

/// Entry point for the application
fn main() {
    yew::Renderer::<App>::new().render();
}
