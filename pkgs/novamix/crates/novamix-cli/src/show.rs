//! What the commands print.

use std::fmt::Write;

use novamix::ipc::Output;
use novamix::ipc::Routes;
use novamix::ipc::Status;
use novamix::mix::Sink;

pub fn output(output: &Output) -> String {
    match &output.name {
        Some(name) => format!("{name} ({})\n", output.how),
        None => format!("no real sink yet ({})\n", output.how),
    }
}

pub fn status(status: &Status) -> String {
    let mut out = String::new();
    let base = if status.connected {
        "connected"
    } else {
        "not connected"
    };
    let _ = writeln!(out, "base station  {base}");
    let _ = writeln!(out, "mix           {}", status.mix);
    for sink in Sink::ALL {
        match status
            .sinks
            .iter()
            .find(|(found, _id, _name)| *found == sink)
        {
            Some((_sink, id, name)) => {
                let _ = writeln!(out, "sink {sink:<6} {name} node {id}");
            }
            None => {
                let _ = writeln!(out, "sink {sink:<6} missing");
            }
        }
    }
    out
}

pub fn routes(routes: &Routes) -> String {
    let mut out = String::from("rules\n");
    for rule in &routes.rules {
        let matches: Vec<String> = rule
            .matches
            .iter()
            .map(|props| {
                let props: Vec<String> = props.iter().map(|(k, v)| format!("{k}={v}")).collect();
                props.join(", ")
            })
            .collect();
        let _ = writeln!(out, "  {}  {}", rule.sink, matches.join(" | "));
    }
    out.push_str("streams\n");
    if routes.streams.is_empty() {
        out.push_str("  none playing\n");
    }
    for stream in &routes.streams {
        let target = stream.target.as_deref().unwrap_or("not linked");
        let note = match stream.rule {
            Some(_) => "",
            None => "  (no rule)",
        };
        let _ = writeln!(out, "  {}  {}  ->  {target}{note}", stream.id, stream.name);
    }
    out
}
